package io.gitlab.voxix.libreusos.data

import android.content.Context
import androidx.core.content.edit

/** Non-secret settings. */
class AppSettings(context: Context) {
    private val prefs = context.getSharedPreferences("settings", Context.MODE_PRIVATE)

    var baseUrl: String
        get() = prefs.getString(KEY_BASE_URL, null) ?: DEFAULT_BASE_URL
        set(value) = prefs.edit { putString(KEY_BASE_URL, value) }

    /** Hours between background syncs (WorkManager allows 15 minutes at least; the settings screen comes later). */
    var syncIntervalHours: Long
        get() = prefs.getLong(KEY_SYNC_HOURS, DEFAULT_SYNC_HOURS).coerceAtLeast(1)
        set(value) = prefs.edit { putLong(KEY_SYNC_HOURS, value) }

    companion object {
        const val DEFAULT_SYNC_HOURS = 6L
        private const val KEY_SYNC_HOURS = "sync_interval_hours"

        const val DEFAULT_BASE_URL = "https://usosapi.polsl.pl/"
        private const val KEY_BASE_URL = "base_url"
    }
}
