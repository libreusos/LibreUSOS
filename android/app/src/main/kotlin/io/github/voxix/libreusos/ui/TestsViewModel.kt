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
import org.libreusos.core.TermTests
import org.libreusos.core.UsosException
import org.libreusos.core.installationTimeZone
import java.time.ZoneId

/** [onSessionExpired] is called at most once, when USOS answers `Unauthorized` to any call. */
class TestsViewModel(
    private val container: AppContainer,
    private val onSessionExpired: () -> Unit,
    private val nowMs: () -> Long = System::currentTimeMillis,
) : ViewModel() {
    private val _state = MutableStateFlow(TestsUiState())
    val state: StateFlow<TestsUiState> = _state.asStateFlow()

    // Touched on the main thread only.
    private var job: Job? = null
    private var expired = false

    /** Loads the tests unless a load is running or the live data is still fresh. */
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
                    _state.update { reduceTestsFailure(it, errorKindOf(t)) }
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
        // The cache is read every time: older terms are only ever served from it (see liveTermIds).
        var cached = emptyList<TermTests>()
        try {
            val (terms, tests) = withContext(Dispatchers.IO) {
                val terms = client.cachedTerms(today)
                terms to client.cachedCourseTests(terms.terms.map { it.id })
            }
            cached = tests
            _state.update {
                reduceTestsCache(it, buildTestSections(terms.terms, tests), listOf(terms.source) + tests.map { t -> t.source })
            }
        } catch (e: CancellationException) {
            throw e
        } catch (_: UsosException.NotCached) {
            // Nothing stored yet.
        } catch (t: Throwable) {
            logFailure(t)
            if (_state.value.sections == null) _state.update { reduceTestsCacheFailure(it, errorKindOf(t)) }
        }
        val (terms, liveIds, live) = withContext(Dispatchers.IO) {
            val terms = container.terms.terms(today, force = forceTerms) { client.terms(today) }
            val liveIds = liveTermIds(terms.terms, cached.map { it.termId }.toSet())
            Triple(terms, liveIds, client.courseTests(liveIds))
        }
        _state.update {
            reduceTestsLive(
                it,
                buildTestSections(terms.terms, mergeTermTests(cached, live.terms, liveIds)),
                listOf(terms.source) + live.terms.map { t -> t.source },
                live.terms.sumOf { t -> t.skipped.size },
                live.failedTerms.size,
                nowMs(),
            )
        }
    }

    /** Opens or closes the row under [key]. */
    fun toggle(key: String) {
        _state.update { reduceToggle(it, key) }
    }
}
