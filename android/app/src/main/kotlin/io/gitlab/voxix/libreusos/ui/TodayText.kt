package io.gitlab.voxix.libreusos.ui

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.data.clockTime
import io.gitlab.voxix.libreusos.data.pick
import org.libreusos.core.Activity
import org.libreusos.core.Frequency
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

/** "45 min", "2 h", "1 h 20 min". */
@Composable
fun spanText(totalMinutes: Int): String {
    val span = spanOf(totalMinutes)
    return when {
        span.hours == 0 -> stringResource(R.string.today_span_min, span.minutes)
        span.minutes == 0 -> stringResource(R.string.today_span_h, span.hours)
        else -> stringResource(R.string.today_span_h_min, span.hours, span.minutes)
    }
}

/** The one stamp format: "dziś, 10:52", "wczoraj, 22:14", "pn. 9 lis, 22:14". */
@Composable
fun stampText(atSec: Long, zone: ZoneId, today: LocalDate, locale: Locale): String {
    val clock = clockOf(atSec, zone)
    return when (stampDay(atSec, zone, today)) {
        StampDay.Today -> stringResource(R.string.today_stamp_today, clock)
        StampDay.Yesterday -> stringResource(R.string.today_stamp_yesterday, clock)
        StampDay.Other ->
            DateTimeFormatter.ofPattern("EEE d MMM, HH:mm", locale).format(Instant.ofEpochSecond(atSec).atZone(zone))
    }
}

fun clockOf(atSec: Long, zone: ZoneId): String =
    DateTimeFormatter.ofPattern("HH:mm").format(Instant.ofEpochSecond(atSec).atZone(zone))

/** The course name, else the activity name, else a placeholder. */
@Composable
fun activityTitle(activity: Activity, lang: String): String =
    activity.courseName.pick(lang).ifBlank { activity.name.pick(lang) }.ifBlank { stringResource(R.string.home_unnamed) }

/** "s. 215 · BU · gr. 2": room, short building and group, whichever exist. */
@Composable
fun placeLine(activity: Activity, includeGroup: Boolean = true): String {
    val room = activity.roomNumber?.takeIf { it.isNotBlank() }?.let { stringResource(R.string.today_room, it) }
    val building = io.gitlab.voxix.libreusos.data.shortBuildingLabel(activity.buildingId)
    val group = if (includeGroup) activity.groupNumber?.let { stringResource(R.string.today_group, it.toInt()) } else null
    return listOfNotNull(room, building, group).joinToString(" · ")
}

@Composable
fun classesCount(n: Int): String = pluralStringResource(R.plurals.today_classes_count, n, n)

/** The chip text for how often a class meets; null for every week, which is the usual case. */
@Composable
fun frequencyLabel(frequency: Frequency): String? = when (frequency) {
    Frequency.EveryWeek -> null
    Frequency.EveryFortnight -> stringResource(R.string.frequency_fortnight)
    Frequency.FortnightOdd -> stringResource(R.string.frequency_fortnight_odd)
    Frequency.FortnightEven -> stringResource(R.string.frequency_fortnight_even)
    Frequency.EveryMonth -> stringResource(R.string.frequency_month)
    Frequency.EveryWorkingDay -> stringResource(R.string.frequency_working_day)
    Frequency.EveryWeekend -> stringResource(R.string.frequency_weekend)
    Frequency.EveryDay -> stringResource(R.string.frequency_day)
    Frequency.Once -> stringResource(R.string.frequency_once)
    Frequency.Irregular -> stringResource(R.string.frequency_irregular)
    is Frequency.Other -> frequency.raw.takeIf { it.isNotBlank() }
}

/** One spoken sentence for a class node: title, type, time range, room, building and a clash note. */
@Composable
fun classDescription(activity: Activity, lang: String, clash: Boolean): String {
    val room = activity.roomNumber?.takeIf { it.isNotBlank() }?.let { stringResource(R.string.plan_a11y_room, it) }
    val building = io.gitlab.voxix.libreusos.data.shortBuildingLabel(activity.buildingId)
        ?.let { stringResource(R.string.plan_a11y_building, it) }
    return listOfNotNull(
        activityTitle(activity, lang),
        activity.classtypeName.pick(lang).takeIf { it.isNotBlank() },
        stringResource(R.string.plan_a11y_time, clockTime(activity.start), clockTime(activity.end)),
        room,
        building,
        if (clash) stringResource(R.string.plan_a11y_clash) else null,
    ).joinToString(", ")
}
