package io.gitlab.voxix.libreusos.sync

import android.content.Context
import androidx.glance.appwidget.updateAll
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import io.gitlab.voxix.libreusos.widget.TimetableWidget

/** Redraws the widget at a boundary (a class ended, midnight). The widget schedules the next one itself. */
class WidgetRefreshWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result {
        TimetableWidget().updateAll(applicationContext)
        return Result.success()
    }
}
