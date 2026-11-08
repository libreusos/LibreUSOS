package io.gitlab.voxix.libreusos.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.gitlab.voxix.libreusos.AppContainer
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * The signed-in person for the account card. `whoami()` may answer from the cache; a failure keeps the
 * card without a name (the university line stays), because a session that expired is reported by the
 * screens that fetch data.
 */
class AccountViewModel(private val container: AppContainer) : ViewModel() {
    private val baseUrl = container.settings.baseUrl

    private val _state = MutableStateFlow(
        AccountUiState(university = universityNameOf(baseUrl), host = hostOf(baseUrl)),
    )
    val state: StateFlow<AccountUiState> = _state.asStateFlow()

    init {
        viewModelScope.launch {
            val user = try {
                withContext(Dispatchers.IO) { container.client(baseUrl)?.whoami()?.user }
            } catch (e: CancellationException) {
                throw e
            } catch (t: Throwable) {
                logFailure(t)
                null
            } ?: return@launch
            _state.update {
                it.copy(
                    name = "${user.firstName} ${user.lastName}".trim().ifEmpty { null },
                    initials = initialsOf(user.firstName, user.lastName),
                )
            }
        }
    }
}
