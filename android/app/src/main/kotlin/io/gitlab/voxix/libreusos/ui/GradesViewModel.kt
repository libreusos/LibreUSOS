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
import org.libreusos.core.CourseGrades
import org.libreusos.core.UsosException
import org.libreusos.core.installationTimeZone
import java.time.ZoneId

/** [onSessionExpired] is called at most once, when USOS answers `Unauthorized` to any call. */
class GradesViewModel(
    private val container: AppContainer,
    private val onSessionExpired: () -> Unit,
    private val online: () -> Boolean = { isOnline(container.appContext) },
    private val nowMs: () -> Long = System::currentTimeMillis,
) : ViewModel() {
    private val _state = MutableStateFlow(GradesUiState())
    val state: StateFlow<GradesUiState> = _state.asStateFlow()

    /** When the automatic retry runs, for "spróbujemy ponownie za 15 min". */
    val syncRecord = container.syncStore.record

    /** The university's USOS address, for the technical details of an error. */
    val baseUrl: String get() = container.settings.baseUrl

    private val _openCourse = MutableStateFlow<OpenCourse?>(null)

    /** The course the detail screen shows; kept here so it survives a rotation. */
    val openCourse: StateFlow<OpenCourse?> = _openCourse.asStateFlow()

    fun showCourse(termId: String, course: CourseGrades) {
        _openCourse.value = OpenCourse(termId, course)
    }

    // Touched on the main thread only.
    private var job: Job? = null
    private var expired = false

    /** Loads the grades unless a load is running or the live data is still fresh. */
    fun ensureLoaded() {
        if (job != null || !isStale(_state.value.liveAtMs, nowMs())) return
        start(userInitiated = false)
    }

    /** Pull-to-refresh. When a load is already running, it waits for that one. */
    fun refresh() {
        if (job != null) {
            _state.update { it.copy(refreshing = true) }
        } else {
            start(userInitiated = true)
        }
    }

    private fun start(userInitiated: Boolean) {
        if (expired) return
        _state.update { it.copy(loading = true, refreshing = userInitiated) }
        val started = viewModelScope.launch(start = CoroutineStart.LAZY) {
            try {
                load(forceTerms = userInitiated)
            } catch (e: CancellationException) {
                throw e
            } catch (t: Throwable) {
                logFailure(t)
                if (t is UsosException.Unauthorized) {
                    if (!expired) {
                        expired = true
                        onSessionExpired()
                    }
                } else {
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

    private suspend fun load(forceTerms: Boolean) {
        // Every FFI call runs on Dispatchers.IO.
        val client = withContext(Dispatchers.IO) { container.client(container.settings.baseUrl) }
            ?: throw AppException(ErrorKind.KEY_MISSING)
        val zone = withContext(Dispatchers.IO) { installationTimeZone() }
        val todayDate = todayIn(zone)
        val today = todayDate.toString()
        _state.update { it.copy(zone = ZoneId.of(zone), today = todayDate) }
        if (_state.value.sections == null) {
            try {
                val (terms, grades) = withContext(Dispatchers.IO) {
                    val terms = client.cachedTerms(today)
                    terms to client.cachedGrades(terms.terms.map { it.id })
                }
                _state.update {
                    reduceCache(
                        it,
                        buildSections(terms.terms, grades),
                        TermList(terms.terms, terms.current?.id),
                        listOf(terms.source) + grades.map { g -> g.source },
                    )
                }
            } catch (e: CancellationException) {
                throw e
            } catch (_: UsosException.NotCached) {
                // Nothing stored yet.
            } catch (t: Throwable) {
                // The live call below may still work.
                logFailure(t)
            }
        }
        val (terms, grades) = withContext(Dispatchers.IO) {
            val terms = container.terms.terms(today, force = forceTerms) { client.terms(today) }
            terms to client.grades(terms.terms.map { it.id })
        }
        // Not live: the core fell back to its copy because the network failed.
        val notLive = failureOf(UsosException.Network("fell back to cache"), online(), nowMs())
        _state.update {
            reduceLive(
                it,
                buildSections(terms.terms, grades),
                TermList(terms.terms, terms.current?.id),
                listOf(terms.source) + grades.map { g -> g.source },
                nowMs(),
                notLive,
            )
        }
    }
}
