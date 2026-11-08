package io.gitlab.voxix.libreusos.data

import android.content.Context
import androidx.core.content.edit
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** How the app is drawn: the light/dark choice and whether Android 12+ wallpaper colours are used. */
data class Appearance(val themeMode: ThemeMode = ThemeMode.System, val dynamicColor: Boolean = true)

/** Non-secret settings. */
class AppSettings(context: Context) {
    private val prefs = context.getSharedPreferences("settings", Context.MODE_PRIVATE)

    var baseUrl: String
        get() = prefs.getString(KEY_BASE_URL, null) ?: DEFAULT_BASE_URL
        set(value) = prefs.edit { putString(KEY_BASE_URL, value) }

    private val _appearance = MutableStateFlow(
        Appearance(
            ThemeMode.fromStored(prefs.getString(KEY_THEME_MODE, null)),
            prefs.getBoolean(KEY_DYNAMIC_COLOR, true),
        ),
    )

    /** The current appearance; the root composable follows it, so a change applies at once. */
    val appearance: StateFlow<Appearance> = _appearance.asStateFlow()

    fun setThemeMode(mode: ThemeMode) {
        prefs.edit { putString(KEY_THEME_MODE, mode.stored) }
        _appearance.value = _appearance.value.copy(themeMode = mode)
    }

    fun setDynamicColor(enabled: Boolean) {
        prefs.edit { putBoolean(KEY_DYNAMIC_COLOR, enabled) }
        _appearance.value = _appearance.value.copy(dynamicColor = enabled)
    }

    /** Hours between background syncs (WorkManager allows 15 minutes at least). */
    var syncIntervalHours: Long
        get() = prefs.getLong(KEY_SYNC_HOURS, DEFAULT_SYNC_HOURS).coerceAtLeast(1)
        set(value) = prefs.edit { putLong(KEY_SYNC_HOURS, value) }

    companion object {
        const val DEFAULT_SYNC_HOURS = 6L
        private const val KEY_SYNC_HOURS = "sync_interval_hours"
        private const val KEY_THEME_MODE = "theme_mode"
        private const val KEY_DYNAMIC_COLOR = "dynamic_color"

        const val DEFAULT_BASE_URL = "https://usosapi.polsl.pl/"
        private const val KEY_BASE_URL = "base_url"
    }
}
