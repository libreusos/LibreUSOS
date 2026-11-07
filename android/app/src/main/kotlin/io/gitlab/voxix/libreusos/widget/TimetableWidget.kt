package io.gitlab.voxix.libreusos.widget

import android.content.Context
import android.content.Intent
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.GlanceTheme
import androidx.glance.LocalContext
import androidx.glance.LocalSize
import androidx.glance.action.Action
import androidx.glance.action.clickable
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.SizeMode
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.provideContent
import androidx.glance.background
import androidx.glance.layout.Alignment
import androidx.glance.layout.Column
import androidx.glance.layout.Row
import androidx.glance.layout.Spacer
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.fillMaxWidth
import androidx.glance.layout.height
import androidx.glance.layout.padding
import androidx.glance.layout.width
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextAlign
import androidx.glance.text.TextStyle
import io.gitlab.voxix.libreusos.MainActivity
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.data.clockTime
import io.gitlab.voxix.libreusos.data.pick
import io.gitlab.voxix.libreusos.sync.SyncScheduler
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

/** "Today / Next": today's remaining classes, then the next day with classes. Reads the core cache only. */
class TimetableWidget : GlanceAppWidget() {
    // Exact: the number of rows and the columns follow the size the user gave the widget.
    override val sizeMode: SizeMode = SizeMode.Exact

    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val snapshot = loadWidgetSnapshot(context)
        // One inexact one-off at the next change (a class ending, or midnight); each update sets the next one.
        val plan = (snapshot.content as? WidgetContent.Timetable)?.plan
        if (snapshot.content != WidgetContent.LoggedOut) {
            val boundary = nextBoundary(plan, snapshot.now).atZone(snapshot.zone).toInstant()
            SyncScheduler.scheduleWidgetRefresh(context, boundary.toEpochMilli() + BOUNDARY_SLACK_MS)
        }
        provideContent { GlanceTheme { WidgetBody(snapshot) } }
    }

    private companion object {
        const val BOUNDARY_SLACK_MS = 15_000L
    }
}

/** Opens the app on the Timetable tab (see MainActivity). */
const val ACTION_OPEN_TIMETABLE = "io.gitlab.voxix.libreusos.action.OPEN_TIMETABLE"

private val COMPACT_HEIGHT = 150.dp
private val WIDE_WIDTH = 200.dp
private val PADDING = 12.dp

private fun openApp(context: Context): Action = actionStartActivity(Intent(context, MainActivity::class.java))

private fun openTimetable(context: Context): Action =
    actionStartActivity(Intent(context, MainActivity::class.java).setAction(ACTION_OPEN_TIMETABLE))

@Composable
private fun WidgetBody(snapshot: WidgetSnapshot) {
    val context = LocalContext.current
    val root = GlanceModifier.fillMaxSize()
        .background(GlanceTheme.colors.widgetBackground)
        .cornerRadius(24.dp)
        .padding(PADDING)
    when (val content = snapshot.content) {
        WidgetContent.LoggedOut -> Message(root, context.getString(R.string.widget_logged_out), openApp(context))
        WidgetContent.NoData -> Message(root, context.getString(R.string.widget_no_data), openApp(context))
        is WidgetContent.Timetable ->
            if (content.plan == null) {
                Message(root, context.getString(R.string.widget_empty), openApp(context), cached(context, snapshot, content))
            } else {
                Classes(root, snapshot, content, content.plan)
            }
    }
}

@Composable
private fun Message(modifier: GlanceModifier, text: String, action: Action, footer: String? = null) {
    Column(modifier.clickable(action), verticalAlignment = Alignment.CenterVertically) {
        Text(
            text,
            GlanceModifier.fillMaxWidth(),
            style = TextStyle(color = GlanceTheme.colors.onSurface, fontSize = 14.sp, textAlign = TextAlign.Center),
            maxLines = 4,
        )
        if (footer != null) Footer(footer)
    }
}

@Composable
private fun Footer(text: String) {
    Text(
        text,
        GlanceModifier.fillMaxWidth().padding(top = 4.dp),
        style = TextStyle(color = GlanceTheme.colors.onSurfaceVariant, fontSize = 10.sp, textAlign = TextAlign.Center),
        maxLines = 1,
    )
}

@Composable
private fun Classes(modifier: GlanceModifier, snapshot: WidgetSnapshot, content: WidgetContent.Timetable, plan: DayPlan) {
    val context = LocalContext.current
    val size = LocalSize.current
    val lang = context.resources.configuration.locales[0].language
    val compact = size.height < COMPACT_HEIGHT
    val wide = size.width >= WIDE_WIDTH
    // Fixed row heights (sp-free approximations) decide how many rows fit under the header.
    val rowHeight = if (compact) 22.dp else 42.dp
    val room = size.height - PADDING * 2 - HEADER_HEIGHT
    val fit = (room / rowHeight).toInt().coerceAtLeast(1)
    val shown = if (plan.items.size > fit) (fit - 1).coerceAtLeast(1) else plan.items.size
    val hidden = plan.items.size - shown
    val action = openTimetable(context)

    Column(modifier) {
        Row(GlanceModifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(
                dayTitle(context, plan, snapshot.zone),
                GlanceModifier.defaultWeight(),
                style = TextStyle(color = GlanceTheme.colors.primary, fontSize = 13.sp, fontWeight = FontWeight.Bold),
                maxLines = 1,
            )
            val stamp = cached(context, snapshot, content)
            if (stamp != null && wide) {
                Text(stamp, style = TextStyle(color = GlanceTheme.colors.onSurfaceVariant, fontSize = 10.sp), maxLines = 1)
            }
        }
        Spacer(GlanceModifier.height(4.dp))
        plan.items.take(shown).forEach { item ->
            ClassRow(item, lang, compact, wide, GlanceModifier.fillMaxWidth().clickable(action))
        }
        if (hidden > 0) {
            Text(
                context.getString(R.string.widget_more, hidden),
                GlanceModifier.padding(top = 2.dp),
                style = TextStyle(color = GlanceTheme.colors.onSurfaceVariant, fontSize = 11.sp),
                maxLines = 1,
            )
        }
        if (!wide) cached(context, snapshot, content)?.let { Footer(it) }
    }
}

private val HEADER_HEIGHT = 22.dp

@Composable
private fun ClassRow(item: TimedActivity, lang: String, compact: Boolean, wide: Boolean, modifier: GlanceModifier) {
    val context = LocalContext.current
    val a = item.activity
    val title = a.courseName.pick(lang).ifBlank { a.name.pick(lang) }.ifBlank { context.getString(R.string.home_unnamed) }
    val room = a.roomNumber?.takeIf { it.isNotBlank() }
    val onSurface = GlanceTheme.colors.onSurface
    val muted = GlanceTheme.colors.onSurfaceVariant
    val time = clockTime(a.start)
    if (compact) {
        Row(modifier.padding(vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(time, GlanceModifier.width(40.dp), style = TextStyle(color = GlanceTheme.colors.primary, fontSize = 12.sp, fontWeight = FontWeight.Medium), maxLines = 1)
            Text(title, GlanceModifier.defaultWeight(), style = TextStyle(color = onSurface, fontSize = 12.sp), maxLines = 1)
            if (wide && room != null) {
                Text(room, GlanceModifier.padding(start = 6.dp), style = TextStyle(color = muted, fontSize = 11.sp), maxLines = 1)
            }
        }
    } else {
        Row(modifier.padding(vertical = 3.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(GlanceModifier.width(44.dp)) {
                Text(time, style = TextStyle(color = GlanceTheme.colors.primary, fontSize = 13.sp, fontWeight = FontWeight.Medium), maxLines = 1)
                Text(clockTime(a.end), style = TextStyle(color = muted, fontSize = 11.sp), maxLines = 1)
            }
            Column(GlanceModifier.defaultWeight()) {
                Text(title, style = TextStyle(color = onSurface, fontSize = 13.sp), maxLines = 1)
                if (room != null) {
                    Text(context.getString(R.string.activity_room, room), style = TextStyle(color = muted, fontSize = 11.sp), maxLines = 1)
                }
            }
        }
    }
}

private fun dayTitle(context: Context, plan: DayPlan, zone: ZoneId): String {
    if (plan.isToday) return context.getString(R.string.widget_today)
    val today = LocalDate.now(zone)
    if (plan.date == today.plusDays(1)) return context.getString(R.string.widget_tomorrow)
    val locale = context.resources.configuration.locales[0]
    return DateTimeFormatter.ofPattern("EEE d MMM", locale).format(plan.date)
}

/** "cached HH:mm" in the installation zone; with the day when it is not today. Null when unknown. */
private fun cached(context: Context, snapshot: WidgetSnapshot, content: WidgetContent.Timetable): String? {
    val at = content.fetchedAtSec?.let { Instant.ofEpochSecond(it).atZone(snapshot.zone) } ?: return null
    val locale: Locale = context.resources.configuration.locales[0]
    val pattern = if (at.toLocalDate() == snapshot.now.toLocalDate()) "HH:mm" else "d MMM HH:mm"
    return context.getString(R.string.widget_cached, DateTimeFormatter.ofPattern(pattern, locale).format(at))
}
