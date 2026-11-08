package io.gitlab.voxix.libreusos.widget

import android.content.Context
import android.content.Intent
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.ColorFilter
import androidx.glance.GlanceModifier
import androidx.glance.GlanceTheme
import androidx.glance.Image
import androidx.glance.ImageProvider
import androidx.glance.LocalContext
import androidx.glance.LocalSize
import androidx.glance.action.Action
import androidx.glance.action.clickable
import androidx.glance.appwidget.CircularProgressIndicator
import androidx.glance.appwidget.action.actionRunCallback
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.cornerRadius
import androidx.glance.background
import androidx.glance.layout.Alignment
import androidx.glance.layout.Box
import androidx.glance.layout.Column
import androidx.glance.layout.Row
import androidx.glance.layout.Spacer
import androidx.glance.layout.fillMaxHeight
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.fillMaxWidth
import androidx.glance.layout.height
import androidx.glance.layout.padding
import androidx.glance.layout.size
import androidx.glance.layout.width
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextAlign
import androidx.glance.text.TextStyle
import androidx.glance.unit.ColorProvider
import io.gitlab.voxix.libreusos.MainActivity
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.data.clockTime
import io.gitlab.voxix.libreusos.data.pick
import io.gitlab.voxix.libreusos.data.shortBuildingLabel
import io.gitlab.voxix.libreusos.ui.theme.KindCode
import io.gitlab.voxix.libreusos.ui.theme.KindGroup
import io.gitlab.voxix.libreusos.ui.theme.kindCodeOf
import org.libreusos.core.Activity
import org.libreusos.core.ActivityKind
import java.time.LocalDate
import java.util.Locale

/** Opens the app on the Timetable tab (see MainActivity). */
const val ACTION_OPEN_TIMETABLE = "io.gitlab.voxix.libreusos.action.OPEN_TIMETABLE"

private val ROW_HEIGHT = 48.dp
private val HEADER_HEIGHT = 40.dp

private val SYSTEM_FONT_TITLE = 14.sp
private val META = 12.sp

private fun openApp(context: Context): Action = actionStartActivity(Intent(context, MainActivity::class.java))

private fun openTimetable(context: Context): Action =
    actionStartActivity(Intent(context, MainActivity::class.java).setAction(ACTION_OPEN_TIMETABLE))

/** What a class-list widget shows: one day's classes, with the words that go with the first one. */
private data class Pane(
    val headerDate: LocalDate,
    val tomorrow: Boolean,
    val items: List<TimedActivity>,
    val now: Boolean,
    val eyebrow: String,
    val rowWord: String?,
    val focusTime: String,
    val counter: String?,
    val footnote: String?,
    val later: NextDay?,
)

private fun timeRange(item: TimedActivity) = "${clockTime(item.activity.start)}–${clockTime(item.activity.end)}"

private fun paneOf(context: Context, day: WidgetDay, today: LocalDate): Pane? = when (day) {
    is WidgetDay.Active -> {
        val first = day.items.first()
        val (eyebrow, word) = when (day.phase) {
            ClassPhase.Now -> R.string.widget_state_now to R.string.widget_row_now
            ClassPhase.Next -> R.string.widget_state_next to R.string.widget_row_next
            ClassPhase.First -> R.string.widget_state_first to R.string.widget_row_first
        }
        Pane(
            headerDate = today,
            tomorrow = false,
            items = day.items,
            now = day.phase == ClassPhase.Now,
            eyebrow = context.getString(eyebrow),
            rowWord = context.getString(word),
            focusTime = if (day.phase == ClassPhase.Now) {
                context.getString(R.string.widget_until, clockTime(first.activity.end))
            } else {
                timeRange(first)
            },
            counter = day.position.takeIf { it > 0 }?.let { context.getString(R.string.widget_position, it, day.total) },
            footnote = null,
            later = day.next,
        )
    }
    is WidgetDay.Over -> day.next?.let { next ->
        val tomorrow = next.date == today.plusDays(1)
        val locale = context.locale()
        Pane(
            headerDate = next.date,
            tomorrow = tomorrow,
            items = next.items,
            now = false,
            eyebrow = if (tomorrow) context.getString(R.string.widget_tomorrow) else shortDay(next.date, locale),
            rowWord = null,
            focusTime = timeRange(next.items.first()),
            counter = context.resources.getQuantityString(R.plurals.today_classes_count, next.items.size, next.items.size),
            footnote = context.getString(R.string.widget_over),
            later = null,
        )
    }
    is WidgetDay.Free -> null
}

private fun Context.locale(): Locale = resources.configuration.locales[0]

private fun titleOf(context: Context, a: Activity): String {
    val lang = context.locale().language
    return a.courseName.pick(lang).ifBlank { a.name.pick(lang) }.ifBlank { context.getString(R.string.home_unnamed) }
}

/** "s. 314 · AEI · gr. 3": room, short building and group, whichever exist. */
private fun placeOf(context: Context, a: Activity): String {
    val room = a.roomNumber?.takeIf { it.isNotBlank() }?.let { context.getString(R.string.today_room, it) }
    val group = a.groupNumber?.let { context.getString(R.string.today_group, it.toInt()) }
    return listOfNotNull(room, shortBuildingLabel(a.buildingId), group).joinToString(" · ")
}

private fun kindOf(a: Activity): KindCode =
    kindCodeOf(a.classtypeId, a.classtypeName.pl ?: a.classtypeName.en, a.kind == ActivityKind.Exam)

@Composable
internal fun WidgetRoot(s: WidgetSnapshot) {
    val context = LocalContext.current
    val size = LocalSize.current
    val bucket = bucketOf(size.width.value, size.height.value)
    val today = s.now.toLocalDate()
    val syncedAt = s.syncedAtMs
    val chip = statusChip(s.expired, staleLevel(syncedAt, s.now.atZone(s.zone).toInstant().toEpochMilli()))
    val stamp = syncedAt?.let { staleStamp(it, s.zone, today, context.locale()) }.orEmpty()
    when (s.mode) {
        WidgetMode.LoggedOut -> SignInView(bucket)
        WidgetMode.Loading -> LoadingView(bucket, size.height.value)
        WidgetMode.Content -> {
            val day = checkNotNull(s.day)
            val pane = paneOf(context, day, today)
            if (pane == null) {
                FreeView(bucket, day, today, chip, stamp, context.locale())
            } else {
                ClassesView(bucket, pane, size.height.value, chip, stamp)
            }
        }
    }
}

// --- Pieces ---

@Composable
private fun mark(tint: ColorProvider, size: Dp = 20.dp) {
    Image(ImageProvider(R.drawable.ic_launcher_monochrome), null, GlanceModifier.size(size), colorFilter = ColorFilter.tint(tint))
}

@Composable
private fun icon(res: Int, tint: ColorProvider, size: Dp = 16.dp) {
    Image(ImageProvider(res), null, GlanceModifier.size(size), colorFilter = ColorFilter.tint(tint))
}

/** The kind code in a fixed 32 dp box, so codes align like mono. Never colour alone: the code is the text. */
@Composable
private fun KindTag(kind: KindCode) {
    val c = GlanceTheme.colors
    val (fill, text) = when (kind.group) {
        KindGroup.Lecture -> c.primaryContainer to c.onPrimaryContainer
        KindGroup.Lab -> c.tertiaryContainer to c.onTertiaryContainer
        KindGroup.Exam -> c.primary to c.onPrimary
        KindGroup.Neutral -> c.surfaceVariant to c.onSurfaceVariant
    }
    Box(GlanceModifier.width(32.dp).height(18.dp).background(fill).cornerRadius(4.dp), contentAlignment = Alignment.Center) {
        Text(kind.code, style = TextStyle(color = text, fontSize = 10.sp, fontWeight = FontWeight.Medium, textAlign = TextAlign.Center), maxLines = 1)
    }
}

@Composable
private fun StatusChipView(chip: StatusChip, stamp: String) {
    val c = GlanceTheme.colors
    val context = LocalContext.current
    val (fill, text, glyph, label) = when (chip) {
        StatusChip.Offline -> Quad(c.secondaryContainer, c.onSecondaryContainer, R.drawable.ic_cloud_off, stamp)
        StatusChip.OfflineOld -> Quad(c.errorContainer, c.onErrorContainer, R.drawable.ic_cloud_off, stamp)
        StatusChip.Expired -> Quad(c.errorContainer, c.onErrorContainer, R.drawable.ic_lock_clock, context.getString(R.string.today_action_relogin))
        StatusChip.None -> return
    }
    Row(
        GlanceModifier.background(fill).cornerRadius(8.dp).padding(horizontal = 8.dp, vertical = 4.dp).clickable(openApp(context)),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        icon(glyph, text, 14.dp)
        Spacer(GlanceModifier.width(4.dp))
        Text(label, style = TextStyle(color = text, fontSize = 11.sp, fontWeight = FontWeight.Medium), maxLines = 1)
    }
}

private data class Quad(val fill: ColorProvider, val text: ColorProvider, val glyph: Int, val label: String)

@Composable
private fun RefreshButton() {
    val c = GlanceTheme.colors
    val context = LocalContext.current
    Box(GlanceModifier.size(HEADER_HEIGHT).clickable(actionRunCallback<RefreshAction>()), contentAlignment = Alignment.Center) {
        Box(GlanceModifier.size(32.dp).background(c.surfaceVariant).cornerRadius(16.dp), contentAlignment = Alignment.Center) {
            Image(
                ImageProvider(R.drawable.ic_refresh),
                context.getString(R.string.widget_refresh),
                GlanceModifier.size(18.dp),
                colorFilter = ColorFilter.tint(c.onSurfaceVariant),
            )
        }
    }
}

@Composable
private fun Header(title: String, hidden: Int, chip: StatusChip, stamp: String) {
    val c = GlanceTheme.colors
    val context = LocalContext.current
    Row(GlanceModifier.fillMaxWidth().height(HEADER_HEIGHT), verticalAlignment = Alignment.CenterVertically) {
        mark(c.primary)
        Spacer(GlanceModifier.width(8.dp))
        Text(
            title,
            GlanceModifier.defaultWeight().clickable(openTimetable(context)),
            style = TextStyle(color = c.onSurface, fontSize = SYSTEM_FONT_TITLE, fontWeight = FontWeight.Medium),
            maxLines = 1,
        )
        if (hidden > 0) {
            Text(
                context.getString(R.string.widget_more, hidden),
                GlanceModifier.padding(horizontal = 4.dp),
                style = TextStyle(color = c.onSurfaceVariant, fontSize = META),
                maxLines = 1,
            )
        }
        StatusChipView(chip, stamp)
        if (chip != StatusChip.Expired) RefreshButton()
    }
}

/** One class, 48 dp tall: time, kind, title and place, and the word for its state on the right. */
@Composable
private fun ClassRow(item: TimedActivity, word: String?, now: Boolean, prefix: String? = null) {
    val c = GlanceTheme.colors
    val context = LocalContext.current
    val a = item.activity
    val place = placeOf(context, a)
    val meta = listOfNotNull(prefix, place.takeIf { it.isNotEmpty() }).joinToString(" · ")
    Row(
        GlanceModifier.fillMaxWidth().height(ROW_HEIGHT).clickable(openTimetable(context)),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (now) {
            Box(GlanceModifier.width(3.dp).fillMaxHeight().background(c.primary)) {}
        } else {
            Spacer(GlanceModifier.width(3.dp))
        }
        Spacer(GlanceModifier.width(6.dp))
        Column(GlanceModifier.width(44.dp)) {
            Text(clockTime(a.start), style = TextStyle(color = c.onSurface, fontSize = SYSTEM_FONT_TITLE, fontWeight = FontWeight.Medium), maxLines = 1)
            Text(clockTime(a.end), style = TextStyle(color = c.onSurfaceVariant, fontSize = META), maxLines = 1)
        }
        KindTag(kindOf(a))
        Spacer(GlanceModifier.width(8.dp))
        Column(GlanceModifier.defaultWeight()) {
            Text(titleOf(context, a), style = TextStyle(color = c.onSurface, fontSize = SYSTEM_FONT_TITLE, fontWeight = FontWeight.Medium), maxLines = 1)
            if (meta.isNotEmpty()) Text(meta, style = TextStyle(color = c.onSurfaceVariant, fontSize = META), maxLines = 1)
        }
        if (word != null) {
            Text(
                word,
                GlanceModifier.padding(start = 6.dp),
                style = TextStyle(color = if (now) c.primary else c.onSurfaceVariant, fontSize = META, fontWeight = FontWeight.Medium),
                maxLines = 1,
            )
        }
    }
}

@Composable
private fun small(text: String, color: ColorProvider, size: androidx.compose.ui.unit.TextUnit = META, bold: Boolean = false) {
    Text(text, style = TextStyle(color = color, fontSize = size, fontWeight = if (bold) FontWeight.Medium else FontWeight.Normal), maxLines = 1)
}

// --- Class list sizes ---

@Composable
private fun ClassesView(bucket: WidgetBucket, pane: Pane, heightDp: Float, chip: StatusChip, stamp: String) {
    val context = LocalContext.current
    val locale = context.locale()
    val first = pane.items.first()
    val title = titleOf(context, first.activity)
    val place = placeOf(context, first.activity)
    val c = GlanceTheme.colors
    val then = pane.items.getOrNull(1)?.let {
        context.getString(R.string.widget_then, clockTime(it.activity.start), titleOf(context, it.activity))
    }
    when (bucket) {
        WidgetBucket.Tiny -> {
            val size = LocalSize.current
            Column(GlanceModifier.fillMaxSize(), verticalAlignment = Alignment.CenterVertically) {
                Row(GlanceModifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        pane.eyebrow.uppercase(locale),
                        GlanceModifier.defaultWeight(),
                        style = TextStyle(color = c.primary, fontSize = 11.sp, fontWeight = FontWeight.Medium),
                        maxLines = 1,
                    )
                    KindTag(kindOf(first.activity))
                }
                val wide = size.width >= 150.dp
                val tall = size.height >= 80.dp
                if (wide) small(title, c.onSurface, SYSTEM_FONT_TITLE, bold = true)
                if (tall || !wide) {
                    small(listOfNotNull(pane.focusTime, place.takeIf { it.isNotEmpty() }).joinToString(" · "), c.onSurfaceVariant)
                }
            }
        }
        WidgetBucket.Strip -> {
            Row(GlanceModifier.fillMaxSize(), verticalAlignment = Alignment.CenterVertically) {
                if (pane.now) {
                    Box(GlanceModifier.width(3.dp).fillMaxHeight().background(c.primary)) {}
                    Spacer(GlanceModifier.width(6.dp))
                }
                Column(GlanceModifier.width(44.dp)) {
                    small(clockTime(first.activity.start), c.onSurface, SYSTEM_FONT_TITLE, bold = true)
                    small(clockTime(first.activity.end), c.onSurfaceVariant)
                }
                KindTag(kindOf(first.activity))
                Spacer(GlanceModifier.width(8.dp))
                Column(GlanceModifier.defaultWeight()) {
                    small(title, c.onSurface, SYSTEM_FONT_TITLE, bold = true)
                    val meta = listOfNotNull(place.takeIf { it.isNotEmpty() }, pane.focusTime.takeIf { pane.now }, then).joinToString(" · ")
                    if (meta.isNotEmpty()) small(meta, c.onSurfaceVariant)
                }
                if (chip != StatusChip.None) {
                    Spacer(GlanceModifier.width(6.dp))
                    StatusChipView(chip, stamp)
                }
            }
        }
        WidgetBucket.Small, WidgetBucket.Tall -> {
            Column(GlanceModifier.fillMaxSize()) {
                Row(GlanceModifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        pane.eyebrow.uppercase(locale),
                        GlanceModifier.defaultWeight(),
                        style = TextStyle(color = c.primary, fontSize = 11.sp, fontWeight = FontWeight.Medium),
                        maxLines = 1,
                    )
                    KindTag(kindOf(first.activity))
                }
                Spacer(GlanceModifier.height(6.dp))
                Text(title, style = TextStyle(color = c.onSurface, fontSize = 16.sp, fontWeight = FontWeight.Medium), maxLines = 2)
                if (place.isNotEmpty()) small(place, c.onSurfaceVariant)
                Spacer(GlanceModifier.height(8.dp))
                Row(GlanceModifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        pane.focusTime,
                        GlanceModifier.defaultWeight(),
                        style = TextStyle(color = c.onSurface, fontSize = SYSTEM_FONT_TITLE, fontWeight = FontWeight.Medium),
                        maxLines = 1,
                    )
                    pane.counter?.let { small(it, c.onSurfaceVariant) }
                }
                if (bucket == WidgetBucket.Tall) {
                    val rest = pane.items.drop(1).take(restCapacity(heightDp))
                    Spacer(GlanceModifier.height(6.dp))
                    rest.forEach { RestRow(it) }
                    val hidden = pane.items.size - 1 - rest.size
                    if (hidden > 0) small(context.getString(R.string.widget_more, hidden), c.onSurfaceVariant)
                } else {
                    (pane.footnote ?: then)?.let {
                        Spacer(GlanceModifier.height(6.dp))
                        small(it, c.onSurfaceVariant)
                    }
                }
                if (bucket == WidgetBucket.Tall && pane.footnote != null) small(pane.footnote, c.onSurfaceVariant)
                if (chip != StatusChip.None) {
                    Spacer(GlanceModifier.defaultWeight())
                    StatusChipView(chip, stamp)
                }
            }
        }
        WidgetBucket.Medium, WidgetBucket.Large -> {
            val plan = planRows(pane.items.size, pane.later?.items?.size ?: 0, rowCapacity(heightDp))
            val headerTitle = if (pane.tomorrow) {
                context.getString(R.string.widget_tomorrow_header, shortDay(pane.headerDate, locale))
            } else {
                dayHeader(pane.headerDate, locale)
            }
            Column(GlanceModifier.fillMaxSize()) {
                Header(headerTitle, plan.todayHidden, chip, stamp)
                pane.items.take(plan.todayShown).forEachIndexed { i, item ->
                    ClassRow(item, pane.rowWord.takeIf { i == 0 }, pane.now && i == 0)
                }
                val later = pane.later
                if (later != null && plan.nextShown > 0) {
                    Row(GlanceModifier.fillMaxWidth().height(32.dp), verticalAlignment = Alignment.CenterVertically) {
                        small(
                            if (later.date == pane.headerDate.plusDays(1)) {
                                context.getString(R.string.widget_tomorrow_header, shortDay(later.date, locale))
                            } else {
                                dayHeader(later.date, locale)
                            },
                            c.onSurface, SYSTEM_FONT_TITLE, bold = true,
                        )
                        Spacer(GlanceModifier.defaultWeight())
                        if (plan.nextHidden > 0) small(context.getString(R.string.widget_more, plan.nextHidden), c.onSurfaceVariant)
                    }
                    later.items.take(plan.nextShown).forEach { ClassRow(it, null, false) }
                }
            }
        }
    }
}

/** Time and title of a later class in the narrow tall column. */
@Composable
private fun RestRow(item: TimedActivity) {
    val c = GlanceTheme.colors
    val context = LocalContext.current
    Row(GlanceModifier.fillMaxWidth().height(36.dp).clickable(openTimetable(context)), verticalAlignment = Alignment.CenterVertically) {
        Text(clockTime(item.activity.start), GlanceModifier.width(44.dp), style = TextStyle(color = c.onSurface, fontSize = SYSTEM_FONT_TITLE, fontWeight = FontWeight.Medium), maxLines = 1)
        Text(titleOf(context, item.activity), GlanceModifier.defaultWeight(), style = TextStyle(color = c.onSurface, fontSize = 13.sp), maxLines = 1)
    }
}

// --- No class today ---

/** The week motif, small: five columns, the next class's slot lit. */
@Composable
private fun Motif(next: NextDay?) {
    val c = GlanceTheme.colors
    val litCol = next?.let { motifColumn(it.date) }
    val litRow = next?.items?.firstOrNull()?.let { motifRow(it.start) }
    Row {
        for (col in 0 until 5) {
            Column(GlanceModifier.padding(end = 2.dp)) {
                for (row in 0 until MOTIF_ROWS) {
                    val lit = col == litCol && row == litRow
                    Box(
                        GlanceModifier.padding(bottom = 2.dp).size(10.dp),
                    ) {
                        Box(GlanceModifier.size(8.dp).background(if (lit) c.primary else c.surfaceVariant).cornerRadius(2.dp)) {}
                    }
                }
            }
        }
    }
}

@Composable
private fun FreeView(bucket: WidgetBucket, day: WidgetDay, today: LocalDate, chip: StatusChip, stamp: String, locale: Locale) {
    val c = GlanceTheme.colors
    val context = LocalContext.current
    val (title, next) = when (day) {
        is WidgetDay.Free -> context.getString(if (day.weekend) R.string.widget_weekend else R.string.today_empty_title) to day.next
        is WidgetDay.Over -> context.getString(R.string.widget_over) to day.next
        is WidgetDay.Active -> return
    }
    val first = next?.items?.firstOrNull()
    val nextLine = first?.let { "${shortDay(next.date, locale)} ${clockTime(it.activity.start)} · ${titleOf(context, it.activity)}" }
        ?: context.getString(R.string.widget_empty)
    when (bucket) {
        WidgetBucket.Tiny, WidgetBucket.Strip -> Column(GlanceModifier.fillMaxSize(), verticalAlignment = Alignment.CenterVertically) {
            small(title, c.onSurface, SYSTEM_FONT_TITLE, bold = true)
            small(nextLine, c.onSurfaceVariant)
        }
        WidgetBucket.Small, WidgetBucket.Tall -> Column(GlanceModifier.fillMaxSize()) {
            Motif(next)
            Spacer(GlanceModifier.height(8.dp))
            Text(title, style = TextStyle(color = c.onSurface, fontSize = 16.sp, fontWeight = FontWeight.Medium), maxLines = 2)
            Spacer(GlanceModifier.height(4.dp))
            Text(nextLine, style = TextStyle(color = c.onSurfaceVariant, fontSize = META), maxLines = 3)
            if (chip != StatusChip.None) {
                Spacer(GlanceModifier.defaultWeight())
                StatusChipView(chip, stamp)
            }
        }
        WidgetBucket.Medium, WidgetBucket.Large -> Column(GlanceModifier.fillMaxSize()) {
            Header(dayHeader(today, locale), 0, chip, stamp)
            Row(verticalAlignment = Alignment.CenterVertically) {
                Motif(next)
                Spacer(GlanceModifier.width(12.dp))
                Text(title, style = TextStyle(color = c.onSurface, fontSize = 18.sp, fontWeight = FontWeight.Medium), maxLines = 2)
            }
            Spacer(GlanceModifier.height(8.dp))
            if (first != null) {
                ClassRow(first, null, false, prefix = next?.let { shortDay(it.date, locale) })
            } else {
                small(nextLine, c.onSurfaceVariant)
            }
        }
    }
}

// --- Signed out and loading ---

@Composable
private fun SignInView(bucket: WidgetBucket) {
    val c = GlanceTheme.colors
    val context = LocalContext.current
    val open = openApp(context)
    when (bucket) {
        WidgetBucket.Tiny, WidgetBucket.Strip ->
            Row(GlanceModifier.fillMaxSize().clickable(open), verticalAlignment = Alignment.CenterVertically) {
                mark(c.primary)
                Spacer(GlanceModifier.width(8.dp))
                Text(context.getString(R.string.widget_sign_in_title), style = TextStyle(color = c.onSurface, fontSize = SYSTEM_FONT_TITLE, fontWeight = FontWeight.Medium), maxLines = 2)
            }
        WidgetBucket.Small, WidgetBucket.Tall ->
            Column(GlanceModifier.fillMaxSize().clickable(open), verticalAlignment = Alignment.CenterVertically) {
                mark(c.primary, 28.dp)
                Spacer(GlanceModifier.height(8.dp))
                Text(context.getString(R.string.widget_sign_in_title), style = TextStyle(color = c.onSurface, fontSize = SYSTEM_FONT_TITLE, fontWeight = FontWeight.Medium), maxLines = 2)
                Spacer(GlanceModifier.height(8.dp))
                SignInButton(open)
            }
        WidgetBucket.Medium, WidgetBucket.Large ->
            Row(GlanceModifier.fillMaxSize().clickable(open), verticalAlignment = Alignment.CenterVertically) {
                Box(GlanceModifier.size(56.dp).background(c.primaryContainer).cornerRadius(28.dp), contentAlignment = Alignment.Center) {
                    mark(c.onPrimaryContainer, 32.dp)
                }
                Spacer(GlanceModifier.width(16.dp))
                Column(GlanceModifier.defaultWeight()) {
                    Text(context.getString(R.string.widget_sign_in_title), style = TextStyle(color = c.onSurface, fontSize = 16.sp, fontWeight = FontWeight.Medium), maxLines = 1)
                    small(context.getString(R.string.widget_sign_in_body), c.onSurfaceVariant)
                    Spacer(GlanceModifier.height(8.dp))
                    SignInButton(open)
                }
            }
    }
}

@Composable
private fun SignInButton(open: Action) {
    val c = GlanceTheme.colors
    val context = LocalContext.current
    Box(
        GlanceModifier.height(36.dp).background(c.primary).cornerRadius(18.dp).padding(horizontal = 16.dp).clickable(open),
        contentAlignment = Alignment.Center,
    ) {
        Text(context.getString(R.string.widget_sign_in), style = TextStyle(color = c.onPrimary, fontSize = SYSTEM_FONT_TITLE, fontWeight = FontWeight.Medium), maxLines = 1)
    }
}

/** Bars where the rows will be, and the system spinner: shown until the first sync has stored a timetable. */
@Composable
private fun LoadingView(bucket: WidgetBucket, heightDp: Float) {
    val c = GlanceTheme.colors
    val context = LocalContext.current
    val rows = if (bucket == WidgetBucket.Tiny || bucket == WidgetBucket.Strip) 1 else rowCapacity(heightDp).coerceAtMost(3)
    Column(GlanceModifier.fillMaxSize().clickable(openApp(context))) {
        Row(GlanceModifier.fillMaxWidth().height(HEADER_HEIGHT), verticalAlignment = Alignment.CenterVertically) {
            mark(c.primary)
            Spacer(GlanceModifier.defaultWeight())
            CircularProgressIndicator(GlanceModifier.size(24.dp), color = c.primary)
        }
        repeat(rows) {
            Row(GlanceModifier.fillMaxWidth().height(ROW_HEIGHT), verticalAlignment = Alignment.CenterVertically) {
                Box(GlanceModifier.width(44.dp).height(14.dp).background(c.surfaceVariant).cornerRadius(4.dp)) {}
                Spacer(GlanceModifier.width(8.dp))
                Box(GlanceModifier.defaultWeight().height(14.dp).background(c.surfaceVariant).cornerRadius(4.dp)) {}
            }
        }
    }
}
