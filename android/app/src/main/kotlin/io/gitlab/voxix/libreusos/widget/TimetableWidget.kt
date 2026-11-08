package io.gitlab.voxix.libreusos.widget

import android.content.Context
import android.os.Build
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.GlanceTheme
import androidx.glance.Image
import androidx.glance.ImageProvider
import androidx.glance.action.ActionParameters
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.SizeMode
import androidx.glance.appwidget.action.ActionCallback
import androidx.glance.appwidget.appWidgetBackground
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.provideContent
import androidx.glance.background
import androidx.glance.ColorFilter
import androidx.glance.layout.Box
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.padding
import androidx.glance.material3.ColorProviders
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.sync.SyncScheduler
import io.gitlab.voxix.libreusos.ui.theme.BrandDark
import io.gitlab.voxix.libreusos.ui.theme.BrandLight

/**
 * The home-screen widget (SYSTEM.md section 13): today's classes, then the next day with classes.
 * Reads the core cache only; the one-off redraw below keeps the picture current between syncs.
 */
class TimetableWidget : GlanceAppWidget() {
    // Each candidate renders once and the launcher shows the largest one that fits, so the six
    // layouts of section 13.1 plus the intermediate heights that change how many rows fit are listed here.
    // shortcut: the heights are fixed steps, so a widget sized between two steps leaves up to one row
    // of empty space, upgrade to SizeMode.Exact if that is ever reported.
    override val sizeMode: SizeMode = SizeMode.Responsive(SIZES)

    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val snapshot = loadWidgetSnapshot(context)
        if (snapshot.mode != WidgetMode.LoggedOut) {
            val at = nextRedraw(snapshot.day, snapshot.now, snapshot.zone, snapshot.syncedAtMs)
            // shortcut: WorkManager runs the one-off a little late (minutes in Doze), so a class change can show
            // that late; upgrade to AlarmManager.setAndAllowWhileIdle (no permission needed, still inexact) if it
            // matters. The 15 s slack makes the redraw land after the boundary, never before it.
            SyncScheduler.scheduleWidgetRefresh(context, at.toEpochMilli() + BOUNDARY_SLACK_MS)
        }
        provideContent {
            val colors = if (snapshot.dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                GlanceTheme.colors
            } else {
                ColorProviders(light = BrandLight, dark = BrandDark)
            }
            GlanceTheme(colors) { WidgetFrame(snapshot) }
        }
    }

    private companion object {
        const val BOUNDARY_SLACK_MS = 15_000L

        val SIZES = setOf(
            // Tiny (2x1), 160 dp wide and up shows the title
            DpSize(110.dp, 48.dp), DpSize(110.dp, 80.dp), DpSize(180.dp, 48.dp), DpSize(180.dp, 80.dp),
            // Strip (4x1)
            DpSize(250.dp, 60.dp),
            // Small (2x2) and Tall (2x4)
            DpSize(110.dp, 110.dp), DpSize(180.dp, 160.dp),
            DpSize(110.dp, 250.dp), DpSize(180.dp, 250.dp), DpSize(180.dp, 330.dp),
            // Medium (4x2) and Large (4x4): one, two, three, four, five and six rows
            DpSize(250.dp, 110.dp), DpSize(250.dp, 160.dp), DpSize(250.dp, 210.dp),
            DpSize(250.dp, 250.dp), DpSize(250.dp, 300.dp), DpSize(250.dp, 350.dp),
        )
    }
}

/** The 24 dp rounded surface. Below Android 12 `cornerRadius` does nothing, so a tinted shape stands in. */
@Composable
private fun WidgetFrame(snapshot: WidgetSnapshot) {
    val surface = GlanceTheme.colors.widgetBackground
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        Box(GlanceModifier.fillMaxSize().appWidgetBackground().background(surface).cornerRadius(24.dp).padding(12.dp)) {
            WidgetRoot(snapshot)
        }
    } else {
        Box(GlanceModifier.fillMaxSize().appWidgetBackground()) {
            Image(ImageProvider(R.drawable.widget_bg), null, GlanceModifier.fillMaxSize(), colorFilter = ColorFilter.tint(surface))
            Box(GlanceModifier.fillMaxSize().padding(12.dp)) { WidgetRoot(snapshot) }
        }
    }
}

/** The refresh button: a sync now; the redraw follows the sync. */
class RefreshAction : ActionCallback {
    override suspend fun onAction(context: Context, glanceId: GlanceId, parameters: ActionParameters) {
        SyncScheduler.syncNow(context)
    }
}
