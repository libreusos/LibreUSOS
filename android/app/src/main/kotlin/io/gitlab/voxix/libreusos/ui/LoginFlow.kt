package io.gitlab.voxix.libreusos.ui

import java.net.URI

/** How long USOS keeps the request token, and so how long the app waits for the browser to come back. */
const val LOGIN_TIMEOUT_MS = 10 * 60 * 1000L

/** The one university this build knows by name; more would need their own consumer key. */
data class KnownUniversity(val name: String, val baseUrl: String)

val SIGN_IN_UNIVERSITIES = listOf(KnownUniversity("Politechnika Śląska", "https://usosapi.polsl.pl/"))

/** The screens of the sign-in flow (SYSTEM.md sections 11 and 12). */
enum class LoginStage {
    /** m-01: the list of universities the app knows. */
    Universities,

    /** m-02: another university, with its address and consumer key. */
    OtherUniversity,

    /** m-03: the authorisation page is open in the browser; step 1 is in progress. */
    Waiting,

    /** The redirect arrived: steps 1 and 2 are done, step 3 finishes the login and runs the first sync. */
    Finishing,

    /** The user came back without approving; nothing was saved. */
    Cancelled,

    /** [LOGIN_TIMEOUT_MS] passed without a redirect. */
    TimedOut,

    /** The user pressed "Odmów" on the university's page. */
    Denied,

    /** Any other failure after the browser was opened; the details sit behind "Szczegóły". */
    Failed,
}

/** What step 3 is doing. */
enum class FinishStep { Exchanging, Syncing }

/** Why the address field is in its error state. */
enum class UrlProblem { NotAnAddress, NotHttps, NoUsosApi }

data class LoginFlowState(
    val stage: LoginStage = LoginStage.Universities,
    val baseUrl: String = "",
    val consumerKey: String = "",
    val consumerSecret: String = "",
    /** The build embeds a consumer key for the known university. */
    val knownHasKey: Boolean = false,
    /** True when this build has no embedded key for [baseUrl], so key and secret must be typed. */
    val needsKey: Boolean = true,
    /** The request token is being fetched. */
    val busy: Boolean = false,
    /** The server said there is no USOS API at [baseUrl]. Cleared when the address changes. */
    val serverUrlProblem: Boolean = false,
    /** USOS rejected the consumer key. Cleared when the key or secret change. */
    val keyRejected: Boolean = false,
    /** The last failure, for the banner (before the browser) or "Szczegóły" (after it). */
    val error: ErrorKind? = null,
    /** The old session expired: a banner on the first screen asks for a new login. */
    val sessionExpired: Boolean = false,
    /** The screen the flow returns to when the user goes back from the browser step. */
    val entry: LoginStage = LoginStage.Universities,
    /** Host of the authorisation page, for step 1. */
    val authHost: String = "",
    val waitingSinceMs: Long = 0,
    /** The app went to the background (the tab opened) while waiting, so a resume without redirect is a cancel. */
    val leftApp: Boolean = false,
    val finishStep: FinishStep = FinishStep.Exchanging,
    val warnings: List<String> = emptyList(),
) {
    /** What the address field shows, or null when it is fine (or still empty). */
    val urlProblem: UrlProblem?
        get() = when {
            serverUrlProblem -> UrlProblem.NoUsosApi
            else -> validateBaseUrl(baseUrl)
        }

    /** Whether "Sprawdź i zaloguj" can be pressed. */
    val canSubmitOther: Boolean
        get() = !busy && baseUrl.isNotBlank() && urlProblem == null &&
            (!needsKey || (consumerKey.isNotBlank() && consumerSecret.isNotBlank()))
}

/** An address problem the app can see without asking the server; empty input is no error (the button is off). */
fun validateBaseUrl(raw: String): UrlProblem? {
    val text = raw.trim()
    if (text.isEmpty()) return null
    val uri = runCatching { URI(text) }.getOrNull() ?: return UrlProblem.NotAnAddress
    val scheme = uri.scheme?.lowercase()
    return when {
        scheme == null || uri.host.isNullOrBlank() -> UrlProblem.NotAnAddress
        scheme == "http" -> UrlProblem.NotHttps
        scheme != "https" -> UrlProblem.NotAnAddress
        else -> null
    }
}

sealed interface LoginEvent {
    /** The first look at the build: the initial screen, the saved address and key availability. */
    data class Loaded(val baseUrl: String, val knownHasKey: Boolean, val needsKey: Boolean) : LoginEvent

    data class NeedsKeyResolved(val baseUrl: String, val needsKey: Boolean) : LoginEvent
    data class OpenOther(val prefillUrl: String?) : LoginEvent
    data object ChooseUniversity : LoginEvent
    data class BaseUrlChanged(val value: String) : LoginEvent
    data class KeyChanged(val value: String) : LoginEvent
    data class SecretChanged(val value: String) : LoginEvent

    /** "Zaloguj" was pressed: the request token is being fetched. */
    data object Submitted : LoginEvent
    data class BeginFailed(val kind: ErrorKind) : LoginEvent
    data class BrowserOpened(val authHost: String, val nowMs: Long) : LoginEvent
    data object BrowserMissing : LoginEvent

    /** The activity stopped (another screen, the browser tab, covers it); not for rotation. */
    data object AppStopped : LoginEvent
    data class AppResumed(val nowMs: Long) : LoginEvent

    /** Time check, from a timer. */
    data class Tick(val nowMs: Long) : LoginEvent

    /** The OAuth redirect URL arrived (also after the app was restarted). */
    data object RedirectReceived : LoginEvent
    data class Exchanged(val warnings: List<String>) : LoginEvent
    data class ExchangeFailed(val kind: ErrorKind) : LoginEvent

    /** The user backed out of the browser step. */
    data object Abandoned : LoginEvent
    data object SessionExpired : LoginEvent

    /** The user logged out; [warnings] are problems that did not stop it (for example, a token that could not be revoked). */
    data class LoggedOut(val warnings: List<String>) : LoginEvent
}

/** The pure state machine of the sign-in flow: every transition is one `when` branch. */
fun reduceLogin(state: LoginFlowState, event: LoginEvent): LoginFlowState = when (event) {
    // The OAuth redirect can start a fresh process: its exchange is already on screen and must not be reset to the list.
    is LoginEvent.Loaded -> if (state.stage == LoginStage.Finishing) {
        state.copy(knownHasKey = event.knownHasKey, needsKey = event.needsKey)
    } else {
        val known = SIGN_IN_UNIVERSITIES.any { sameServer(it.baseUrl, event.baseUrl) }
        val startOnList = event.knownHasKey && (known || event.baseUrl.isBlank())
        state.copy(
            stage = if (startOnList) LoginStage.Universities else LoginStage.OtherUniversity,
            entry = if (startOnList) LoginStage.Universities else LoginStage.OtherUniversity,
            baseUrl = event.baseUrl,
            knownHasKey = event.knownHasKey,
            needsKey = event.needsKey,
        )
    }

    is LoginEvent.NeedsKeyResolved ->
        if (event.baseUrl == state.baseUrl) state.copy(needsKey = event.needsKey) else state

    is LoginEvent.OpenOther -> state.copy(
        stage = LoginStage.OtherUniversity,
        entry = LoginStage.OtherUniversity,
        baseUrl = event.prefillUrl ?: state.baseUrl,
        serverUrlProblem = false,
        keyRejected = false,
        error = null,
        sessionExpired = false,
    )

    LoginEvent.ChooseUniversity -> state.copy(
        stage = LoginStage.Universities,
        entry = LoginStage.Universities,
        busy = false,
        serverUrlProblem = false,
        keyRejected = false,
        error = null,
        warnings = emptyList(),
    )

    is LoginEvent.BaseUrlChanged -> state.copy(baseUrl = event.value, serverUrlProblem = false, error = null)

    is LoginEvent.KeyChanged -> state.copy(consumerKey = event.value, keyRejected = false, error = null)

    is LoginEvent.SecretChanged -> state.copy(consumerSecret = event.value, keyRejected = false, error = null)

    LoginEvent.Submitted -> state.copy(
        busy = true,
        error = null,
        serverUrlProblem = false,
        keyRejected = false,
        sessionExpired = false,
        warnings = emptyList(),
        // The first screen the user filled in is where "back" leads; a retry from a result screen keeps it.
        entry = if (state.stage == LoginStage.Universities || state.stage == LoginStage.OtherUniversity) state.stage else state.entry,
    )

    is LoginEvent.BeginFailed -> when (state.stage) {
        LoginStage.Universities -> state.copy(busy = false, error = event.kind)
        LoginStage.OtherUniversity -> when (event.kind) {
            // No USOS API at the address, or one that does not understand the request.
            ErrorKind.API, ErrorKind.INVALID_INPUT -> state.copy(busy = false, serverUrlProblem = true)
            // USOS rejected the consumer key (401 "Invalid consumer").
            ErrorKind.UNAUTHORIZED -> state.copy(busy = false, keyRejected = true)
            else -> state.copy(busy = false, error = event.kind)
        }
        // A retry from a result screen that failed again.
        else -> state.copy(stage = LoginStage.Failed, busy = false, error = event.kind)
    }

    is LoginEvent.BrowserOpened -> state.copy(
        stage = LoginStage.Waiting,
        busy = false,
        authHost = event.authHost,
        waitingSinceMs = event.nowMs,
        leftApp = false,
        error = null,
    )

    LoginEvent.BrowserMissing -> when (state.stage) {
        LoginStage.Waiting, LoginStage.Finishing -> state.copy(stage = LoginStage.Failed, error = ErrorKind.NO_BROWSER)
        // Opening failed before the Waiting screen showed (cannot normally happen: BrowserOpened comes first).
        else -> state.copy(busy = false, error = ErrorKind.NO_BROWSER)
    }

    LoginEvent.AppStopped -> if (state.stage == LoginStage.Waiting) state.copy(leftApp = true) else state

    is LoginEvent.AppResumed -> when {
        state.stage != LoginStage.Waiting -> state
        event.nowMs - state.waitingSinceMs >= LOGIN_TIMEOUT_MS -> state.copy(stage = LoginStage.TimedOut, leftApp = false)
        state.leftApp -> state.copy(stage = LoginStage.Cancelled, leftApp = false)
        else -> state
    }

    is LoginEvent.Tick ->
        if (state.stage == LoginStage.Waiting && event.nowMs - state.waitingSinceMs >= LOGIN_TIMEOUT_MS) {
            state.copy(stage = LoginStage.TimedOut, leftApp = false)
        } else {
            state
        }

    // Accepted from any screen: a redirect also arrives after a cancel (the tab was closed late) and after a restart.
    LoginEvent.RedirectReceived -> state.copy(
        stage = LoginStage.Finishing,
        finishStep = FinishStep.Exchanging,
        busy = false,
        leftApp = false,
        error = null,
        sessionExpired = false,
    )

    is LoginEvent.Exchanged -> state.copy(finishStep = FinishStep.Syncing, warnings = event.warnings)

    is LoginEvent.ExchangeFailed -> state.copy(
        stage = if (event.kind == ErrorKind.UNAUTHORIZED) LoginStage.Denied else LoginStage.Failed,
        error = event.kind,
        leftApp = false,
    )

    LoginEvent.Abandoned -> state.copy(
        stage = state.entry,
        busy = false,
        leftApp = false,
        error = null,
        warnings = emptyList(),
    )

    LoginEvent.SessionExpired -> state.copy(
        stage = state.entry,
        busy = false,
        sessionExpired = true,
        error = null,
        warnings = emptyList(),
    )

    is LoginEvent.LoggedOut -> state.copy(
        stage = state.entry,
        busy = false,
        sessionExpired = false,
        error = null,
        leftApp = false,
        warnings = event.warnings,
    )
}

/** Whether two API addresses name the same server (scheme and trailing slash aside). */
fun sameServer(a: String, b: String): Boolean = hostOf(a).equals(hostOf(b), ignoreCase = true)

/** The name to show for the known university at [baseUrl], or null. */
fun knownUniversityAt(baseUrl: String): KnownUniversity? = SIGN_IN_UNIVERSITIES.firstOrNull { sameServer(it.baseUrl, baseUrl) }
