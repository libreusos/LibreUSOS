package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.ui.ClashPart
import io.gitlab.voxix.libreusos.ui.HourRange
import io.gitlab.voxix.libreusos.ui.clashGroups
import io.gitlab.voxix.libreusos.ui.clashParts
import io.gitlab.voxix.libreusos.ui.dayCells
import io.gitlab.voxix.libreusos.ui.dayCounts
import io.gitlab.voxix.libreusos.ui.groupMeetings
import io.gitlab.voxix.libreusos.ui.hourRange
import io.gitlab.voxix.libreusos.ui.termWeek
import io.gitlab.voxix.libreusos.ui.weekGrid
import io.gitlab.voxix.libreusos.ui.weekRangeLabel
import io.gitlab.voxix.libreusos.widget.timed
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.libreusos.core.Activity
import org.libreusos.core.ActivityKind
import org.libreusos.core.LangText
import java.time.LocalDate
import java.util.Locale

class PlanModelTest {
    private fun activity(start: String, end: String, course: String? = "C", type: String? = "WYK", group: UInt? = 1u) = Activity(
        kind = ActivityKind.Classgroup,
        start = start,
        end = end,
        date = start.take(10),
        name = LangText(null, null),
        courseId = course,
        courseName = LangText(course, null),
        classtypeId = type,
        classtypeName = LangText(null, null),
        groupNumber = group,
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

    private fun day(vararg times: String) = timed(times.map { t ->
        val (s, e) = t.split("-")
        activity("2026-11-12T$s:00", "2026-11-12T$e:00")
    })

    @Test
    fun backToBackClassesDoNotClash() {
        val items = day("08:30-10:00", "10:00-11:30")
        assertTrue(clashGroups(items).isEmpty())
        assertEquals(listOf(ClashPart.None, ClashPart.None), clashParts(items))
    }

    @Test
    fun pairClashNamesTheOverlap() {
        val items = day("08:30-10:00", "12:00-13:30", "12:45-14:15", "14:30-16:00")
        val group = clashGroups(items).single()
        assertEquals(2, group.items.size)
        assertEquals("2026-11-12T12:45", group.from.toString())
        assertEquals("2026-11-12T13:30", group.to.toString())
        assertEquals(45, group.minutes)
        assertEquals(
            listOf(ClashPart.None, ClashPart.First, ClashPart.Last, ClashPart.None),
            clashParts(items),
        )
    }

    @Test
    fun chainedClassesFormOneRunWithTheWholeStretch() {
        // The first and the third do not overlap each other, but both overlap the second.
        val items = day("09:00-10:00", "09:45-11:00", "10:30-12:00")
        val group = clashGroups(items).single()
        assertEquals(3, group.items.size)
        assertEquals(180, group.minutes)
        assertEquals(listOf(ClashPart.First, ClashPart.Middle, ClashPart.Last), clashParts(items))
    }

    @Test
    fun overlappingClassesShareTheColumnSideBySide() {
        val cells = dayCells(day("09:00-10:00", "12:00-13:30", "12:45-14:15", "13:30-14:00"))
        assertEquals(2, cells.size)
        assertEquals(1, cells[0].blocks.single().columns)
        val clash = cells[1]
        assertTrue(clash.isClash)
        // The third class starts when the first ends, so it takes the first one's column back.
        assertEquals(listOf(0, 1, 0), clash.blocks.map { it.column })
        assertEquals(2, clash.blocks[0].columns)
        assertEquals(12 * 60, clash.startMin)
        assertEquals(14 * 60 + 15, clash.endMin)
    }

    @Test
    fun hourRangeFitsTheDataButKeepsEightHours() {
        assertEquals(HourRange(8, 16), hourRange(emptyList()))
        assertEquals(HourRange(8, 16), hourRange(day("09:00-10:00")))
        assertEquals(HourRange(7, 16), hourRange(day("07:30-09:00")))
        assertEquals(HourRange(8, 20), hourRange(day("16:00-19:15")))
    }

    @Test
    fun weekGridHasFiveDaysUnlessTheWeekendHasClasses() {
        val monday = LocalDate.of(2026, 11, 9)
        val weekday = activity("2026-11-12T09:00:00", "2026-11-12T10:00:00")
        val saturday = activity("2026-11-14T09:00:00", "2026-11-14T10:00:00")
        assertEquals(5, weekGrid(listOf(weekday), monday).days.size)
        val full = weekGrid(listOf(weekday, saturday), monday)
        assertEquals(7, full.days.size)
        assertEquals(1, full.days[3].cells.size)
        assertEquals(1, full.days[5].cells.size)
        // Classes of other weeks are left out.
        assertEquals(0, weekGrid(listOf(weekday), monday.plusWeeks(1)).days.sumOf { it.cells.size })
    }

    @Test
    fun weekRangeLabelNamesTheMonthOnce() {
        assertEquals("9–15 listopada", weekRangeLabel(LocalDate.of(2026, 11, 9), Locale("pl")))
        assertEquals("9–15 November", weekRangeLabel(LocalDate.of(2026, 11, 9), Locale.UK))
        assertFalse(weekRangeLabel(LocalDate.of(2026, 11, 30), Locale("pl")).startsWith("30–"))
    }

    @Test
    fun groupMeetingsCountsTheSameGroupOnly() {
        val current = activity("2026-11-10T10:15:00", "2026-11-10T11:45:00")
        val cached = listOf(
            activity("2026-11-03T10:15:00", "2026-11-03T11:45:00"),
            current,
            activity("2026-11-17T10:15:00", "2026-11-17T11:45:00"),
            activity("2026-11-24T10:15:00", "2026-11-24T11:45:00"),
            activity("2026-11-18T10:15:00", "2026-11-18T11:45:00", group = 2u),
            activity("2026-11-19T10:15:00", "2026-11-19T11:45:00", type = "LAB"),
        )
        val meetings = groupMeetings(current, cached)
        assertEquals(4, meetings.total)
        assertEquals(2, meetings.position)
        assertEquals(listOf("2026-11-17T10:15:00", "2026-11-24T10:15:00"), meetings.upcoming.map { it.start })
    }

    @Test
    fun groupMeetingsOfAClassWithoutCourseIsJustThatClass() {
        val current = activity("2026-11-10T10:15:00", "2026-11-10T11:45:00", course = null)
        val meetings = groupMeetings(current, listOf(current, activity("2026-11-17T10:15:00", "2026-11-17T11:45:00", course = null)))
        assertEquals(1, meetings.total)
        assertEquals(1, meetings.position)
        assertTrue(meetings.upcoming.isEmpty())
    }

    @Test
    fun dayCountsGroupByStartDate() {
        val counts = dayCounts(
            listOf(
                activity("2026-11-12T08:00:00", "2026-11-12T09:30:00"),
                activity("2026-11-12T10:00:00", "2026-11-12T11:30:00"),
                activity("2026-11-13T08:00:00", "2026-11-13T09:30:00"),
            ),
        )
        assertEquals(mapOf(LocalDate.of(2026, 11, 12) to 2, LocalDate.of(2026, 11, 13) to 1), counts)
    }

    @Test
    fun termWeekCountsMondayWeeksFromTheWeekOfTheFirstDay() {
        val start = LocalDate.of(2026, 10, 1) // a Thursday: that week is week 1
        val end = LocalDate.of(2027, 1, 31)
        assertEquals(1, termWeek(start, end, LocalDate.of(2026, 10, 1)))
        assertEquals(1, termWeek(start, end, LocalDate.of(2026, 10, 4)))
        assertEquals(2, termWeek(start, end, LocalDate.of(2026, 10, 5)))
        assertEquals(7, termWeek(start, end, LocalDate.of(2026, 11, 10)))
        assertNull(termWeek(start, end, LocalDate.of(2026, 9, 30)))
        assertNull(termWeek(start, end, LocalDate.of(2027, 2, 1)))
    }
}
