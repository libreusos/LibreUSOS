package io.github.voxix.libreusos.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.github.voxix.libreusos.AppContainer
import io.github.voxix.libreusos.data.todayIn
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.libreusos.core.Identity
import org.libreusos.core.Timetable
import org.libreusos.core.UsosException
import org.libreusos.core.installationTimeZone

data class HomeUiState(
    val loading: Boolean = true,
    val identity: Identity? = null,
    val timetable: Timetable? = null,
    val errors: List<ErrorKind> = emptyList(),
    val zone: java.time.ZoneId? = null,
)

/** [onSessionExpired] is called when USOS answers `Unauthorized` to any call. */
class HomeViewModel(
    private val container: AppContainer,
    private val onSessionExpired: () -> Unit,
) : ViewModel() {
    private val _state = MutableStateFlow(HomeUiState())
    val state: StateFlow<HomeUiState> = _state.asStateFlow()

    private var job: Job? = null

    init {
        refresh()
    }

    /** Single-flight: ignored while a refresh is running. */
    fun refresh() {
        if (job?.isActive == true) return
        _state.update { it.copy(loading = true, errors = emptyList()) }
        job = viewModelScope.launch {
            val errors = mutableListOf<ErrorKind>()
            var expired = false
            // Every FFI call runs on Dispatchers.IO.
            suspend fun <T> io(block: suspend () -> T): T? = try {
                withContext(Dispatchers.IO) { block() }
            } catch (e: CancellationException) {
                throw e
            } catch (t: Throwable) {
                logFailure(t)
                if (t is UsosException.Unauthorized) expired = true
                errors += errorKindOf(t)
                null
            }

            val client = io { container.client(container.settings.baseUrl) ?: throw AppException(ErrorKind.KEY_MISSING) }
            val zone = io { installationTimeZone() } ?: "Europe/Warsaw"
            var identity: Identity? = null
            var timetable: Timetable? = null
            if (client != null) {
                identity = io { client.whoami() }
                timetable = io {
                    val today = todayIn(zone).toString()
                    client.timetable(today, today)
                }
            }
            if (expired) {
                onSessionExpired()
                return@launch
            }
            _state.update { HomeUiState(false, identity ?: it.identity, timetable ?: it.timetable, errors, java.time.ZoneId.of(zone)) }
        }
    }
}
