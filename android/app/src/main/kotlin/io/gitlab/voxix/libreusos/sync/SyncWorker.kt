package io.gitlab.voxix.libreusos.sync

import android.content.Context
import androidx.glance.appwidget.updateAll
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import io.gitlab.voxix.libreusos.LibreUsosApp
import io.gitlab.voxix.libreusos.data.SyncStore
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
        val store = container.syncStore
        // The planned retry has nothing to do when a sync worked in the meantime.
        if (inputData.getBoolean(SyncScheduler.KEY_IS_RETRY, false) && store.record.value.nextAttemptAtMs == null) {
            return Result.success()
        }
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
                store.succeeded()
                Result.success()
            }
        } catch (e: CancellationException) {
            throw e
        } catch (t: Throwable) {
            // Class name only: core messages and tokens never reach logcat.
            logFailure(t)
            if (outcomeOf(t) == SyncOutcome.StopUnauthorized) {
                // The session is dead: no more syncing. The 401 is not retried: the app shows "Sesja USOS
                // wygasła" and asks for a new login, and the widget shows its "Zaloguj ponownie" chip.
                SyncScheduler.cancelPeriodic(applicationContext)
                store.setExpired(true)
                TimetableWidget().updateAll(applicationContext)
            } else {
                store.failed()
            }
            Result.failure()
        }
        if (synced) TimetableWidget().updateAll(applicationContext)
        return result
    }
}

/** What a failed sync means: another attempt is planned by the app's own backoff (see [SyncStore]), or never. */
enum class SyncOutcome { Failure, StopUnauthorized }

/** Pure mapping from a sync error to its outcome. A periodic work that fails still runs again in its next period. */
fun outcomeOf(t: Throwable): SyncOutcome =
    if (t is UsosException.Unauthorized) SyncOutcome.StopUnauthorized else SyncOutcome.Failure
