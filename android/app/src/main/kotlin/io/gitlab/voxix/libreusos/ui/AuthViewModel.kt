package io.gitlab.voxix.libreusos.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.gitlab.voxix.libreusos.AppContainer
import androidx.glance.appwidget.updateAll
import io.gitlab.voxix.libreusos.data.SecretCodec
import io.gitlab.voxix.libreusos.data.isStale
import io.gitlab.voxix.libreusos.sync.SyncScheduler
import io.gitlab.voxix.libreusos.widget.TimetableWidget
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
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

/** The longest the first sync may keep the login screen. */
private const val FIRST_SYNC_WAIT_MS = 20_000L

const val OAUTH_CALLBACK = "libreusos://oauth-callback"

enum class AuthState { Loading, LoggedOut, LoggedIn }

/** Activity-scoped: owns the login flow, which outlives the screens. Never logs tokens. */
class AuthViewModel(private val container: AppContainer) : ViewModel() {
    private val _auth = MutableStateFlow(AuthState.Loading)
    val auth: StateFlow<AuthState> = _auth.asStateFlow()

    private val _login = MutableStateFlow(LoginFlowState(baseUrl = container.settings.baseUrl))
    val login: StateFlow<LoginFlowState> = _login.asStateFlow()

    private var timeoutJob: Job? = null

    /** The coroutine of a `login()` that may still be waiting for USOS; [abandonPending] cancels it. */
    private var loginJob: Job? = null

    /** The authorisation page of the login in progress (not a secret: it carries the public request token only). */
    private var authorizeUrl: String? = null

    /** The login itself started the first sync, so the auth observer must not start a second one. */
    @Volatile private var firstSyncStarted = false

    private val _homeError = MutableStateFlow<ErrorKind?>(null)
    /** A failed logout, shown in a dialog over the shell (the user is still logged in then). */
    val homeError: StateFlow<ErrorKind?> = _homeError.asStateFlow()

    private val _openUrl = Channel<String>(Channel.BUFFERED)
    /** URLs the activity must open in a browser. */
    val openUrl = _openUrl.receiveAsFlow()

    private val _sessionExpired = MutableStateFlow(false)
    /** USOS rejected the token while the app is open: the screens keep their data and offer [relogin]. */
    val sessionExpired: StateFlow<Boolean> = _sessionExpired.asStateFlow()

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
                            val record = container.syncStore.record.value
                            // A worker that met a 401 stopped the schedule: it stays stopped until a new login.
                            if (!record.expired) {
                                SyncScheduler.schedulePeriodic(context, container.settings.syncIntervalHours)
                                // On app start only when the data is older than 15 minutes (SYSTEM.md section 11).
                                val stale = isStale(record.lastSuccessMs, System.currentTimeMillis())
                                if (!firstSyncStarted && stale) SyncScheduler.syncNow(context)
                            }
                            firstSyncStarted = false
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
                    val knownHasKey = container.hasEmbeddedKey(SIGN_IN_UNIVERSITIES.first().baseUrl)
                    dispatch(LoginEvent.Loaded(baseUrl, knownHasKey, needsKey))
                    val loggedIn = container.client(baseUrl)?.hasToken() == true
                    if (loggedIn) AuthState.LoggedIn else AuthState.LoggedOut
                }
                _auth.value = state ?: AuthState.LoggedOut
            }
        }
    }

    private fun dispatch(event: LoginEvent) = _login.update { reduceLogin(it, event) }

    fun onBaseUrlChange(value: String) {
        dispatch(LoginEvent.BaseUrlChanged(value))
        viewModelScope.launch {
            val needsKey = runIo { !container.hasEmbeddedKey(value) } ?: return@launch
            dispatch(LoginEvent.NeedsKeyResolved(value, needsKey))
        }
    }

    fun onConsumerKeyChange(value: String) = dispatch(LoginEvent.KeyChanged(value))

    fun onConsumerSecretChange(value: String) = dispatch(LoginEvent.SecretChanged(value))

    /** "Inna uczelnia": the form for an address and a consumer key. */
    fun openOtherUniversity(prefillUrl: String? = null) {
        dispatch(LoginEvent.OpenOther(prefillUrl))
        prefillUrl?.let(::onBaseUrlChange)
    }

    fun chooseUniversity() {
        abandonPending()
        dispatch(LoginEvent.ChooseUniversity)
    }

    /** The back arrow of the browser step: forget this attempt and return to the screen it began on. */
    fun abandon() {
        abandonPending()
        dispatch(LoginEvent.Abandoned)
    }

    private fun abandonPending() {
        timeoutJob?.cancel()
        loginJob?.cancel()
        // Under the mutex: it runs after a cancelled login released it, so a login that saved a
        // pending state just before the cancellation is cleared too, and a later login saves after it.
        viewModelScope.launch { mutex.withLock { runIo(onError = {}) { container.pendingLoginStore.clear() } } }
    }

    /** Sign in at the known university (the one with a key built into the app). */
    fun loginKnown() {
        val known = SIGN_IN_UNIVERSITIES.first()
        if (!_login.value.knownHasKey) {
            // This build has no key for it: the same form as any other university, with the address filled in.
            openOtherUniversity(known.baseUrl)
            return
        }
        _login.update { it.copy(baseUrl = known.baseUrl, needsKey = false) }
        login()
    }

    fun login() {
        val ui = _login.value
        if (ui.busy) return
        timeoutJob?.cancel()
        dispatch(LoginEvent.Submitted)
        loginJob = viewModelScope.launch {
            mutex.withLock {
                val url = runIo(onError = { dispatch(LoginEvent.BeginFailed(it)) }) {
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
                    authorizeUrl = url
                    val now = System.currentTimeMillis()
                    dispatch(LoginEvent.BrowserOpened(hostOf(url), now))
                    timeoutJob = viewModelScope.launch {
                        delay(LOGIN_TIMEOUT_MS)
                        dispatch(LoginEvent.Tick(System.currentTimeMillis()))
                    }
                    _openUrl.send(url)
                }
            }
        }
    }

    /** "Otwórz stronę ponownie": the same page again; a new login when the process lost it. */
    fun reopenPage() {
        val url = authorizeUrl
        if (url == null) login() else viewModelScope.launch { _openUrl.send(url) }
    }

    /** The browser reported an error (for example, no browser installed). */
    fun onBrowserFailed() {
        timeoutJob?.cancel()
        dispatch(LoginEvent.BrowserMissing)
    }

    /** The activity went out of sight (not a rotation): a browser tab may be covering it. */
    fun onAppStopped() = dispatch(LoginEvent.AppStopped)

    /** The activity is back. Without a redirect first, a waiting login was cancelled or has run out. */
    fun onAppResumed() = dispatch(LoginEvent.AppResumed(System.currentTimeMillis()))

    /** The OAuth redirect URL, delivered to the activity by the `libreusos://` intent filter. */
    fun onRedirect(redirectUrl: String) {
        timeoutJob?.cancel()
        dispatch(LoginEvent.RedirectReceived)
        viewModelScope.launch {
            mutex.withLock {
                val loggedIn = runIo(onError = { dispatch(LoginEvent.ExchangeFailed(it)) }) {
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
                    _sessionExpired.value = false
                    container.syncStore.setExpired(false)
                    dispatch(LoginEvent.Exchanged(loggedIn.warnings))
                    // Step 3: the first sync, so the first screen is not empty. A slow network ends the wait.
                    firstSyncStarted = true
                    runIo(onError = {}) {
                        SyncScheduler.schedulePeriodic(container.appContext, container.settings.syncIntervalHours)
                        SyncScheduler.syncNowAndWait(container.appContext, FIRST_SYNC_WAIT_MS)
                    }
                    _auth.value = AuthState.LoggedIn
                }
            }
        }
    }

    fun dismissLogoutError() {
        _homeError.value = null
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
                    _sessionExpired.value = false
                    container.syncStore.clear()
                    dispatch(LoginEvent.LoggedOut(warnings))
                    _auth.value = AuthState.LoggedOut
                }
            }
        }
    }

    /**
     * USOS rejected the token. Nothing is deleted: the cached data stays on screen under a "session
     * expired" banner, and the dead token is replaced when [relogin] ends in a new login (the same
     * account keeps its cache; another one replaces it in the core). No more background syncing.
     */
    fun onSessionExpired() {
        if (!_sessionExpired.compareAndSet(expect = false, update = true)) return
        viewModelScope.launch {
            runIo(onError = {}) {
                SyncScheduler.cancelAll(container.appContext)
                container.syncStore.setExpired(true)
                // After cancelAll: the redraw schedules the next boundary again, the stale chips still need it.
                TimetableWidget().updateAll(container.appContext)
            }
        }
    }

    /** "Zaloguj ponownie": the login screen with the university of the expired session still selected. */
    fun relogin() {
        dispatch(LoginEvent.SessionExpired)
        _auth.value = AuthState.LoggedOut
    }

    /**
     * Runs [block] on `Dispatchers.IO`. On failure it logs the class name only, reports a localized
     * kind through [onError] (the login screen by default), and returns null.
     */
    private suspend fun <T> runIo(
        onError: (ErrorKind) -> Unit = { kind -> dispatch(LoginEvent.BeginFailed(kind)) },
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
