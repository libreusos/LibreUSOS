package io.gitlab.voxix.libreusos.widget

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.glance.appwidget.updateAll
import io.gitlab.voxix.libreusos.sync.SyncScheduler
import io.gitlab.voxix.libreusos.ui.logFailure
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

/** The clock was set: redraw, and time the boundary refresh again (its delay was computed on the old clock). */
class TimeChangedReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_TIME_CHANGED) return
        val app = context.applicationContext
        val pending = goAsync()
        CoroutineScope(Dispatchers.IO).launch {
            try {
                // Forget the old boundary first, so the update below schedules a fresh one.
                SyncScheduler.cancelWidgetRefresh(app)
                TimetableWidget().updateAll(app)
            } catch (t: Throwable) {
                logFailure(t)
            } finally {
                pending.finish()
            }
        }
    }
}
