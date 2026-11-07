package io.gitlab.voxix.libreusos.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.gitlab.voxix.libreusos.AppContainer
import androidx.glance.appwidget.updateAll
import io.gitlab.voxix.libreusos.data.SecretCodec
import io.gitlab.voxix.libreusos.sync.SyncScheduler
import io.gitlab.voxix.libreusos.widget.TimetableWidget
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

const val OAUTH_CALLBACK = "libreusos://oauth-callback"

enum class AuthState { Loading, LoggedOut, LoggedIn }

data class LoginUiState(
    val baseUrl: String,
    val consumerKey: String = "",
    val consumerSecret: String = "",
    /** True when this build has no embedded key for [baseUrl], so key and secret must be typed. */
    val needsKey: Boolean = false,
    val busy: Boolean = false,
    val error: ErrorKind? = null,
    val warnings: List<String> = emptyList(),
)

/** Activity-scoped: owns the login flow, which outlives the screens. Never logs tokens. */
class AuthViewModel(private val container: AppContainer) : ViewModel() {
    private val _auth = MutableStateFlow(AuthState.Loading)
    val auth: StateFlow<AuthState> = _auth.asStateFlow()

    private val _login = MutableStateFlow(LoginUiState(baseUrl = container.settings.baseUrl))
    val login: StateFlow<LoginUiState> = _login.asStateFlow()

    private val _homeError = MutableStateFlow<ErrorKind?>(null)
    /** A failed logout, shown on the home screen (the user is still logged in then). */
    val homeError: StateFlow<ErrorKind?> = _homeError.asStateFlow()

    private val _openUrl = Channel<String>(Channel.BUFFERED)
    /** URLs the activity must open in a browser. */
    val openUrl = _openUrl.receiveAsFlow()

    private val mutex = Mutex()

    init {
        // Background sync and the widget follow the login state: on after a login (and at app start while
        // logged in), off after a logout. Every emission redraws the widget, so it never shows a stale account.
        viewModelScope.launch {
            _auth.collect { state ->
                if (state == AuthState.Loading) return@collect
                withContext(Dispatchers.IO) {
                    try {
                        val context = container.appContext
                        if (state == AuthState.LoggedIn) {
                            SyncScheduler.schedulePeriodic(context, container.settings.syncIntervalHours)
                            SyncScheduler.syncNow(context)
                        } else {
                            SyncScheduler.cancelAll(context)
                        }
                        TimetableWidget().updateAll(context)
                    } catch (e: CancellationException) {
                        throw e
                    } catch (t: Throwable) {
                        logFailure(t)
                    }
                }
            }
        }
        viewModelScope.launch {
            mutex.withLock {
                val baseUrl = _login.value.baseUrl
                val state = runIo {
                    val needsKey = !container.hasEmbeddedKey(baseUrl)
                    _login.update { it.copy(needsKey = needsKey) }
                    val loggedIn = container.client(baseUrl)?.hasToken() == true
                    if (loggedIn) AuthState.LoggedIn else AuthState.LoggedOut
                }
                _auth.value = state ?: AuthState.LoggedOut
            }
        }
    }

    fun onBaseUrlChange(value: String) {
        _login.update { it.copy(baseUrl = value) }
        viewModelScope.launch {
            val needsKey = runIo { !container.hasEmbeddedKey(value) } ?: return@launch
            _login.update { if (it.baseUrl == value) it.copy(needsKey = needsKey) else it }
        }
    }

    fun onConsumerKeyChange(value: String) = _login.update { it.copy(consumerKey = value) }

    fun onConsumerSecretChange(value: String) = _login.update { it.copy(consumerSecret = value) }

    fun login() {
        val ui = _login.value
        if (ui.busy) return
        _login.update { it.copy(busy = true, error = null, warnings = emptyList()) }
        viewModelScope.launch {
            mutex.withLock {
                val url = runIo {
                    val baseUrl = ui.baseUrl.trim()
                    container.settings.baseUrl = baseUrl
                    val credentials = if (container.hasEmbeddedKey(baseUrl)) {
                        null
                    } else {
                        SecretCodec.ConsumerCredentials(ui.consumerKey.trim(), ui.consumerSecret.trim())
                    }
                    val client = container.client(baseUrl, credentials) ?: throw AppException(ErrorKind.KEY_MISSING)
                    val begun = client.beginLogin(OAUTH_CALLBACK)
                    // Persist before the browser opens: this process may be killed meanwhile.
                    container.pendingLoginStore.save(client.baseUrl(), begun.state())
                    begun.authorizeUrl()
                }
                if (url != null) {
                    _openUrl.send(url)
                    // The pending login is saved; the button works again if the user comes back without finishing.
                    _login.update { it.copy(busy = false) }
                }
            }
        }
    }

    /** The browser reported an error (for example, no browser installed). */
    fun onBrowserFailed() {
        _login.update { it.copy(busy = false, error = ErrorKind.NO_BROWSER) }
    }

    /** The OAuth redirect URL, delivered to the activity by the `libreusos://` intent filter. */
    fun onRedirect(redirectUrl: String) {
        _login.update { it.copy(busy = true, error = null) }
        viewModelScope.launch {
            mutex.withLock {
                val loggedIn = runIo {
                    // Always from the saved file (never an in-memory copy that can be stale).
                    val saved = container.pendingLoginStore.load() ?: throw AppException(ErrorKind.NO_LOGIN)
                    container.settings.baseUrl = saved.baseUrl
                    _login.update { it.copy(baseUrl = saved.baseUrl) }
                    val client = container.client(saved.baseUrl) ?: throw AppException(ErrorKind.KEY_MISSING)
                    val result = client.resumeLogin(saved.state).complete(redirectUrl)
                    container.pendingLoginStore.clear()
                    result
                }
                if (loggedIn != null) {
                    _login.update { it.copy(busy = false, warnings = loggedIn.warnings) }
                    _auth.value = AuthState.LoggedIn
                }
            }
        }
    }

    fun logout() {
        _homeError.value = null
        container.terms.clear()
        viewModelScope.launch {
            mutex.withLock {
                val warnings = runIo(onError = { _homeError.value = it }) {
                    container.pendingLoginStore.clear()
                    val client = container.client(container.settings.baseUrl)
                    if (client == null) {
                        // No consumer key, so no client: still remove the stored token.
                        container.tokenStore.deleteAll()
                        emptyList()
                    } else {
                        client.logout(revoke = true)
                    }
                }
                if (warnings != null) {
                    _login.update { it.copy(busy = false, warnings = warnings) }
                    _auth.value = AuthState.LoggedOut
                }
            }
        }
    }

    /** USOS rejected the token: forget it and ask for a new login. */
    fun onSessionExpired() {
        viewModelScope.launch {
            mutex.withLock {
                // Already logged out (a second failing call, or the user's own logout): nothing to do.
                if (_auth.value == AuthState.LoggedOut) return@withLock
                container.terms.clear()
                runIo { container.client(container.settings.baseUrl)?.logout(revoke = false) }
                _login.update { it.copy(busy = false, error = ErrorKind.SESSION_EXPIRED, warnings = emptyList()) }
                _auth.value = AuthState.LoggedOut
            }
        }
    }

    /**
     * Runs [block] on `Dispatchers.IO`. On failure it logs the class name only, reports a localized
     * kind through [onError] (the login screen by default), and returns null.
     */
    private suspend fun <T> runIo(
        onError: (ErrorKind) -> Unit = { kind -> _login.update { it.copy(busy = false, error = kind) } },
        block: suspend () -> T,
    ): T? = try {
        withContext(Dispatchers.IO) { block() }
    } catch (e: CancellationException) {
        throw e
    } catch (t: Throwable) {
        // Throwable: a missing native library is an UnsatisfiedLinkError, not an Exception.
        logFailure(t)
        onError(errorKindOf(t))
        null
    }
}
