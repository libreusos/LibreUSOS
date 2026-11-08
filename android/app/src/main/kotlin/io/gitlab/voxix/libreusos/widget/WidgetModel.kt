package io.gitlab.voxix.libreusos.widget

import org.libreusos.core.Activity
import java.time.DayOfWeek
import java.time.Instant
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.DateTimeParseException
import java.time.temporal.ChronoUnit
import java.util.Locale

/** An activity with its start and end parsed once. */
data class TimedActivity(val activity: Activity, val start: LocalDateTime, val end: LocalDateTime)

/** Activities with unparseable times, or ending before they start, are dropped; the rest is sorted by start, then end. */
fun timed(activities: List<Activity>): List<TimedActivity> = activities.mapNotNull { a ->
    try {
        val start = LocalDateTime.parse(a.start)
        val end = LocalDateTime.parse(a.end)
        if (end < start) null else TimedActivity(a, start, end)
    } catch (_: DateTimeParseException) {
        null
    }
}.sortedWith(compareBy({ it.start }, { it.end }))

/** What the widget says about the first class it shows: "pierwsze", "teraz" or "następne". */
enum class ClassPhase { First, Now, Next }

/** The first day after today that has classes. */
data class NextDay(val date: LocalDate, val items: List<TimedActivity>)

/** Which day state the widget is in (SYSTEM.md section 13.3). Holidays and cancellations need core data we do not have. */
sealed interface WidgetDay {
    /**
     * Classes remain today: [items] have not ended yet, the first is the one the phase describes.
     * [position] of [total] is the first class's place among today's classes, 0 when it began yesterday.
     */
    data class Active(
        val items: List<TimedActivity>,
        val phase: ClassPhase,
        val position: Int,
        val total: Int,
        val next: NextDay?,
    ) : WidgetDay

    /** Today had classes and they are over. */
    data class Over(val total: Int, val next: NextDay?) : WidgetDay

    /** No class today: a free weekday, or a Saturday or Sunday ([weekend]). */
    data class Free(val weekend: Boolean, val next: NextDay?) : WidgetDay
}

/**
 * The day state at [now]. A class that began before midnight stays until it ends. [activities] must be sorted
 * ([timed]).
 */
fun widgetDay(activities: List<TimedActivity>, now: LocalDateTime): WidgetDay {
    val today = now.toLocalDate()
    val todays = activities.filter { it.start.toLocalDate() == today }
    val remaining = activities.filter { it.start.toLocalDate() <= today && it.end > now }
    val nextDate = activities.filter { it.start.toLocalDate() > today }.minOfOrNull { it.start.toLocalDate() }
    val next = nextDate?.let { date -> NextDay(date, activities.filter { it.start.toLocalDate() == date }) }
    if (remaining.isNotEmpty()) {
        val first = remaining.first()
        val phase = when {
            first.start <= now -> ClassPhase.Now
            first == todays.firstOrNull() -> ClassPhase.First
            else -> ClassPhase.Next
        }
        return WidgetDay.Active(remaining, phase, todays.indexOf(first) + 1, todays.size, next)
    }
    if (todays.isNotEmpty()) return WidgetDay.Over(todays.size, next)
    return WidgetDay.Free(today.dayOfWeek == DayOfWeek.SATURDAY || today.dayOfWeek == DayOfWeek.SUNDAY, next)
}

enum class WidgetMode { LoggedOut, Loading, Content }

/**
 * What fills the widget. A session that expired with nothing cached has nothing to show but the way back in,
 * the same as logged out; one with a cache keeps it under a chip.
 */
fun widgetMode(hasToken: Boolean, hasCache: Boolean, expired: Boolean): WidgetMode = when {
    !hasToken || (expired && !hasCache) -> WidgetMode.LoggedOut
    !hasCache -> WidgetMode.Loading
    else -> WidgetMode.Content
}

/** How old the data is (SYSTEM.md section 13.3): nothing under 6 h, a neutral chip to 48 h, then an error chip. */
enum class StaleLevel { Fresh, Stale, VeryStale }

const val STALE_AFTER_MS = 6 * 3_600_000L
const val VERY_STALE_AFTER_MS = 48 * 3_600_000L

fun staleLevel(syncedAtMs: Long?, nowMs: Long): StaleLevel {
    val age = if (syncedAtMs == null) 0L else nowMs - syncedAtMs
    return when {
        age >= VERY_STALE_AFTER_MS -> StaleLevel.VeryStale
        age >= STALE_AFTER_MS -> StaleLevel.Stale
        else -> StaleLevel.Fresh
    }
}

/** The one chip the widget shows: an expired session wins over old data. */
enum class StatusChip { None, Offline, OfflineOld, Expired }

fun statusChip(expired: Boolean, level: StaleLevel): StatusChip = when {
    expired -> StatusChip.Expired
    level == StaleLevel.Stale -> StatusChip.Offline
    level == StaleLevel.VeryStale -> StatusChip.OfflineOld
    else -> StatusChip.None
}

/**
 * When the picture changes next: a class starts or ends, midnight, or the data turns old. Null never happens
 * (midnight always lies ahead).
 */
fun nextRedraw(day: WidgetDay?, now: LocalDateTime, zone: ZoneId, syncedAtMs: Long?): Instant {
    val midnight = now.toLocalDate().plusDays(1).atStartOfDay()
    val items = (day as? WidgetDay.Active)?.items.orEmpty()
    val classEdges = items.flatMap { listOf(it.start, it.end) }.filter { it > now && it < midnight }
    val local = (classEdges + midnight).min().atZone(zone).toInstant()
    val nowMs = now.atZone(zone).toInstant().toEpochMilli()
    val staleEdges = if (syncedAtMs == null) emptyList() else
        listOf(syncedAtMs + STALE_AFTER_MS, syncedAtMs + VERY_STALE_AFTER_MS).filter { it > nowMs }
    val staleEdge = staleEdges.minOrNull()?.let { Instant.ofEpochMilli(it) }
    return if (staleEdge != null && staleEdge < local) staleEdge else local
}

/** The size the layout was chosen for (SYSTEM.md section 13.1). */
enum class WidgetBucket { Tiny, Strip, Small, Tall, Medium, Large }

/** Width under 250 dp always picks the narrow column, so TALL and MEDIUM cannot overlap. */
fun bucketOf(widthDp: Float, heightDp: Float): WidgetBucket {
    val wide = widthDp >= 250f
    return when {
        heightDp >= 250f -> if (wide) WidgetBucket.Large else WidgetBucket.Tall
        heightDp >= 110f -> if (wide) WidgetBucket.Medium else WidgetBucket.Small
        else -> if (wide) WidgetBucket.Strip else WidgetBucket.Tiny
    }
}

/** Rows that fit under the 32 dp header in [heightDp], at 48 dp each, with the 12 dp padding. At least one. */
fun rowCapacity(heightDp: Float): Int = ((heightDp - 24f - 32f) / 48f).toInt().coerceAtLeast(1)

/** What a list-style widget draws: [shown] today's rows, a second day when room is left, and what overflowed. */
data class RowPlan(val todayShown: Int, val todayHidden: Int, val nextShown: Int, val nextHidden: Int)

/**
 * Fits [today] and [next] classes in [capacity] rows. A second day needs its own header row and at least one class;
 * what does not fit becomes "+N później" in the header of its day. Nothing scrolls.
 */
fun planRows(today: Int, next: Int, capacity: Int): RowPlan {
    val todayShown = minOf(today, capacity)
    val left = capacity - todayShown - 1
    val nextShown = if (today > todayShown || left < 1) 0 else minOf(next, left)
    return RowPlan(todayShown, today - todayShown, nextShown, if (nextShown == 0) 0 else next - nextShown)
}

/** Columns of the week motif are Monday to Friday; null for a weekend day. */
fun motifColumn(date: LocalDate): Int? = (date.dayOfWeek.value - 1).takeIf { it in 0..4 }

/** Rows of the motif are two-hour bands from 08:00 to 18:00 and later; earlier classes light the first. */
const val MOTIF_ROWS = 5

fun motifRow(start: LocalDateTime): Int = ((start.hour - 8) / 2).coerceIn(0, MOTIF_ROWS - 1)

/** Rows of the rest of the day under the focus class in the tall column (36 dp each, 150 dp above them, 24 dp padding). */
fun restCapacity(heightDp: Float): Int = ((heightDp - 24f - 150f) / 36f).toInt().coerceAtLeast(0)

/** "Wtorek, 10 lis": the weekday in full, the day and the short month, first letter capitalised. */
fun dayHeader(date: LocalDate, locale: Locale): String =
    DateTimeFormatter.ofPattern("EEEE, d MMM", locale).format(date).replaceFirstChar { it.titlecase(locale) }

/** "wt. 10 lis": the short form after "Jutro,". */
fun shortDay(date: LocalDate, locale: Locale): String = DateTimeFormatter.ofPattern("EEE d MMM", locale).format(date)

/** When the data is from: "22:14" today, "sob. 22:14" within the last week, else "7 lis 22:14". */
fun staleStamp(atMs: Long, zone: ZoneId, today: LocalDate, locale: Locale): String {
    val at = Instant.ofEpochMilli(atMs).atZone(zone)
    val days = ChronoUnit.DAYS.between(at.toLocalDate(), today)
    val pattern = when {
        days <= 0L -> "HH:mm"
        days < 7L -> "EEE HH:mm"
        else -> "d MMM HH:mm"
    }
    return DateTimeFormatter.ofPattern(pattern, locale).format(at)
}
