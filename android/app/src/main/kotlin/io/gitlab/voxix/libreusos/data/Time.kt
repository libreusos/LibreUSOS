package io.gitlab.voxix.libreusos.data

import org.libreusos.core.LangText
import java.time.Clock
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.LocalTime
import java.time.ZoneId

/** Today's date in the installation's zone (not the device's), as the core expects it. */
fun todayIn(zoneId: String, clock: Clock = Clock.systemUTC()): LocalDate =
    LocalDate.now(clock.withZone(ZoneId.of(zoneId)))

/** The text in [lang] ("pl" or "en"), else the other language, else empty. */
fun LangText.pick(lang: String): String {
    val (first, second) = if (lang == "pl") pl to en else en to pl
    return first?.takeIf { it.isNotBlank() } ?: second?.takeIf { it.isNotBlank() } ?: ""
}

/** The `HH:mm` of a local date-time `YYYY-MM-DDTHH:MM:SS`, or the input when it does not parse. */
fun clockTime(localDateTime: String): String = try {
    LocalDateTime.parse(localDateTime).toLocalTime().toString().take(5)
} catch (_: java.time.format.DateTimeParseException) {
    try {
        LocalTime.parse(localDateTime).toString().take(5)
    } catch (_: java.time.format.DateTimeParseException) {
        localDateTime
    }
}

/** Page 0 of the timetable pager. A Monday, far enough in the past for any realistic use. */
val TIMETABLE_ANCHOR: LocalDate = LocalDate.of(2000, 1, 3)

/** Two thousand weeks, about 38 years. */
const val TIMETABLE_PAGES = 7 * 2000

fun pageOf(date: LocalDate): Int =
    java.time.temporal.ChronoUnit.DAYS.between(TIMETABLE_ANCHOR, date).toInt().coerceIn(0, TIMETABLE_PAGES - 1)

fun dateOfPage(page: Int): LocalDate = TIMETABLE_ANCHOR.plusDays(page.toLong())

/** The Monday of the week that contains [date]. */
fun mondayOf(date: LocalDate): LocalDate =
    date.with(java.time.temporal.TemporalAdjusters.previousOrSame(java.time.DayOfWeek.MONDAY))

/**
 * A short building label, or null. USOS building ids like `BU/0016` are internal codes, so only
 * a short id made of letters and digits counts as meaningful.
 */
fun shortBuildingLabel(buildingId: String?): String? =
    buildingId?.trim()?.takeIf { it.length in 1..6 && it.all(Char::isLetterOrDigit) }

/** Live data older than this is refetched when the screen comes back. */
const val LIVE_MAX_AGE_MS = 15 * 60_000L

/** True when data last fetched live at [liveAtMs] (null: never, or not live any more) should be refetched. */
fun isStale(liveAtMs: Long?, nowMs: Long, maxAgeMs: Long = LIVE_MAX_AGE_MS): Boolean =
    liveAtMs == null || nowMs - liveAtMs >= maxAgeMs

/** When data was fetched, in [zone]: `HH:mm` for today, else `d MMM HH:mm` in [locale]. */
fun fetchedLabel(epochSec: Long, zone: ZoneId, locale: java.util.Locale, today: LocalDate = LocalDate.now(zone)): String {
    val at = java.time.Instant.ofEpochSecond(epochSec).atZone(zone)
    val pattern = if (at.toLocalDate() == today) "HH:mm" else "d MMM HH:mm"
    return java.time.format.DateTimeFormatter.ofPattern(pattern, locale).format(at)
}
