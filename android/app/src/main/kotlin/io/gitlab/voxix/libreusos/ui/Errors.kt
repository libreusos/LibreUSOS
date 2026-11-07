package io.gitlab.voxix.libreusos.ui

import android.util.Log
import androidx.annotation.StringRes
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.security.SecretBoxException
import org.libreusos.core.UsosException
import java.io.IOException

/** What went wrong, as the screens show it (localized). Raw exception text is never shown. */
enum class ErrorKind(@StringRes val messageRes: Int) {
    NETWORK(R.string.error_network),
    UNAUTHORIZED(R.string.error_unauthorized),
    API(R.string.error_api),
    NOT_CACHED(R.string.error_not_cached),
    STORAGE(R.string.error_storage),
    INVALID_INPUT(R.string.error_invalid_input),
    KEY_MISSING(R.string.error_key_missing),
    NO_LOGIN(R.string.error_no_login),
    NO_BROWSER(R.string.error_no_browser),
    SESSION_EXPIRED(R.string.error_session_expired),
    UNKNOWN(R.string.error_unknown),
}

/** An app-level failure with a known [kind]. */
class AppException(val kind: ErrorKind) : Exception(kind.name)

fun errorKindOf(t: Throwable): ErrorKind = when (t) {
    is AppException -> t.kind
    is UsosException.Network -> ErrorKind.NETWORK
    is UsosException.Unauthorized -> ErrorKind.UNAUTHORIZED
    is UsosException.Api -> ErrorKind.API
    is UsosException.NotCached -> ErrorKind.NOT_CACHED
    is UsosException.Store, is SecretBoxException, is IOException -> ErrorKind.STORAGE
    is UsosException.InvalidInput -> ErrorKind.INVALID_INPUT
    else -> ErrorKind.UNKNOWN
}

/**
 * Logs only the class name of a failure: messages of core errors and tokens must never reach
 * logcat, and nothing here prints a TokenRecord or a PendingLoginState.
 */
fun logFailure(t: Throwable) {
    Log.w("LibreUSOS", "Failure: ${t.javaClass.name}")
}
