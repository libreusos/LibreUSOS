package io.gitlab.voxix.libreusos.sync

import android.content.Context
import androidx.glance.appwidget.updateAll
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import io.gitlab.voxix.libreusos.LibreUsosApp
import io.gitlab.voxix.libreusos.data.todayIn
import io.gitlab.voxix.libreusos.ui.logFailure
import io.gitlab.voxix.libreusos.widget.TimetableWidget
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.libreusos.core.UsosException
import org.libreusos.core.installationTimeZone

/** Weeks of timetable one sync fetches, counted from the Monday of the current week. */
const val SYNC_WEEKS = 3u

/**
 * Refreshes the core cache with the app's shared client, then redraws the widgets.
 * Unauthorized is the app's business (it shows the login on its next start); here it only stops the schedule.
 */
class SyncWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result {
        val container = (applicationContext as LibreUsosApp).container
        val retryNetwork = inputData.getBoolean(SyncScheduler.KEY_RETRY, true)
        var synced = false
        val result = try {
            withContext(Dispatchers.IO) {
                val client = container.client(container.settings.baseUrl) ?: return@withContext Result.failure()
                if (!client.hasToken()) {
                    // Logged out meanwhile: nothing to sync.
                    SyncScheduler.cancelPeriodic(applicationContext)
                    return@withContext Result.success()
                }
                client.sync(todayIn(installationTimeZone()).toString(), SYNC_WEEKS)
                synced = true
                Result.success()
            }
        } catch (e: CancellationException) {
            throw e
        } catch (t: Throwable) {
            // Class name only: core messages and tokens never reach logcat.
            logFailure(t)
            val outcome = outcomeOf(t, retryNetwork, runAttemptCount)
            if (outcome == SyncOutcome.StopUnauthorized) {
                // The session is dead: no more syncing and no widget refresh chain; redraw once.
                SyncScheduler.cancelPeriodic(applicationContext)
                SyncScheduler.cancelWidgetRefresh(applicationContext)
                TimetableWidget().updateAll(applicationContext)
            }
            if (outcome == SyncOutcome.Retry) Result.retry() else Result.failure()
        }
        if (synced) TimetableWidget().updateAll(applicationContext)
        return result
    }

}

/** What a failed sync means for WorkManager. */
enum class SyncOutcome { Retry, Failure, StopUnauthorized }

private const val MAX_NETWORK_ATTEMPTS = 5
private const val MAX_API_ATTEMPTS = 3

/**
 * Pure mapping from a sync error to its outcome. [runAttempt] is WorkManager's `runAttemptCount`
 * (0 on the first run). A periodic work that fails still runs again in its next period.
 */
fun outcomeOf(t: Throwable, retryNetwork: Boolean, runAttempt: Int): SyncOutcome = when (t) {
    is UsosException.Unauthorized -> SyncOutcome.StopUnauthorized
    is UsosException.Network ->
        if (retryNetwork && runAttempt < MAX_NETWORK_ATTEMPTS) SyncOutcome.Retry else SyncOutcome.Failure
    is UsosException.Api ->
        if (retryNetwork && runAttempt < MAX_API_ATTEMPTS) SyncOutcome.Retry else SyncOutcome.Failure
    else -> SyncOutcome.Failure
}
