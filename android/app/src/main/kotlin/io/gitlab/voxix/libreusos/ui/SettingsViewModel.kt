package io.gitlab.voxix.libreusos.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.gitlab.voxix.libreusos.AppContainer
import io.gitlab.voxix.libreusos.data.Appearance
import io.gitlab.voxix.libreusos.data.ThemeMode
import io.gitlab.voxix.libreusos.data.nearestSyncInterval
import io.gitlab.voxix.libreusos.sync.SyncScheduler
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** Only settings the app honours today: appearance and the background sync interval. */
class SettingsViewModel(private val container: AppContainer) : ViewModel() {
    val appearance: StateFlow<Appearance> = container.settings.appearance

    private val _syncHours = MutableStateFlow(nearestSyncInterval(container.settings.syncIntervalHours))
    val syncHours: StateFlow<Long> = _syncHours.asStateFlow()

    fun setThemeMode(mode: ThemeMode) = container.settings.setThemeMode(mode)

    fun setDynamicColor(enabled: Boolean) = container.settings.setDynamicColor(enabled)

    /** Stores the interval and applies it to the running schedule at once. */
    fun setSyncHours(hours: Long) {
        if (hours == _syncHours.value) return
        container.settings.syncIntervalHours = hours
        _syncHours.value = hours
        viewModelScope.launch {
            try {
                // An expired session has no schedule to update (a new login starts it with the stored interval).
                if (!container.syncStore.record.value.expired) {
                    withContext(Dispatchers.IO) { SyncScheduler.schedulePeriodic(container.appContext, hours, replace = true) }
                }
            } catch (e: CancellationException) {
                throw e
            } catch (t: Throwable) {
                logFailure(t)
            }
        }
    }
}
