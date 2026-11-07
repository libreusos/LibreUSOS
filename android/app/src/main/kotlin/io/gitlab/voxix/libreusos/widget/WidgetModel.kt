package io.gitlab.voxix.libreusos.widget

import org.libreusos.core.Activity
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.format.DateTimeParseException

/** An activity with its start and end parsed once. */
data class TimedActivity(val activity: Activity, val start: LocalDateTime, val end: LocalDateTime)

/** The day the widget lists: [today], or the next day with classes. */
data class DayPlan(val date: LocalDate, val isToday: Boolean, val items: List<TimedActivity>)

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

/**
 * Classes that have not ended at [now] and started today or earlier (one crossing midnight stays); when none are left, all classes of the first later
 * day that has any. Null when the cache holds neither.
 */
fun pickDay(activities: List<TimedActivity>, now: LocalDateTime): DayPlan? {
    val today = now.toLocalDate()
    val remaining = activities.filter { it.start.toLocalDate() <= today && it.end > now }
    if (remaining.isNotEmpty()) return DayPlan(today, true, remaining)
    val next = activities.filter { it.start.toLocalDate() > today }.minOfOrNull { it.start.toLocalDate() }
        ?: return null
    return DayPlan(next, false, activities.filter { it.start.toLocalDate() == next })
}

/**
 * When the widget content changes next: the end of the first class that has not ended, or midnight.
 * (Classes of a later day only matter from that day's midnight on.)
 */
fun nextBoundary(plan: DayPlan?, now: LocalDateTime): LocalDateTime {
    val midnight = now.toLocalDate().plusDays(1).atStartOfDay()
    val nextEnd = plan?.takeIf { it.isToday }?.items?.map { it.end }?.filter { it > now }?.minOrNull()
    return if (nextEnd != null && nextEnd < midnight) nextEnd else midnight
}
