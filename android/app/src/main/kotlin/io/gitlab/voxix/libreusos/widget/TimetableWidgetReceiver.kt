package io.gitlab.voxix.libreusos.widget

import android.content.Context
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import io.gitlab.voxix.libreusos.sync.SyncScheduler

class TimetableWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = TimetableWidget()

    /** The last widget was removed: stop the boundary refresh chain. */
    override fun onDisabled(context: Context) {
        super.onDisabled(context)
        SyncScheduler.cancelWidgetRefresh(context)
    }
}
