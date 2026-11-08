package io.gitlab.voxix.libreusos.sync

import android.content.Context
import androidx.work.Constraints
import androidx.work.Data
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.await
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.withTimeoutOrNull
import java.util.concurrent.TimeUnit

/** Everything the app asks WorkManager for. Unique work names make every call idempotent. */
object SyncScheduler {
    private const val PERIODIC = "sync-periodic"
    private const val NOW = "sync-now"
    private const val WIDGET_REFRESH = "widget-refresh"

    private const val RETRY = "sync-retry"

    /** Input of [SyncWorker]: this run is the planned retry after a failure (it does nothing when a sync worked since). */
    internal const val KEY_IS_RETRY = "is_retry"

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
            .build()
        val policy = if (replace) ExistingPeriodicWorkPolicy.UPDATE else ExistingPeriodicWorkPolicy.KEEP
        WorkManager.getInstance(context).enqueueUniquePeriodicWork(PERIODIC, policy, request)
    }

    /** One sync as soon as the network is there (the app opened). Retries after a failure go through [scheduleRetry]. */
    fun syncNow(context: Context): androidx.work.Operation {
        val request = OneTimeWorkRequestBuilder<SyncWorker>().setConstraints(online).build()
        return WorkManager.getInstance(context).enqueueUniqueWork(NOW, ExistingWorkPolicy.KEEP, request)
    }

    /**
     * The retry after a failed sync, in [delayMs] (SYSTEM.md section 11: 15 min, 1 h, 6 h). WorkManager's own
     * backoff stops at 5 h and cannot show the user the next step, so this is a plain one-off that replaces
     * any earlier one. It is a separate unique work so a running sync is never replaced.
     */
    fun scheduleRetry(context: Context, delayMs: Long) {
        val request = OneTimeWorkRequestBuilder<SyncWorker>()
            .setConstraints(online)
            .setInitialDelay(delayMs, TimeUnit.MILLISECONDS)
            .setInputData(Data.Builder().putBoolean(KEY_IS_RETRY, true).build())
            .build()
        WorkManager.getInstance(context).enqueueUniqueWork(RETRY, ExistingWorkPolicy.REPLACE, request)
    }

    /**
     * [syncNow], then suspends until no sync-now work is left unfinished (any outcome), or [timeoutMs] passes
     * (no network, battery low: the work keeps waiting for its constraints). Used by the first sync after login.
     */
    suspend fun syncNowAndWait(context: Context, timeoutMs: Long) {
        val wm = WorkManager.getInstance(context)
        syncNow(context).await()
        withTimeoutOrNull(timeoutMs) {
            wm.getWorkInfosForUniqueWorkFlow(NOW).first { infos -> infos.all { it.state.isFinished } }
        }
    }

    /** Logout: no more syncing, no widget refresh chain. */
    fun cancelAll(context: Context) {
        val wm = WorkManager.getInstance(context)
        wm.cancelUniqueWork(PERIODIC)
        wm.cancelUniqueWork(NOW)
        wm.cancelUniqueWork(RETRY)
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
