package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.ui.GradesRow
import io.gitlab.voxix.libreusos.ui.Outcome
import io.gitlab.voxix.libreusos.ui.SessionLabel
import io.gitlab.voxix.libreusos.ui.buildRows
import io.gitlab.voxix.libreusos.ui.formatNumber
import io.gitlab.voxix.libreusos.ui.gradeReports
import io.gitlab.voxix.libreusos.ui.latestPerReport
import io.gitlab.voxix.libreusos.ui.buildSections
import io.gitlab.voxix.libreusos.ui.courseOutcome
import io.gitlab.voxix.libreusos.ui.outcomeOf
import io.gitlab.voxix.libreusos.ui.sessionLabel
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.libreusos.core.CourseGrades
import org.libreusos.core.DataSource
import org.libreusos.core.Grade
import org.libreusos.core.LangText
import org.libreusos.core.Term
import org.libreusos.core.TermGrades
import org.libreusos.core.UnitGrades
import java.util.Locale

class GradesModelTest {
    private fun term(id: String, start: String, end: String) = Term(
        id = id,
        name = LangText("Semestr $id", "Term $id"),
        startDate = start,
        endDate = end,
        finishDate = null,
        orderKey = null,
        isActive = null,
    )

    private fun grade(session: UInt, symbol: String?, passes: Boolean?, examId: String? = "E") = Grade(
        examId = examId,
        session = session,
        symbol = symbol,
        description = LangText(null, null),
        passes = passes,
        countsIntoAverage = true,
        comment = null,
        dateModified = null,
        dateAcquisition = null,
        numericValue = null,
    )

    private fun course(id: String, name: String?, vararg grades: Grade, units: List<UnitGrades> = emptyList()) = CourseGrades(
        courseId = id,
        name = LangText(name, null),
        termId = "T",
        ects = null,
        grades = grades.toList(),
        units = units,
        courseGrade = null,
    )

    private fun termGrades(id: String, vararg courses: CourseGrades, average: Double? = null) = TermGrades(
        termId = id,
        courses = courses.toList(),
        average = average,
        weightedAverage = null,
        source = DataSource.Live,
    )

    private val passed = grade(1u, "4,5", true)

    @Test fun newestTermComesFirstAndTermsWithoutGradesDisappear() {
        val terms = listOf(
            term("2025/2026-Z", "2025-10-01", "2026-02-15"),
            term("2026/2027-Z", "2026-10-01", "2027-02-15"),
            term("2025/2026-L", "2026-02-16", "2026-09-30"),
            term("2024/2025-Z", "2024-10-01", "2025-02-15"),
        )
        val grades = listOf(
            termGrades("2025/2026-Z", course("a", "A", passed)),
            termGrades("2026/2027-Z", course("b", "B", passed)),
            termGrades("2025/2026-L", course("c", "C", passed)),
            termGrades("2024/2025-Z", course("d", "D")),
        )
        assertEquals(
            listOf("2026/2027-Z", "2025/2026-L", "2025/2026-Z"),
            buildSections(terms, grades).map { it.termId },
        )
    }

    @Test fun unknownTermsComeLastNewestIdFirst() {
        val terms = listOf(term("T1", "2020-10-01", "2021-02-01"))
        val grades = listOf(
            termGrades("A", course("a", "A", passed)),
            termGrades("T1", course("b", "B", passed)),
            termGrades("Z", course("c", "C", passed)),
            termGrades("T1", course("dup", "Dup", passed)),
        )
        val sections = buildSections(terms, grades)
        assertEquals(listOf("T1", "Z", "A"), sections.map { it.termId })
        assertEquals("b", sections[0].grades.courses.single().courseId)
        assertNull(sections[1].term)
    }

    @Test fun unitGradesAloneKeepACourseListed() {
        val unit = UnitGrades("U", "LAB", LangText("Laboratorium", null), listOf(passed))
        val grades = listOf(termGrades("T1", course("a", "A", units = listOf(unit))))
        assertEquals(1, buildSections(emptyList(), grades).size)
    }

    @Test fun rowsHaveAHeaderPerTermAndCoursesSortedByName() {
        val terms = listOf(term("T1", "2026-10-01", "2027-02-01"))
        val sections = buildSections(
            terms,
            listOf(
                termGrades(
                    "T1",
                    course("2", "zoologia", passed),
                    course("1", "Algebra", passed),
                    course("3", "Empty"),
                    course("4", "ćwiczenia", passed),
                    average = 4.25,
                ),
            ),
        )
        val rows = buildRows(sections, "pl")
        val header = rows[0] as GradesRow.Header
        assertEquals("Semestr T1", header.title)
        assertEquals(4.25, header.average!!, 0.0)
        assertNull(header.weightedAverage)
        assertEquals(listOf("Algebra", "ćwiczenia", "zoologia"), rows.drop(1).map { (it as GradesRow.Course).name })
        assertEquals(rows.size, rows.map { it.key }.toSet().size)
        assertEquals("Term T1", (buildRows(sections, "en")[0] as GradesRow.Header).title)
    }

    @Test fun headerFallsBackToTheTermId() {
        val rows = buildRows(buildSections(emptyList(), listOf(termGrades("X", course("a", "A", passed)))), "en")
        assertEquals("X", (rows[0] as GradesRow.Header).title)
    }

    @Test fun outcomeFollowsTheLatestCourseLevelSession() {
        val failed = grade(1u, "2", false)
        val retake = grade(2u, "3", true)
        assertEquals(Outcome.FAIL, courseOutcome(course("a", "A", failed)))
        assertEquals(Outcome.PASS, courseOutcome(course("a", "A", retake, failed)))
        assertEquals(Outcome.UNKNOWN, courseOutcome(course("a", "A")))
        assertEquals(Outcome.UNKNOWN, outcomeOf(grade(1u, "ZAL", null)))
        assertEquals(Outcome.PASS, outcomeOf(passed))
    }

    @Test fun reportsAreGroupedBySessionWithinEachExam() {
        val e1s2 = grade(2u, "3", true, "E1")
        val e2s1 = grade(1u, "2", false, "E2")
        val e1s1 = grade(1u, "2", false, "E1")
        val loose = grade(1u, "ZAL", true, null)
        val reports = gradeReports(listOf(e1s2, e2s1, e1s1, loose))
        assertEquals(listOf(listOf(e1s1, e1s2), listOf(e2s1), listOf(loose)), reports)
        assertEquals(listOf(e1s2, e2s1, loose), latestPerReport(listOf(e1s2, e2s1, e1s1, loose)))
    }

    @Test fun courseOutcomeUsesTheLatestSessionOfEachReport() {
        val failR1 = grade(1u, "2", false, "E1")
        val passR1 = grade(2u, "3", true, "E1")
        val failR2 = grade(1u, "2", false, "E2")
        val unknown = grade(1u, "ZAL", null, "E3")
        // A passed retake of one report passes the course even though another report failed.
        assertEquals(Outcome.PASS, courseOutcome(course("a", "A", failR1, passR1, failR2)))
        // Every report's latest session fails.
        assertEquals(Outcome.FAIL, courseOutcome(course("a", "A", failR1, failR2)))
        // A passed first session followed by a failed retake of the same report fails.
        assertEquals(Outcome.FAIL, courseOutcome(course("a", "A", grade(1u, "3", true, "E1"), grade(2u, "2", false, "E1"))))
        assertEquals(Outcome.UNKNOWN, courseOutcome(course("a", "A", failR1, unknown)))
    }

    private fun unit(vararg grades: Grade) = UnitGrades("U", "LAB", LangText("Lab", null), grades.toList())

    @Test fun unitGradesTintACourseWithoutCourseLevelGrades() {
        val pass = grade(1u, "ZAL", true)
        val fail = grade(1u, "NZAL", false)
        val retaken = listOf(grade(1u, "NZAL", false), grade(2u, "ZAL", true))
        fun outcome(vararg units: UnitGrades) = courseOutcome(course("a", "A", units = units.toList()))
        assertEquals(Outcome.FAIL, outcome(unit(pass), unit(fail)))
        assertEquals(Outcome.PASS, outcome(unit(pass), unit(*retaken.toTypedArray())))
        assertEquals(Outcome.UNKNOWN, outcome(unit(pass), unit(grade(1u, "ZAL", null))))
        assertEquals(Outcome.UNKNOWN, outcome(unit()))
        assertEquals(Outcome.UNKNOWN, outcome())
        // Course-level grades win over the units.
        assertEquals(Outcome.PASS, courseOutcome(course("a", "A", passed, units = listOf(unit(fail)))))
    }

    @Test fun numbersUseTheLocaleSeparator() {
        assertEquals("4,33", formatNumber(13.0 / 3, Locale("pl")))
        assertEquals("4.33", formatNumber(13.0 / 3, Locale.US))
    }

    @Test fun sessionsAreLabelled() {
        assertEquals(SessionLabel.First, sessionLabel(1u))
        assertEquals(SessionLabel.Retake, sessionLabel(2u))
        assertEquals(SessionLabel.Later(3), sessionLabel(3u))
        assertTrue(sessionLabel(0u) is SessionLabel.Later)
    }
}
