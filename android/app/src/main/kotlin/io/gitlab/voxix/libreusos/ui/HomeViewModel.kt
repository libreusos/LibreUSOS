package io.gitlab.voxix.libreusos.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.gitlab.voxix.libreusos.AppContainer
import io.gitlab.voxix.libreusos.data.isOnline
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
import org.libreusos.core.DataSource
import org.libreusos.core.UsosException
import org.libreusos.core.installationTimeZone
import java.time.ZoneId

/** Days after today that Today loads too, so a day without classes can name the next ones. */
private const val LOOKAHEAD_DAYS = 7L

/** [onSessionExpired] is called when USOS answers `Unauthorized`; the cached plan stays on screen. */
class HomeViewModel(
    private val container: AppContainer,
    private val onSessionExpired: () -> Unit,
    private val online: () -> Boolean = { isOnline(container.appContext) },
    private val nowMs: () -> Long = System::currentTimeMillis,
) : ViewModel() {
    private val _state = MutableStateFlow(HomeUiState())
    val state: StateFlow<HomeUiState> = _state.asStateFlow()

    /** When the automatic retry runs, for "spróbujemy ponownie za 15 min". */
    val syncRecord = container.syncStore.record

    /** The university's USOS address, for the technical details of an error. */
    val baseUrl: String get() = container.settings.baseUrl

    // Touched on the main thread only.
    private var job: Job? = null
    private var expired = false

    /** First visit and every return to the screen: loads when nothing is shown, the day changed or the data is old. */
    fun ensureLoaded(todayNow: java.time.LocalDate) {
        if (needsRefresh(_state.value, todayNow, nowMs())) start(userInitiated = false)
    }

    /** Pull-to-refresh and "Spróbuj ponownie". When a load is already running, it waits for that one. */
    fun refresh() {
        if (job != null) _state.update { it.copy(refreshing = true) } else start(userInitiated = true)
    }

    private fun start(userInitiated: Boolean) {
        if (expired || job != null) return
        _state.update { it.copy(loading = true, refreshing = userInitiated) }
        val started = viewModelScope.launch(start = CoroutineStart.LAZY) {
            try {
                load()
            } catch (e: CancellationException) {
                throw e
            } catch (t: Throwable) {
                logFailure(t)
                if (t is UsosException.Unauthorized) {
                    expired = true
                    onSessionExpired()
                } else {
                    container.syncStore.failed()
                    _state.update { reduceFailure(it, failureOf(t, online(), nowMs())) }
                }
            } finally {
                job = null
                _state.update { it.copy(loading = false, refreshing = false) }
            }
        }
        job = started
        started.start()
    }

    private suspend fun load() {
        // Every FFI call runs on Dispatchers.IO.
        val client = withContext(Dispatchers.IO) { container.client(container.settings.baseUrl) }
            ?: throw AppException(ErrorKind.KEY_MISSING)
        val zone = withContext(Dispatchers.IO) { installationTimeZone() }
        val today = todayIn(zone)
        val from = today.toString()
        val to = today.plusDays(LOOKAHEAD_DAYS).toString()
        _state.update { it.copy(zone = ZoneId.of(zone), today = today) }
        if (_state.value.activities == null) {
            try {
                val cached = withContext(Dispatchers.IO) { client.cachedTimetable(from, to) }
                (cached.source as? DataSource.Cached)?.takeIf { cached.daysCached > 0u }?.let { src ->
                    _state.update { reduceCache(it, cached.activities, src.fetchedAt) }
                }
            } catch (e: CancellationException) {
                throw e
            } catch (_: UsosException.NotCached) {
                // Nothing stored yet.
            } catch (t: Throwable) {
                // The live call below may still work; a broken cache read alone is not shown.
                logFailure(t)
            }
        }
        val result = withContext(Dispatchers.IO) { client.timetable(from, to) }
        when (val source = result.source) {
            DataSource.Live -> {
                container.syncStore.succeeded(nowMs())
                _state.update { reduceLive(it, result.activities, nowMs()) }
            }
            // The core fell back to its copy because the network failed.
            is DataSource.Cached -> {
                container.syncStore.failed()
                val failure = failureOf(UsosException.Network("fell back to cache"), online(), nowMs())
                val hasData = result.daysCached > 0u
                _state.update {
                    reduceFailure(it, failure, result.activities.takeIf { hasData }, source.fetchedAt.takeIf { hasData })
                }
            }
        }
    }
}
