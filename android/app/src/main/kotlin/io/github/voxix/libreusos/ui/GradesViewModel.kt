package io.github.voxix.libreusos.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.github.voxix.libreusos.AppContainer
import io.github.voxix.libreusos.data.isStale
import io.github.voxix.libreusos.data.todayIn
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
import org.libreusos.core.UsosException
import org.libreusos.core.installationTimeZone
import java.time.ZoneId

/** [onSessionExpired] is called at most once, when USOS answers `Unauthorized` to any call. */
class GradesViewModel(
    private val container: AppContainer,
    private val onSessionExpired: () -> Unit,
    private val nowMs: () -> Long = System::currentTimeMillis,
) : ViewModel() {
    private val _state = MutableStateFlow(GradesUiState())
    val state: StateFlow<GradesUiState> = _state.asStateFlow()

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
        _state.update { it.copy(loading = true, refreshing = userInitiated, error = null) }
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
                    _state.update { reduceFailure(it, errorKindOf(t)) }
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
        val today = todayIn(zone).toString()
        _state.update { it.copy(zone = ZoneId.of(zone)) }
        if (_state.value.sections == null) {
            try {
                val (terms, grades) = withContext(Dispatchers.IO) {
                    val terms = client.cachedTerms(today)
                    terms to client.cachedGrades(terms.terms.map { it.id })
                }
                _state.update { reduceCache(it, buildSections(terms.terms, grades), listOf(terms.source) + grades.map { g -> g.source }) }
            } catch (e: CancellationException) {
                throw e
            } catch (_: UsosException.NotCached) {
                // Nothing stored yet.
            } catch (t: Throwable) {
                logFailure(t)
                _state.update { reduceCacheFailure(it, errorKindOf(t)) }
            }
        }
        val (terms, grades) = withContext(Dispatchers.IO) {
            val terms = container.terms.terms(today, force = forceTerms) { client.terms(today) }
            terms to client.grades(terms.terms.map { it.id })
        }
        _state.update {
            reduceLive(it, buildSections(terms.terms, grades), listOf(terms.source) + grades.map { g -> g.source }, nowMs())
        }
    }
}
