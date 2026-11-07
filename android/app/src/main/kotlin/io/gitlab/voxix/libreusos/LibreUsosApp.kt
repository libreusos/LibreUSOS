package io.gitlab.voxix.libreusos

import android.app.Application
import androidx.work.Configuration

/**
 * WorkManager starts on demand (its startup initializer is removed in the manifest): workers and the
 * widget find the shared [container] through this class, which exists before any of them runs.
 */
class LibreUsosApp : Application(), Configuration.Provider {
    lateinit var container: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        container = AppContainer(this)
    }

    override val workManagerConfiguration: Configuration
        get() = Configuration.Builder().build()
}
