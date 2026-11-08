package io.gitlab.voxix.libreusos.ui

import io.gitlab.voxix.libreusos.data.mondayOf
import io.gitlab.voxix.libreusos.widget.TimedActivity
import io.gitlab.voxix.libreusos.widget.timed
import org.libreusos.core.Activity
import java.time.Duration
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter
import java.util.Locale

/**
 * The classes of one day split into runs of overlapping ones, singletons included. [items] must be
 * sorted by start. Back-to-back classes (one ends exactly when the next starts) do not overlap, as in the core.
 */
fun overlapClusters(items: List<TimedActivity>): List<List<TimedActivity>> {
    val clusters = mutableListOf<MutableList<TimedActivity>>()
    var end: LocalDateTime? = null
    for (item in items) {
        if (end != null && item.start < end) clusters.last() += item else clusters += mutableListOf(item)
        end = if (end == null || item.start >= end || item.end > end) item.end else end
    }
    return clusters
}

/** Where a class sits in a run of overlapping classes; the 3 dp error bar spans the run. */
enum class ClashPart(val barAbove: Boolean, val barBelow: Boolean) {
    None(false, false),
    First(false, true),
    Middle(true, true),
    Last(true, false),
}

/** One [ClashPart] per item, in the order of [items] (sorted by start). */
fun clashParts(items: List<TimedActivity>): List<ClashPart> = overlapClusters(items).flatMap { cluster ->
    cluster.indices.map { i ->
        when {
            cluster.size == 1 -> ClashPart.None
            i == 0 -> ClashPart.First
            i == cluster.lastIndex -> ClashPart.Last
            else -> ClashPart.Middle
        }
    }
}

/**
 * Two or more classes that overlap. [from] to [to] is what the banner names: the overlap of a pair
 * ("12:45–13:30", 45 min), or the whole stretch for three or more.
 */
data class ClashGroup(val items: List<TimedActivity>) {
    val from: LocalDateTime =
        if (items.size == 2) maxOf(items[0].start, items[1].start) else items.minOf { it.start }
    val to: LocalDateTime =
        if (items.size == 2) minOf(items[0].end, items[1].end) else items.maxOf { it.end }
    val minutes: Int = ceilMinutes(Duration.between(from, to))
}

fun clashGroups(items: List<TimedActivity>): List<ClashGroup> =
    overlapClusters(items).filter { it.size > 1 }.map(::ClashGroup)

/** How many timed classes start on each date; the week strip draws up to three dots from it. */
fun dayCounts(activities: List<Activity>): Map<LocalDate, Int> =
    timed(activities).groupingBy { it.start.toLocalDate() }.eachCount()

/** The hours the week grid draws: whole hours, [start] inclusive, [end] the last line. */
data class HourRange(val start: Int, val end: Int)

private const val DAY_MIN = 24 * 60

private fun minuteOfDay(t: LocalDateTime) = t.hour * 60 + t.minute

/** Minutes from midnight of the class's start day; a class running past midnight ends at 24:00 there. */
private fun endMinute(item: TimedActivity) =
    if (item.end.toLocalDate() > item.start.toLocalDate()) DAY_MIN else minuteOfDay(item.end)

/**
 * Fitted to the data (SYSTEM.md section 10): from the earlier of 08:00 and the first start, to the
 * later of 16:00 and the last end rounded up, so it is never narrower than 8 hours.
 */
fun hourRange(items: List<TimedActivity>): HourRange {
    val first = items.minOfOrNull { minuteOfDay(it.start) / 60 } ?: 8
    val last = items.maxOfOrNull { (endMinute(it) + 59) / 60 } ?: 16
    return HourRange(minOf(8, first), maxOf(16, last))
}

/** A class in the week grid: [column] of [columns] side by side in its cell. Minutes are from midnight. */
data class GridBlock(val item: TimedActivity, val column: Int, val columns: Int, val startMin: Int, val endMin: Int)

/** A run of overlapping classes (or one class); more than one block means a clash. */
data class GridCell(val blocks: List<GridBlock>) {
    val startMin: Int = blocks.minOf { it.startMin }
    val endMin: Int = blocks.maxOf { it.endMin }
    val isClash: Boolean get() = blocks.size > 1
}

/** The cells of one day: each run of overlapping classes shares the day column, split into sub-columns. */
fun dayCells(items: List<TimedActivity>): List<GridCell> = overlapClusters(items).map { cluster ->
    val columnEnds = mutableListOf<LocalDateTime>()
    val placed = cluster.map { item ->
        val free = columnEnds.indexOfFirst { it <= item.start }
        val column = if (free >= 0) free else columnEnds.size.also { columnEnds += item.end }
        if (free >= 0) columnEnds[free] = item.end
        column to item
    }
    val columns = columnEnds.size
    GridCell(placed.map { (column, item) -> GridBlock(item, column, columns, minuteOfDay(item.start), endMinute(item)) })
}

data class GridDay(val date: LocalDate, val cells: List<GridCell>)

/** [days] is Monday to Friday, or to Sunday when the week has weekend classes. */
data class WeekGrid(val range: HourRange, val days: List<GridDay>)

fun weekGrid(activities: List<Activity>, monday: LocalDate): WeekGrid {
    val sunday = monday.plusDays(6)
    val items = timed(activities).filter { it.start.toLocalDate() in monday..sunday }
    val dayCount = if (items.any { it.start.toLocalDate() > monday.plusDays(4) }) 7 else 5
    val days = (0 until dayCount).map { offset ->
        val date = monday.plusDays(offset.toLong())
        GridDay(date, dayCells(items.filter { it.start.toLocalDate() == date }))
    }
    return WeekGrid(hourRange(items), days)
}

/** "9–15 listopada", or "30 lis – 6 gru" when the week spans two months. */
fun weekRangeLabel(monday: LocalDate, locale: Locale): String {
    val sunday = monday.plusDays(6)
    return if (monday.month == sunday.month) {
        "${monday.dayOfMonth}–" + DateTimeFormatter.ofPattern("d MMMM", locale).format(sunday)
    } else {
        val short = DateTimeFormatter.ofPattern("d MMM", locale)
        short.format(monday) + " – " + short.format(sunday)
    }
}

/** The other meetings of a class's group, from the cached timetable. [position] is 1-based among all of them. */
data class GroupMeetings(val total: Int, val position: Int, val upcoming: List<Activity>)

/** The meetings of [current]'s course, class type and group in [cached] (which may or may not hold [current] itself). */
fun groupMeetings(current: Activity, cached: List<Activity>): GroupMeetings {
    val same = if (current.courseId == null) {
        emptyList()
    } else {
        cached.filter {
            it.courseId == current.courseId && it.classtypeId == current.classtypeId && it.groupNumber == current.groupNumber
        }
    }
    val all = (same.filter { it.start != current.start } + current).sortedBy { it.start }
    val position = all.indexOfFirst { it.start == current.start } + 1
    return GroupMeetings(all.size, position, all.drop(position))
}

/** "08:30". */
fun hm(t: LocalDateTime): String = "%02d:%02d".format(t.hour, t.minute)

/**
 * The number of the week of [day] in a term that runs from [start] to [end], counting the week with
 * the first day as week 1 (weeks run Monday to Sunday). Null when [day] is outside the term.
 */
fun termWeek(start: LocalDate, end: LocalDate, day: LocalDate): Int? =
    if (day < start || day > end) null else (java.time.temporal.ChronoUnit.WEEKS.between(mondayOf(start), mondayOf(day)) + 1).toInt()
