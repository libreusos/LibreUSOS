package io.gitlab.voxix.libreusos.ui

import io.gitlab.voxix.libreusos.widget.TimedActivity
import io.gitlab.voxix.libreusos.widget.timed
import org.libreusos.core.Activity
import java.time.Duration
import java.time.LocalDate
import java.time.LocalDateTime

/** Where a class stands against "now". */
enum class RowState { Past, Now, Upcoming }

/**
 * One class of the day plan. [gapBeforeMin] is the free time since the previous class, when it is worth
 * saying. [clash] is the class's place in a run of overlapping classes.
 */
data class TimelineRow(val item: TimedActivity, val state: RowState, val gapBeforeMin: Int?, val clash: ClashPart = ClashPart.None)

/**
 * The now card: the class in progress ([current]) or else the next one. [minutes] is the time left, or until
 * it starts. [progress] is 0 to 1 and only means something for a current class. [nextIndex] is the class after it.
 */
data class NowCardInfo(val index: Int, val current: Boolean, val minutes: Int, val progress: Float, val nextIndex: Int?)

data class TodayPlan(val rows: List<TimelineRow>, val card: NowCardInfo?)

/**
 * A free stretch shorter than this is the usual break between periods, not an "okienko"
 * (SYSTEM.md section 14: "Przerwa" is the 15 min between periods).
 */
const val MIN_GAP_MIN = 20

/** Whole minutes in [d], rounded up, so "do końca 1 min" lasts until the very end. */
fun ceilMinutes(d: Duration): Int = ((d.seconds + 59) / 60).toInt().coerceAtLeast(0)

/** A time span as the screens print it: hours and minutes, never both zero. */
data class Span(val hours: Int, val minutes: Int)

fun spanOf(totalMinutes: Int): Span = Span(totalMinutes / 60, totalMinutes % 60).let {
    if (it.hours == 0 && it.minutes == 0) Span(0, 1) else it
}

/** The classes of [today] with their state at [now], the gaps between them and the card's class. */
fun todayPlan(activities: List<Activity>, today: LocalDate, now: LocalDateTime): TodayPlan {
    val items = timed(activities).filter { it.start.toLocalDate() == today }
    var latestEnd: LocalDateTime? = null
    val clash = clashParts(items)
    val rows = items.mapIndexed { i, item ->
        val state = when {
            item.end <= now -> RowState.Past
            item.start <= now -> RowState.Now
            else -> RowState.Upcoming
        }
        val gap = latestEnd?.let { ceilMinutes(Duration.between(it, item.start)) }?.takeIf { it >= MIN_GAP_MIN }
        if (latestEnd == null || item.end > latestEnd!!) latestEnd = item.end
        TimelineRow(item, state, gap, clash[i])
    }
    val cardIndex = rows.indexOfFirst { it.state == RowState.Now }.takeIf { it >= 0 }
        ?: rows.indexOfFirst { it.state == RowState.Upcoming }.takeIf { it >= 0 }
    if (cardIndex == null) return TodayPlan(rows, null)
    val row = rows[cardIndex]
    val current = row.state == RowState.Now
    val total = Duration.between(row.item.start, row.item.end).seconds.coerceAtLeast(1)
    val card = NowCardInfo(
        index = cardIndex,
        current = current,
        minutes = ceilMinutes(if (current) Duration.between(now, row.item.end) else Duration.between(now, row.item.start)),
        progress = if (current) (Duration.between(row.item.start, now).seconds.toFloat() / total).coerceIn(0f, 1f) else 0f,
        nextIndex = rows.indices.firstOrNull { it > cardIndex && rows[it].state == RowState.Upcoming },
    )
    return TodayPlan(rows, card)
}

/** The first later day with classes, for a day without any. */
data class NextClasses(val date: LocalDate, val items: List<TimedActivity>)

fun nextClasses(activities: List<Activity>, today: LocalDate): NextClasses? {
    val later = timed(activities).filter { it.start.toLocalDate() > today }
    val date = later.minOfOrNull { it.start.toLocalDate() } ?: return null
    return NextClasses(date, later.filter { it.start.toLocalDate() == date })
}

/** How many next classes are listed before the rest folds into one "Jeszcze N zajęć" row. */
const val NEXT_CLASSES_SHOWN = 3

/** The classes listed for [next], and how many more there are. */
fun splitNextClasses(items: List<TimedActivity>): Pair<List<TimedActivity>, Int> =
    if (items.size <= NEXT_CLASSES_SHOWN) items to 0 else items.take(NEXT_CLASSES_SHOWN - 1) to items.size - (NEXT_CLASSES_SHOWN - 1)
