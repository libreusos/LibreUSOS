package io.gitlab.voxix.libreusos.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.gitlab.voxix.libreusos.AppContainer
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
import org.libreusos.core.Timetable
import org.libreusos.core.UsosException
import org.libreusos.core.installationTimeZone
import java.time.LocalDate
import java.time.ZoneId

/** An activity with a key that is unique within its day and stable across refreshes. */
data class KeyedActivity(val key: String, val activity: Activity)

/** What the screen knows about one week (Monday to Sunday). */
data class WeekState(
    val timetable: Timetable? = null,
    /** [timetable] grouped by start day, sorted as the core sorted it. Built once per update. */
    val byDay: Map<LocalDate, List<KeyedActivity>> = emptyMap(),
    /** A cache read or a live fetch is running. */
    val loading: Boolean = false,
    /** A user pull-to-refresh is waiting for the running fetch. */
    val refreshing: Boolean = false,
    /** The last attempt failed with this (shown when there is no data to fall back on). */
    val error: ErrorKind? = null,
    /** When the data was last fetched live (epoch ms); null when it is cached, or a refresh failed. */
    val liveAtMs: Long? = null,
    /** When the data on screen was fetched (epoch seconds), live or cached. */
    val dataAtSec: Long? = null,
    /** A refresh failed and [timetable] is older data. */
    val refreshFailed: Boolean = false,
)

data class TimetableUiState(
    /** Null until the installation's zone is known. */
    val zone: ZoneId? = null,
    val today: LocalDate? = null,
    val weeks: Map<LocalDate, WeekState> = emptyMap(),
)

/** [onSessionExpired] is called at most once, when USOS answers `Unauthorized` to any call. */
class TimetableViewModel(
    private val container: AppContainer,
    private val onSessionExpired: () -> Unit,
    private val nowMs: () -> Long = System::currentTimeMillis,
) : ViewModel() {
    private val _state = MutableStateFlow(TimetableUiState())
    val state: StateFlow<TimetableUiState> = _state.asStateFlow()

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
            _state.update { it.copy(zone = ZoneId.of(zone), today = todayIn(zone)) }
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
        update(weekStart) { it.copy(loading = true, refreshing = userInitiated, error = null) }
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
                    update(weekStart) {
                        it.copy(error = errorKindOf(t), refreshFailed = it.timetable != null, liveAtMs = null)
                    }
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
        var cacheError: ErrorKind? = null
        if (_state.value.weeks[weekStart]?.timetable == null) {
            try {
                val cached = withContext(Dispatchers.IO) { client.cachedTimetable(from, to) }
                update(weekStart) { if (it.timetable == null) it.withTimetable(cached, null) else it }
            } catch (e: CancellationException) {
                throw e
            } catch (_: UsosException.NotCached) {
                // Nothing stored for this week yet.
            } catch (t: Throwable) {
                logFailure(t)
                cacheError = errorKindOf(t)
            }
        }
        val live = withContext(Dispatchers.IO) { client.timetable(from, to) }
        val liveAt = if (live.source == DataSource.Live) nowMs() else null
        update(weekStart) { it.withTimetable(live, liveAt).copy(error = cacheError, refreshFailed = false) }
    }

    private fun WeekState.withTimetable(timetable: Timetable, liveAtMs: Long?): WeekState = copy(
        timetable = timetable,
        byDay = groupByDay(timetable.activities),
        liveAtMs = liveAtMs,
        dataAtSec = when (val source = timetable.source) {
            DataSource.Live -> (liveAtMs ?: nowMs()) / 1000
            is DataSource.Cached -> source.fetchedAt
        },
    )

    private fun update(weekStart: LocalDate, change: (WeekState) -> WeekState) {
        _state.update { s -> s.copy(weeks = s.weeks + (weekStart to change(s.weeks[weekStart] ?: WeekState()))) }
    }
}

/** Groups by the start day; the key repeats nothing within a day, even for identical entries. */
private fun groupByDay(activities: List<Activity>): Map<LocalDate, List<KeyedActivity>> {
    val seen = HashMap<String, Int>()
    val keyed = ArrayList<Pair<LocalDate, KeyedActivity>>(activities.size)
    for (a in activities) {
        val day = try {
            LocalDate.parse(a.date)
        } catch (_: java.time.format.DateTimeParseException) {
            continue
        }
        val base = listOf(a.start, a.end, a.courseId, a.classtypeId, a.groupNumber, a.roomId).joinToString("|")
        val n = seen.merge(base, 1, Int::plus)
        keyed += day to KeyedActivity("$base#$n", a)
    }
    return keyed.groupBy({ it.first }, { it.second })
}
