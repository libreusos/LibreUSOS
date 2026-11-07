package io.gitlab.voxix.libreusos

import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.getValue
import androidx.browser.customtabs.CustomTabsIntent
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import io.gitlab.voxix.libreusos.ui.App
import io.gitlab.voxix.libreusos.ui.AuthState
import io.gitlab.voxix.libreusos.ui.AuthViewModel
import io.gitlab.voxix.libreusos.ui.TAB_TIMETABLE
import io.gitlab.voxix.libreusos.widget.ACTION_OPEN_TIMETABLE
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * singleTask: the OAuth redirect `libreusos://oauth-callback?...` comes back to this instance
 * through [onNewIntent], or starts it after the process died while the browser was open.
 */
class MainActivity : ComponentActivity() {
    private val container get() = (application as LibreUsosApp).container

    private val authViewModel: AuthViewModel by lazy {
        ViewModelProvider(
            this,
            viewModelFactory { initializer { AuthViewModel(container) } },
        )[AuthViewModel::class.java]
    }

    /** The last redirect handled; saved, so a recreated activity does not process it twice. */
    private var handledRedirect: String? = null

    /** A tab another screen (the widget) asked for; the shell consumes it once it shows. */
    private val requestedTab = MutableStateFlow<String?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        handledRedirect = savedInstanceState?.getString(STATE_HANDLED_REDIRECT)
        enableEdgeToEdge()
        setContent {
            val tab by requestedTab.asStateFlow().collectAsStateWithLifecycle()
            App(authViewModel, container, tab, onTabHandled = { requestedTab.value = null })
        }

        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                authViewModel.openUrl.collect { openInBrowser(it) }
            }
        }
        // A tab asked for while logged out must not fire after a later login. (Loading keeps it: cold start.)
        lifecycleScope.launch {
            authViewModel.auth.collect { if (it == AuthState.LoggedOut) requestedTab.value = null }
        }
        handleIntent(intent)
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        outState.putString(STATE_HANDLED_REDIRECT, handledRedirect)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleIntent(intent)
    }

    private fun handleIntent(intent: Intent) {
        if (intent.action == ACTION_OPEN_TIMETABLE) {
            // Consume it: a recreation must not jump to the tab again.
            intent.action = Intent.ACTION_MAIN
            requestedTab.value = TAB_TIMETABLE
            return
        }
        val data = intent.data ?: return
        if (intent.action != Intent.ACTION_VIEW) return
        // An old redirect replayed from the recents list is stale.
        if (intent.flags and Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY != 0) return
        if (data.scheme != "libreusos" || data.host != "oauth-callback") return
        val redirect = data.toString()
        // Consume it: neither a recreation nor a second delivery may process it again.
        intent.data = null
        if (redirect == handledRedirect) return
        handledRedirect = redirect
        authViewModel.onRedirect(redirect)
    }

    /** A Custom Tab when a browser supports it; otherwise (or on failure) plain ACTION_VIEW. */
    private fun openInBrowser(url: String) {
        val uri = Uri.parse(url)
        try {
            CustomTabsIntent.Builder().build().launchUrl(this, uri)
        } catch (_: ActivityNotFoundException) {
            try {
                startActivity(Intent(Intent.ACTION_VIEW, uri))
            } catch (_: ActivityNotFoundException) {
                authViewModel.onBrowserFailed()
            }
        }
    }

    private companion object {
        const val STATE_HANDLED_REDIRECT = "handled_redirect"
    }
}
