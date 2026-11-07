package io.gitlab.voxix.libreusos.sync

import android.content.Context
import androidx.work.BackoffPolicy
import androidx.work.Constraints
import androidx.work.Data
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import java.util.concurrent.TimeUnit

/** Everything the app asks WorkManager for. Unique work names make every call idempotent. */
object SyncScheduler {
    private const val PERIODIC = "sync-periodic"
    private const val NOW = "sync-now"
    private const val WIDGET_REFRESH = "widget-refresh"

    /** Input of [SyncWorker]: whether a network failure should be retried with backoff. */
    internal const val KEY_RETRY = "retry"

    private val online = Constraints.Builder()
        .setRequiredNetworkType(NetworkType.CONNECTED)
        .setRequiresBatteryNotLow(true)
        .build()

    /**
     * Periodic sync every [hours] (KEEP: an existing schedule stays, with its timing). To apply a changed
     * interval later, call it with [replace], which updates the schedule in place.
     */
    fun schedulePeriodic(context: Context, hours: Long, replace: Boolean = false) {
        val request = PeriodicWorkRequestBuilder<SyncWorker>(hours, TimeUnit.HOURS)
            .setConstraints(online)
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, 5, TimeUnit.MINUTES)
            .build()
        val policy = if (replace) ExistingPeriodicWorkPolicy.UPDATE else ExistingPeriodicWorkPolicy.KEEP
        WorkManager.getInstance(context).enqueueUniquePeriodicWork(PERIODIC, policy, request)
    }

    /** One sync as soon as the network is there (the app opened). No retry loop: the periodic one covers that. */
    fun syncNow(context: Context) {
        val request = OneTimeWorkRequestBuilder<SyncWorker>()
            .setConstraints(online)
            .setInputData(Data.Builder().putBoolean(KEY_RETRY, false).build())
            .build()
        WorkManager.getInstance(context).enqueueUniqueWork(NOW, ExistingWorkPolicy.KEEP, request)
    }

    /** Logout: no more syncing, no widget refresh chain. */
    fun cancelAll(context: Context) {
        val wm = WorkManager.getInstance(context)
        wm.cancelUniqueWork(PERIODIC)
        wm.cancelUniqueWork(NOW)
        cancelWidgetRefresh(context)
    }

    fun cancelPeriodic(context: Context) {
        WorkManager.getInstance(context).cancelUniqueWork(PERIODIC)
    }

    /**
     * Makes sure one inexact one-off redraws the widget at [atMs] (epoch millis; a class ends, or midnight).
     * Idempotent: the same boundary is not scheduled twice (several widget instances, repeated updates).
     * A pending later-or-earlier refresh is replaced; one that is due or running is never cancelled:
     * the new one is appended and waits for it.
     */
    @Synchronized
    fun scheduleWidgetRefresh(context: Context, atMs: Long) {
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val now = System.currentTimeMillis()
        val pending = prefs.getLong(KEY_REFRESH_AT, 0L)
        if (pending == atMs) return
        val request = OneTimeWorkRequestBuilder<WidgetRefreshWorker>()
            .setInitialDelay((atMs - now).coerceAtLeast(MIN_DELAY_MS), TimeUnit.MILLISECONDS)
            .build()
        val policy = if (pending > now) ExistingWorkPolicy.REPLACE else ExistingWorkPolicy.APPEND_OR_REPLACE
        WorkManager.getInstance(context).enqueueUniqueWork(WIDGET_REFRESH, policy, request)
        prefs.edit().putLong(KEY_REFRESH_AT, atMs).apply()
    }

    /** Forgets the pending boundary refresh (the last widget went, or logout, or the clock changed). */
    @Synchronized
    fun cancelWidgetRefresh(context: Context) {
        WorkManager.getInstance(context).cancelUniqueWork(WIDGET_REFRESH)
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().remove(KEY_REFRESH_AT).apply()
    }

    private const val PREFS = "widget_refresh"
    private const val KEY_REFRESH_AT = "refresh_at"
    private const val MIN_DELAY_MS = 60_000L
}
