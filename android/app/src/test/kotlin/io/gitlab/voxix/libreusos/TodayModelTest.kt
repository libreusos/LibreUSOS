package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.ui.RowState
import io.gitlab.voxix.libreusos.ui.Span
import io.gitlab.voxix.libreusos.ui.ceilMinutes
import io.gitlab.voxix.libreusos.ui.nextClasses
import io.gitlab.voxix.libreusos.ui.spanOf
import io.gitlab.voxix.libreusos.ui.splitNextClasses
import io.gitlab.voxix.libreusos.ui.todayPlan
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.libreusos.core.Activity
import org.libreusos.core.ActivityKind
import org.libreusos.core.LangText
import java.time.Duration
import java.time.LocalDate
import java.time.LocalDateTime

class TodayModelTest {
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

    private val today = LocalDate.of(2026, 11, 9)
    private val day = listOf(
        activity("2026-11-09T11:30:00", "2026-11-09T13:00:00"),
        activity("2026-11-09T08:00:00", "2026-11-09T09:30:00"),
        activity("2026-11-09T09:45:00", "2026-11-09T11:15:00"),
        activity("2026-11-09T14:30:00", "2026-11-09T16:00:00"),
        activity("2026-11-10T10:00:00", "2026-11-10T11:30:00"),
    )

    private fun at(time: String) = LocalDateTime.parse("2026-11-09T$time")

    @Test
    fun ceilMinutesRoundsUp() {
        assertEquals(0, ceilMinutes(Duration.ZERO))
        assertEquals(1, ceilMinutes(Duration.ofSeconds(1)))
        assertEquals(1, ceilMinutes(Duration.ofSeconds(60)))
        assertEquals(2, ceilMinutes(Duration.ofSeconds(61)))
        assertEquals(0, ceilMinutes(Duration.ofSeconds(-30)))
    }

    @Test
    fun spanSplitsHoursAndMinutes() {
        assertEquals(Span(0, 45), spanOf(45))
        assertEquals(Span(2, 0), spanOf(120))
        assertEquals(Span(1, 20), spanOf(80))
        assertEquals(Span(0, 1), spanOf(0))
    }

    @Test
    fun planKeepsOnlyTodayInOrder() {
        val plan = todayPlan(day, today, at("07:00"))
        assertEquals(4, plan.rows.size)
        assertEquals(listOf("08:00", "09:45", "11:30", "14:30"), plan.rows.map { it.item.start.toLocalTime().toString() })
    }

    @Test
    fun statesFollowTheClock() {
        val plan = todayPlan(day, today, at("10:00"))
        assertEquals(listOf(RowState.Past, RowState.Now, RowState.Upcoming, RowState.Upcoming), plan.rows.map { it.state })
        val card = plan.card!!
        assertTrue(card.current)
        assertEquals(1, card.index)
        assertEquals(75, card.minutes)
        assertEquals(2, card.nextIndex)
        assertEquals(15f / 90f, card.progress, 0.001f)
    }

    @Test
    fun cardIsTheNextClassBetweenClasses() {
        val card = todayPlan(day, today, at("11:20")).card!!
        assertEquals(2, card.index)
        assertEquals(false, card.current)
        assertEquals(10, card.minutes)
        assertEquals(0f, card.progress, 0f)
        assertEquals(3, card.nextIndex)
    }

    @Test
    fun noCardAfterTheLastClass() {
        val plan = todayPlan(day, today, at("17:00"))
        assertNull(plan.card)
        assertTrue(plan.rows.all { it.state == RowState.Past })
    }

    @Test
    fun anEndedClassIsPastAtItsEnd() {
        assertEquals(RowState.Past, todayPlan(day, today, at("09:30")).rows[0].state)
    }

    @Test
    fun shortBreaksAreNotGaps() {
        val plan = todayPlan(day, today, at("07:00"))
        assertNull(plan.rows[0].gapBeforeMin)
        assertNull(plan.rows[1].gapBeforeMin)
        assertNull(plan.rows[2].gapBeforeMin)
        assertEquals(90, plan.rows[3].gapBeforeMin)
    }

    @Test
    fun aGapStartsAtTheLatestEnd() {
        val overlapping = listOf(
            activity("2026-11-09T08:00:00", "2026-11-09T12:00:00"),
            activity("2026-11-09T09:00:00", "2026-11-09T10:00:00"),
            activity("2026-11-09T12:45:00", "2026-11-09T14:00:00"),
        )
        val plan = todayPlan(overlapping, today, at("07:00"))
        assertNull(plan.rows[1].gapBeforeMin)
        assertEquals(45, plan.rows[2].gapBeforeMin)
    }

    @Test
    fun emptyDayHasNoRowsAndNoCard() {
        val plan = todayPlan(listOf(day.last()), today, at("07:00"))
        assertTrue(plan.rows.isEmpty())
        assertNull(plan.card)
    }

    @Test
    fun nextClassesAreTheFirstLaterDay() {
        val next = nextClasses(day, today.plusDays(1).minusDays(1))!!
        assertEquals(LocalDate.of(2026, 11, 10), next.date)
        assertEquals(1, next.items.size)
        assertNull(nextClasses(day, LocalDate.of(2026, 11, 10)))
    }

    @Test
    fun longNextListFoldsIntoOneRow() {
        val items = day.take(4).let { io.gitlab.voxix.libreusos.widget.timed(it) }
        assertEquals(3 to 0, splitNextClasses(items.take(3)).let { it.first.size to it.second })
        val (shown, more) = splitNextClasses(items)
        assertEquals(2, shown.size)
        assertEquals(2, more)
    }
}
