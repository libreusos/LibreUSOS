package io.gitlab.voxix.libreusos.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.gitlab.voxix.libreusos.AppContainer
import io.gitlab.voxix.libreusos.data.isOnline
import io.gitlab.voxix.libreusos.data.isStale
import io.gitlab.voxix.libreusos.data.todayIn
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.libreusos.core.Activity
import org.libreusos.core.DataSource
import org.libreusos.core.Term
import org.libreusos.core.UsosException
import org.libreusos.core.installationTimeZone
import java.time.LocalDate
import java.time.ZoneId

/** What the screen knows about one week (Monday to Sunday). */
data class WeekState(
    /** Null until the cache or USOS answered. */
    val activities: List<Activity>? = null,
    /** A cache read or a live fetch is running. */
    val loading: Boolean = false,
    /** A user pull-to-refresh is waiting for the running fetch. */
    val refreshing: Boolean = false,
    /** Why the last refresh failed; [activities] is then the older copy, if any. */
    val failure: HomeFailure? = null,
    /** When the data was last fetched live (epoch ms); null when it is cached, or a refresh failed. */
    val liveAtMs: Long? = null,
    /** When the data on screen was fetched (epoch seconds), live or cached. */
    val dataAtSec: Long? = null,
)

/** The week as the state helpers of Today see it, so Plan shows the same banners, status and full-screen states. */
fun WeekState?.asHomeState(zone: ZoneId, today: LocalDate): HomeUiState = HomeUiState(
    zone = zone,
    today = today,
    activities = this?.activities,
    dataAtSec = this?.dataAtSec,
    liveAtMs = this?.liveAtMs,
    loading = this?.loading ?: true,
    refreshing = this?.refreshing ?: false,
    failure = this?.failure,
)

data class TimetableUiState(
    /** Null until the installation's zone is known. */
    val zone: ZoneId? = null,
    val today: LocalDate? = null,
    val weeks: Map<LocalDate, WeekState> = emptyMap(),
    /** The term the core calls current, for the week number in the header; null when unknown. */
    val term: Term? = null,
)

/** How far around a class the cached timetable is searched for the other meetings of its group. */
private const val MEETINGS_RADIUS_DAYS = 182L

/** [onSessionExpired] is called at most once, when USOS answers `Unauthorized` to any call. */
class TimetableViewModel(
    private val container: AppContainer,
    private val onSessionExpired: () -> Unit,
    private val online: () -> Boolean = { isOnline(container.appContext) },
    private val nowMs: () -> Long = System::currentTimeMillis,
) : ViewModel() {
    private val _state = MutableStateFlow(TimetableUiState())
    val state: StateFlow<TimetableUiState> = _state.asStateFlow()

    /** When the automatic retry runs, for "spróbujemy ponownie za 15 min". */
    val syncRecord = container.syncStore.record

    /** The university's USOS address, for the technical details of an error. */
    val baseUrl: String get() = container.settings.baseUrl

    private val _openClass = MutableStateFlow<Activity?>(null)

    /** The class the detail screen shows; kept here so it survives a rotation. */
    val openClass: StateFlow<Activity?> = _openClass.asStateFlow()

    fun showClass(activity: Activity) {
        _openClass.value = activity
    }

    // Touched on the main thread only.
    private val jobs = mutableMapOf<LocalDate, Job>()
    private var zoneId: String? = null
    private var expired = false

    init {
        updateToday()
    }

    /** Recomputes today in the installation's zone (the day may have changed while the app slept). */
    fun updateToday() {
        viewModelScope.launch {
            val zone = zoneId ?: withContext(Dispatchers.IO) { installationTimeZone() }.also { zoneId = it }
            val today = todayIn(zone)
            _state.update { it.copy(zone = ZoneId.of(zone), today = today) }
            loadTerm(today.toString())
        }
    }

    private var termLoadedFor: String? = null

    /**
     * Reads the current term once per day: the stored terms first, USOS only when none are stored. It
     * only decorates the header, so a failure is logged and the header simply has no week number.
     */
    private suspend fun loadTerm(today: String) {
        if (termLoadedFor == today) return
        termLoadedFor = today
        try {
            val client = withContext(Dispatchers.IO) { container.client(container.settings.baseUrl) } ?: return
            val term = withContext(Dispatchers.IO) {
                try {
                    client.cachedTerms(today).current
                } catch (_: UsosException.NotCached) {
                    container.terms.terms(today) { client.terms(today) }.current
                }
            }
            _state.update { it.copy(term = term) }
        } catch (e: CancellationException) {
            throw e
        } catch (t: Throwable) {
            termLoadedFor = null
            logFailure(t)
        }
    }

    /** Loads each week that is not running and has no fresh live data: the cache first, then USOS. */
    fun ensureWeeks(weekStarts: Set<LocalDate>) {
        for (week in weekStarts) {
            if (week in jobs) continue
            if (!isStale(_state.value.weeks[week]?.liveAtMs, nowMs())) continue
            start(week, userInitiated = false)
        }
    }

    /** Pull-to-refresh. When a fetch of [weekStart] is already running, it waits for that one. */
    fun refresh(weekStart: LocalDate) {
        if (weekStart in jobs) {
            update(weekStart) { it.copy(refreshing = true) }
        } else {
            start(weekStart, userInitiated = true)
        }
    }

    private fun start(weekStart: LocalDate, userInitiated: Boolean) {
        if (expired) return
        update(weekStart) { it.copy(loading = true, refreshing = userInitiated) }
        val job = viewModelScope.launch(start = CoroutineStart.LAZY) {
            try {
                fetch(weekStart)
            } catch (e: CancellationException) {
                throw e
            } catch (t: Throwable) {
                logFailure(t)
                if (t is UsosException.Unauthorized) {
                    if (!expired) {
                        expired = true
                        jobs.filterKeys { it != weekStart }.values.forEach { it.cancel() }
                        onSessionExpired()
                    }
                } else {
                    update(weekStart) { it.copy(failure = failureOf(t, online(), nowMs()), liveAtMs = null) }
                }
            } finally {
                jobs.remove(weekStart)
                update(weekStart) { it.copy(loading = false, refreshing = false) }
            }
        }
        jobs[weekStart] = job
        job.start()
    }

    private suspend fun fetch(weekStart: LocalDate) {
        val from = weekStart.toString()
        val to = weekStart.plusDays(6).toString()
        // Every FFI call runs on Dispatchers.IO.
        val client = withContext(Dispatchers.IO) { container.client(container.settings.baseUrl) }
            ?: throw AppException(ErrorKind.KEY_MISSING)
        if (_state.value.weeks[weekStart]?.activities == null) {
            try {
                val cached = withContext(Dispatchers.IO) { client.cachedTimetable(from, to) }
                (cached.source as? DataSource.Cached)?.takeIf { cached.daysCached > 0u }?.let { src ->
                    update(weekStart) { if (it.activities == null) it.copy(activities = cached.activities, dataAtSec = src.fetchedAt) else it }
                }
            } catch (e: CancellationException) {
                throw e
            } catch (_: UsosException.NotCached) {
                // Nothing stored for this week yet.
            } catch (t: Throwable) {
                // The live fetch below still runs; a broken cache read alone is not shown.
                logFailure(t)
            }
        }
        val live = withContext(Dispatchers.IO) { client.timetable(from, to) }
        when (val source = live.source) {
            DataSource.Live -> update(weekStart) {
                it.copy(activities = live.activities, dataAtSec = nowMs() / 1000, liveAtMs = nowMs(), failure = null)
            }
            // The core fell back to its copy because the network failed.
            is DataSource.Cached -> update(weekStart) {
                val failure = failureOf(UsosException.Network("fell back to cache"), online(), nowMs())
                if (live.daysCached > 0u) {
                    it.copy(activities = live.activities, dataAtSec = source.fetchedAt, liveAtMs = null, failure = failure)
                } else {
                    it.copy(liveAtMs = null, failure = failure)
                }
            }
        }
    }

    /** The meetings of [current]'s group in the cached timetable around it. Never fails: no cache means one meeting. */
    suspend fun meetingsOf(current: Activity): GroupMeetings {
        val day = runCatching { LocalDate.parse(current.date) }.getOrNull()
        val cached = if (day == null) {
            emptyList()
        } else {
            try {
                val client = withContext(Dispatchers.IO) { container.client(container.settings.baseUrl) }
                client?.let {
                    withContext(Dispatchers.IO) {
                        it.cachedTimetable(day.minusDays(MEETINGS_RADIUS_DAYS).toString(), day.plusDays(MEETINGS_RADIUS_DAYS).toString())
                    }.activities
                }.orEmpty()
            } catch (e: CancellationException) {
                throw e
            } catch (t: Throwable) {
                logFailure(t)
                emptyList()
            }
        }
        return groupMeetings(current, cached)
    }

    private fun update(weekStart: LocalDate, change: (WeekState) -> WeekState) {
        _state.update { s -> s.copy(weeks = s.weeks + (weekStart to change(s.weeks[weekStart] ?: WeekState()))) }
    }
}
