package io.github.voxix.libreusos

import io.github.voxix.libreusos.sync.SyncOutcome
import io.github.voxix.libreusos.sync.outcomeOf
import io.github.voxix.libreusos.widget.nextBoundary
import io.github.voxix.libreusos.widget.pickDay
import io.github.voxix.libreusos.widget.timed
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.libreusos.core.Activity
import org.libreusos.core.ActivityKind
import org.libreusos.core.LangText
import org.libreusos.core.UsosException
import java.time.Duration
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.ZoneId

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

    private val day = listOf(
        activity("2026-11-05T11:30:00", "2026-11-05T13:00:00"),
        activity("2026-11-05T08:00:00", "2026-11-05T09:30:00"),
        activity("2026-11-06T10:00:00", "2026-11-06T11:30:00"),
        activity("2026-11-09T10:00:00", "2026-11-09T11:30:00"),
        activity("broken", "broken"),
    )

    private fun at(text: String) = LocalDateTime.parse(text)

    @Test fun unparseableTimesAreDroppedAndTheRestIsSorted() {
        val all = timed(day)
        assertEquals(4, all.size)
        assertEquals(at("2026-11-05T08:00:00"), all.first().start)
    }

    @Test fun runningClassStaysUntilItEnds() {
        val plan = pickDay(timed(day), at("2026-11-05T09:00:00"))!!
        assertTrue(plan.isToday)
        assertEquals(2, plan.items.size)
        val later = pickDay(timed(day), at("2026-11-05T09:30:00"))!!
        assertEquals(1, later.items.size)
    }

    @Test fun afterTheLastClassTheNextDayWithClassesShows() {
        val plan = pickDay(timed(day), at("2026-11-05T13:00:00"))!!
        assertFalse(plan.isToday)
        assertEquals(LocalDate.of(2026, 11, 6), plan.date)
        val skip = pickDay(timed(day), at("2026-11-06T12:00:00"))!!
        assertEquals(LocalDate.of(2026, 11, 9), skip.date)
    }

    @Test fun nothingLeftInTheRange() {
        assertNull(pickDay(timed(day), at("2026-11-09T12:00:00")))
        assertNull(pickDay(emptyList(), at("2026-11-05T12:00:00")))
    }

    @Test fun boundaryIsTheNextEndOrMidnight() {
        val items = timed(day)
        val morning = at("2026-11-05T07:00:00")
        assertEquals(at("2026-11-05T09:30:00"), nextBoundary(pickDay(items, morning), morning))
        val noon = at("2026-11-05T12:00:00")
        assertEquals(at("2026-11-05T13:00:00"), nextBoundary(pickDay(items, noon), noon))
        val evening = at("2026-11-05T14:00:00")
        assertEquals(at("2026-11-06T00:00:00"), nextBoundary(pickDay(items, evening), evening))
        assertEquals(at("2026-11-06T00:00:00"), nextBoundary(null, evening))
    }

    @Test fun classCrossingMidnightStaysUntilItEnds() {
        val items = timed(listOf(activity("2026-11-05T23:00:00", "2026-11-06T01:00:00")))
        val late = at("2026-11-05T23:30:00")
        assertEquals(at("2026-11-06T00:00:00"), nextBoundary(pickDay(items, late), late))
        val after = at("2026-11-06T00:30:00")
        val plan = pickDay(items, after)!!
        assertTrue(plan.isToday)
        assertEquals(at("2026-11-06T01:00:00"), nextBoundary(plan, after))
        assertNull(pickDay(items, at("2026-11-06T01:00:00")))
    }

    @Test fun classEndingAtMidnightIsGoneAtMidnight() {
        val items = timed(listOf(activity("2026-11-05T22:00:00", "2026-11-06T00:00:00")))
        val late = at("2026-11-05T23:00:00")
        assertEquals(at("2026-11-06T00:00:00"), nextBoundary(pickDay(items, late), late))
        assertNull(pickDay(items, at("2026-11-06T00:00:00")))
    }

    @Test fun endBeforeStartIsDropped() {
        assertTrue(timed(listOf(activity("2026-11-05T10:00:00", "2026-11-05T09:00:00"))).isEmpty())
        assertEquals(1, timed(listOf(activity("2026-11-05T10:00:00", "2026-11-05T10:00:00"))).size)
    }

    @Test fun boundaryOnDstDaysIsStillTheNextLocalMidnight() {
        val warsaw = ZoneId.of("Europe/Warsaw")
        fun hoursToNextMidnight(day: String): Long {
            val noon = at("${day}T12:00:00")
            val boundary = nextBoundary(null, noon)
            assertEquals(LocalDate.parse(day).plusDays(1).atStartOfDay(), boundary)
            val start = LocalDate.parse(day).atStartOfDay(warsaw).toInstant()
            return Duration.between(start, boundary.atZone(warsaw).toInstant()).toHours()
        }
        assertEquals(25, hoursToNextMidnight("2026-10-25"))
        assertEquals(23, hoursToNextMidnight("2026-03-29"))
        // A class in the repeated hour (02:30 happens twice) still resolves to an instant.
        val repeated = timed(listOf(activity("2026-10-25T02:30:00", "2026-10-25T03:30:00")))
        val noon = at("2026-10-25T01:00:00")
        val end = nextBoundary(pickDay(repeated, noon), noon)
        assertEquals(at("2026-10-25T03:30:00"), end)
        end.atZone(warsaw).toInstant()
    }

    @Test fun syncErrorsMapToOutcomes() {
        val unauthorized = UsosException.Unauthorized("x")
        val network = UsosException.Network("x")
        val api = UsosException.Api(500u, null, null, emptyList(), "x")
        assertEquals(SyncOutcome.StopUnauthorized, outcomeOf(unauthorized, true, 0))
        assertEquals(SyncOutcome.Retry, outcomeOf(network, true, 0))
        assertEquals(SyncOutcome.Retry, outcomeOf(network, true, 4))
        assertEquals(SyncOutcome.Failure, outcomeOf(network, true, 5))
        assertEquals(SyncOutcome.Failure, outcomeOf(network, false, 0))
        assertEquals(SyncOutcome.Retry, outcomeOf(api, true, 2))
        assertEquals(SyncOutcome.Failure, outcomeOf(api, true, 3))
        assertEquals(SyncOutcome.Failure, outcomeOf(UsosException.NotCached("x"), true, 0))
        assertEquals(SyncOutcome.Failure, outcomeOf(UsosException.Store("x"), true, 0))
    }
}
