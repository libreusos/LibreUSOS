package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.sync.SyncOutcome
import io.gitlab.voxix.libreusos.sync.outcomeOf
import io.gitlab.voxix.libreusos.widget.ClassPhase
import io.gitlab.voxix.libreusos.widget.RowPlan
import io.gitlab.voxix.libreusos.widget.STALE_AFTER_MS
import io.gitlab.voxix.libreusos.widget.StaleLevel
import io.gitlab.voxix.libreusos.widget.StatusChip
import io.gitlab.voxix.libreusos.widget.VERY_STALE_AFTER_MS
import io.gitlab.voxix.libreusos.widget.WidgetBucket
import io.gitlab.voxix.libreusos.widget.WidgetDay
import io.gitlab.voxix.libreusos.widget.WidgetMode
import io.gitlab.voxix.libreusos.widget.bucketOf
import io.gitlab.voxix.libreusos.widget.motifColumn
import io.gitlab.voxix.libreusos.widget.motifRow
import io.gitlab.voxix.libreusos.widget.nextRedraw
import io.gitlab.voxix.libreusos.widget.planRows
import io.gitlab.voxix.libreusos.widget.rowCapacity
import io.gitlab.voxix.libreusos.widget.staleLevel
import io.gitlab.voxix.libreusos.widget.staleStamp
import io.gitlab.voxix.libreusos.widget.statusChip
import io.gitlab.voxix.libreusos.widget.timed
import io.gitlab.voxix.libreusos.widget.widgetDay
import io.gitlab.voxix.libreusos.widget.widgetMode
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.libreusos.core.Activity
import org.libreusos.core.ActivityKind
import org.libreusos.core.LangText
import org.libreusos.core.UsosException
import java.time.Duration
import java.time.Instant
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.ZoneId
import java.util.Locale

class WidgetModelTest {
    private fun activity(start: String, end: String) = Activity(
        kind = ActivityKind.Classgroup,
        start = start,
        end = end,
        date = start.take(10),
        name = LangText(null, null),
        courseId = null,
        courseName = LangText("Course", null),
        classtypeId = null,
        classtypeName = LangText(null, null),
        groupNumber = null,
        buildingId = null,
        buildingName = LangText(null, null),
        roomId = null,
        roomNumber = null,
        lecturerIds = emptyList(),
        unitId = null,
        frequency = null,
        url = null,
        clashes = false,
    )

    // Thu 5 Nov 2026: two classes; Fri 6: one; Mon 9: one; one broken row.
    private val week = timed(
        listOf(
            activity("2026-11-05T11:30:00", "2026-11-05T13:00:00"),
            activity("2026-11-05T08:00:00", "2026-11-05T09:30:00"),
            activity("2026-11-06T10:00:00", "2026-11-06T11:30:00"),
            activity("2026-11-09T10:00:00", "2026-11-09T11:30:00"),
            activity("broken", "broken"),
        ),
    )

    private fun at(text: String) = LocalDateTime.parse(text)
    private val warsaw = ZoneId.of("Europe/Warsaw")

    @Test fun unparseableTimesAreDroppedAndTheRestIsSorted() {
        assertEquals(4, week.size)
        assertEquals(at("2026-11-05T08:00:00"), week.first().start)
        assertTrue(timed(listOf(activity("2026-11-05T10:00:00", "2026-11-05T09:00:00"))).isEmpty())
    }

    @Test fun beforeTheFirstClassItIsFirst() {
        val d = widgetDay(week, at("2026-11-05T07:00:00")) as WidgetDay.Active
        assertEquals(ClassPhase.First, d.phase)
        assertEquals(2, d.items.size)
        assertEquals(1, d.position)
        assertEquals(2, d.total)
        assertEquals(LocalDate.of(2026, 11, 6), d.next?.date)
    }

    @Test fun duringAClassItIsNowAndEndsDropTheClass() {
        val now = widgetDay(week, at("2026-11-05T09:00:00")) as WidgetDay.Active
        assertEquals(ClassPhase.Now, now.phase)
        assertEquals(2, now.items.size)
        val gap = widgetDay(week, at("2026-11-05T09:30:00")) as WidgetDay.Active
        assertEquals(ClassPhase.Next, gap.phase)
        assertEquals(1, gap.items.size)
        assertEquals(2, gap.position)
    }

    @Test fun afterTheLastClassTheDayIsOverAndTomorrowIsNext() {
        val over = widgetDay(week, at("2026-11-05T13:00:00")) as WidgetDay.Over
        assertEquals(2, over.total)
        assertEquals(LocalDate.of(2026, 11, 6), over.next?.date)
    }

    @Test fun aFreeDaySkipsToTheNextDayWithClasses() {
        val free = widgetDay(week, at("2026-11-07T12:00:00")) as WidgetDay.Free
        assertTrue(free.weekend)
        assertEquals(LocalDate.of(2026, 11, 9), free.next?.date)
        val weekday = widgetDay(week, at("2026-11-10T12:00:00")) as WidgetDay.Free
        assertEquals(false, weekday.weekend)
        assertNull(weekday.next)
    }

    @Test fun classCrossingMidnightStaysUntilItEnds() {
        val items = timed(listOf(activity("2026-11-05T23:00:00", "2026-11-06T01:00:00")))
        val after = widgetDay(items, at("2026-11-06T00:30:00")) as WidgetDay.Active
        assertEquals(ClassPhase.Now, after.phase)
        assertEquals(0, after.position)
        assertTrue(widgetDay(items, at("2026-11-06T01:00:00")) is WidgetDay.Free)
    }

    @Test fun modeFollowsTokenCacheAndExpiry() {
        assertEquals(WidgetMode.LoggedOut, widgetMode(false, true, false))
        assertEquals(WidgetMode.LoggedOut, widgetMode(true, false, true))
        assertEquals(WidgetMode.Loading, widgetMode(true, false, false))
        assertEquals(WidgetMode.Content, widgetMode(true, true, true))
    }

    @Test fun staleLevelsSwitchAtSixAndFortyEightHours() {
        val now = 1_000_000_000_000L
        assertEquals(StaleLevel.Fresh, staleLevel(null, now))
        assertEquals(StaleLevel.Fresh, staleLevel(now - STALE_AFTER_MS + 1, now))
        assertEquals(StaleLevel.Stale, staleLevel(now - STALE_AFTER_MS, now))
        assertEquals(StaleLevel.Stale, staleLevel(now - VERY_STALE_AFTER_MS + 1, now))
        assertEquals(StaleLevel.VeryStale, staleLevel(now - VERY_STALE_AFTER_MS, now))
    }

    @Test fun anExpiredSessionWinsTheChip() {
        assertEquals(StatusChip.None, statusChip(false, StaleLevel.Fresh))
        assertEquals(StatusChip.Offline, statusChip(false, StaleLevel.Stale))
        assertEquals(StatusChip.OfflineOld, statusChip(false, StaleLevel.VeryStale))
        assertEquals(StatusChip.Expired, statusChip(true, StaleLevel.Fresh))
    }

    private fun redraw(now: String, synced: Long? = null, zone: ZoneId = warsaw): Instant {
        val n = at(now)
        return nextRedraw(widgetDay(week, n), n, zone, synced)
    }

    @Test fun redrawIsTheNextClassStartOrEndOrMidnight() {
        assertEquals(at("2026-11-05T08:00:00").atZone(warsaw).toInstant(), redraw("2026-11-05T07:00:00"))
        assertEquals(at("2026-11-05T09:30:00").atZone(warsaw).toInstant(), redraw("2026-11-05T09:00:00"))
        assertEquals(at("2026-11-05T11:30:00").atZone(warsaw).toInstant(), redraw("2026-11-05T09:30:00"))
        assertEquals(at("2026-11-06T00:00:00").atZone(warsaw).toInstant(), redraw("2026-11-05T14:00:00"))
    }

    @Test fun redrawComesEarlierWhenTheDataTurnsOld() {
        val now = at("2026-11-05T14:00:00")
        val nowMs = now.atZone(warsaw).toInstant().toEpochMilli()
        val synced = nowMs - STALE_AFTER_MS + 600_000L
        assertEquals(Instant.ofEpochMilli(nowMs + 600_000L), redraw("2026-11-05T14:00:00", synced))
        val old = nowMs - VERY_STALE_AFTER_MS + 60_000L
        assertEquals(Instant.ofEpochMilli(nowMs + 60_000L), redraw("2026-11-05T14:00:00", old))
        // already very old: nothing more to wait for but midnight
        assertEquals(at("2026-11-06T00:00:00").atZone(warsaw).toInstant(), redraw("2026-11-05T14:00:00", nowMs - 3 * VERY_STALE_AFTER_MS))
    }

    @Test fun redrawKeepsMidnightOnDstDaysAndMidnightCrossingClasses() {
        fun hoursToMidnight(day: String): Long {
            val noon = at("${day}T12:00:00")
            val end = nextRedraw(null, noon, warsaw, null)
            val start = LocalDate.parse(day).atStartOfDay(warsaw).toInstant()
            return Duration.between(start, end).toHours()
        }
        assertEquals(25, hoursToMidnight("2026-10-25"))
        assertEquals(23, hoursToMidnight("2026-03-29"))
        val items = timed(listOf(activity("2026-11-05T23:00:00", "2026-11-06T01:00:00")))
        val late = at("2026-11-05T23:30:00")
        assertEquals(at("2026-11-06T00:00:00").atZone(warsaw).toInstant(), nextRedraw(widgetDay(items, late), late, warsaw, null))
        val after = at("2026-11-06T00:30:00")
        assertEquals(at("2026-11-06T01:00:00").atZone(warsaw).toInstant(), nextRedraw(widgetDay(items, after), after, warsaw, null))
    }

    @Test fun bucketsFollowSection13() {
        assertEquals(WidgetBucket.Tiny, bucketOf(110f, 48f))
        assertEquals(WidgetBucket.Strip, bucketOf(250f, 60f))
        assertEquals(WidgetBucket.Small, bucketOf(110f, 110f))
        assertEquals(WidgetBucket.Medium, bucketOf(250f, 110f))
        assertEquals(WidgetBucket.Tall, bucketOf(180f, 330f))
        assertEquals(WidgetBucket.Large, bucketOf(250f, 250f))
    }

    @Test fun rowsFitTheHeightAndOverflowBecomesACount() {
        assertEquals(1, rowCapacity(110f))
        assertEquals(4, rowCapacity(250f))
        assertEquals(RowPlan(2, 0, 0, 0), planRows(2, 3, 2))
        assertEquals(RowPlan(2, 1, 0, 0), planRows(3, 3, 2))
        // a second day needs its header row: 2 today + header + 3 of 4 tomorrow in 6 rows
        assertEquals(RowPlan(2, 0, 3, 1), planRows(2, 4, 6))
        assertEquals(RowPlan(2, 0, 0, 0), planRows(2, 4, 3))
    }

    @Test fun motifLightsTheSlotOfTheNextClass() {
        assertEquals(0, motifColumn(LocalDate.of(2026, 11, 9)))
        assertNull(motifColumn(LocalDate.of(2026, 11, 8)))
        assertEquals(0, motifRow(at("2026-11-09T07:00:00")))
        assertEquals(1, motifRow(at("2026-11-09T10:30:00")))
        assertEquals(4, motifRow(at("2026-11-09T20:00:00")))
    }

    @Test fun stampShowsTimeTodayAndDayLater() {
        val today = LocalDate.of(2026, 11, 8)
        fun stamp(iso: String) = staleStamp(Instant.parse(iso).toEpochMilli(), ZoneId.of("UTC"), today, Locale.US)
        assertEquals("22:14", stamp("2026-11-08T22:14:00Z"))
        assertEquals("Sat 22:14", stamp("2026-11-07T22:14:00Z"))
        assertEquals("1 Nov 22:14", stamp("2026-11-01T22:14:00Z"))
    }

    @Test fun syncErrorsMapToOutcomes() {
        assertEquals(SyncOutcome.StopUnauthorized, outcomeOf(UsosException.Unauthorized("x")))
        assertEquals(SyncOutcome.Failure, outcomeOf(UsosException.Network("x")))
        assertEquals(SyncOutcome.Failure, outcomeOf(UsosException.Api(500u, null, null, emptyList(), "x")))
        assertEquals(SyncOutcome.Failure, outcomeOf(UsosException.Store("x")))
    }
}
