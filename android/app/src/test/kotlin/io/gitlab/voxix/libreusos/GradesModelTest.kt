package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.ui.Outcome
import io.gitlab.voxix.libreusos.ui.SessionLabel
import io.gitlab.voxix.libreusos.ui.TermSection
import io.gitlab.voxix.libreusos.ui.buildCourseRows
import io.gitlab.voxix.libreusos.ui.buildSections
import io.gitlab.voxix.libreusos.ui.finalGradeOf
import io.gitlab.voxix.libreusos.ui.gradeDate
import io.gitlab.voxix.libreusos.ui.gradeReports
import io.gitlab.voxix.libreusos.ui.outcomeOf
import io.gitlab.voxix.libreusos.ui.sessionLabel
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
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
import java.time.LocalDate

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

    private fun grade(session: UInt, symbol: String?, passes: Boolean?, examId: String? = "E", numeric: Double? = null) = Grade(
        examId = examId,
        session = session,
        symbol = symbol,
        description = LangText(null, null),
        passes = passes,
        countsIntoAverage = true,
        comment = null,
        dateModified = null,
        dateAcquisition = null,
        numericValue = numeric,
    )

    private fun course(id: String, name: String?, vararg grades: Grade, units: List<UnitGrades> = emptyList(), ects: Double? = null) = CourseGrades(
        courseId = id,
        name = LangText(name, null),
        termId = "T",
        ects = ects,
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

    private fun unit(id: String, vararg grades: Grade) = UnitGrades("U$id", id, LangText(id, null), grades.toList())

    @Test fun termsKeepTheirOrderAndOnlyListedCoursesCount() {
        val terms = listOf(term("T1", "2025-10-01", "2026-02-15"))
        val grades = listOf(
            termGrades("T1", course("a", "A", passed)),
            termGrades("T2", course("b", null)),
            termGrades("T1", course("c", "C")),
        )
        val sections = buildSections(terms, grades)
        assertEquals(listOf("T1"), sections.map { it.termId })
        assertEquals("a", sections.single().grades.courses.single().courseId)
        assertEquals(terms.single(), sections.single().term)
    }

    @Test fun aTermHasGradesOnlyWhenACourseHasOne() {
        val withUnit = course("a", "A", units = listOf(unit("LAB", passed)))
        assertTrue(TermSection("T", null, termGrades("T", withUnit)).hasGrades)
        assertFalse(TermSection("T", null, termGrades("T", course("a", "A"))).hasGrades)
    }

    @Test fun theFinalGradeIsTheLatestSittingOfThePassingReport() {
        val failed = grade(1u, "2,0", false)
        val retake = grade(2u, "3,0", true)
        val final = finalGradeOf(course("a", "A", failed, retake))!!
        assertEquals(retake, final.grade)
        assertEquals(failed, final.earlier)
        val single = finalGradeOf(course("a", "A", passed))!!
        assertEquals(passed, single.grade)
        assertNull(single.earlier)
        assertNull(finalGradeOf(course("a", "A")))
    }

    @Test fun aLaterPassingReportWinsOverAFailedLastOne() {
        val pass = grade(1u, "4,0", true, "E1")
        val fail = grade(1u, "2,0", false, "E2")
        assertEquals(pass, finalGradeOf(course("a", "A", pass, fail))!!.grade)
        assertEquals(fail, finalGradeOf(course("a", "A", fail))!!.grade)
    }

    @Test fun rowsSortByNameAndMarkTheSingleBestGrade() {
        val section = TermSection(
            "T",
            null,
            termGrades(
                "T",
                course("z", "Zeta", grade(1u, "5,0", true, numeric = 5.0), units = listOf(unit("WYK"), unit("LAB"), unit("WYK")), ects = 5.0),
                course("a", "Ąbc", grade(1u, "3,5", true, numeric = 3.5)),
                course("m", "Mid"),
            ),
        )
        val rows = buildCourseRows(section, "pl")
        assertEquals(listOf("Ąbc", "Mid", "Zeta"), rows.map { it.name })
        assertEquals(listOf(false, false, true), rows.map { it.best })
        assertEquals(listOf("WYK", "LAB"), rows.last().kinds.map { it.code })
        assertNull(rows[1].final)
        assertTrue(buildCourseRows(null, "pl").isEmpty())
    }

    @Test fun tiedTopGradesAndFailedGradesAreNeverHighlighted() {
        val tie = TermSection("T", null, termGrades("T", course("a", "A", grade(1u, "5,0", true, numeric = 5.0)), course("b", "B", grade(1u, "5,0", true, numeric = 5.0))))
        assertTrue(buildCourseRows(tie, "pl").none { it.best })
        val failedOnly = TermSection("T", null, termGrades("T", course("a", "A", grade(1u, "2,0", false, numeric = 2.0))))
        assertTrue(buildCourseRows(failedOnly, "pl").none { it.best })
    }

    @Test fun outcomeFollowsWhatUsosSays() {
        assertEquals(Outcome.UNKNOWN, outcomeOf(grade(1u, "ZAL", null)))
        assertEquals(Outcome.PASS, outcomeOf(passed))
        assertEquals(Outcome.FAIL, outcomeOf(grade(1u, "2", false)))
    }

    @Test fun reportsAreGroupedBySessionWithinEachExam() {
        val e1s2 = grade(2u, "3", true, "E1")
        val e2s1 = grade(1u, "2", false, "E2")
        val e1s1 = grade(1u, "2", false, "E1")
        val loose = grade(1u, "ZAL", true, null)
        val reports = gradeReports(listOf(e1s2, e2s1, e1s1, loose))
        assertEquals(listOf(listOf(e1s1, e1s2), listOf(e2s1), listOf(loose)), reports)
    }

    @Test fun sessionsAreLabelled() {
        assertEquals(SessionLabel.First, sessionLabel(1u))
        assertEquals(SessionLabel.Retake, sessionLabel(2u))
        assertEquals(SessionLabel.Later(3), sessionLabel(3u))
        assertTrue(sessionLabel(0u) is SessionLabel.Later)
    }

    @Test fun aGradeDateIsTheAcquisitionDayElseTheModification() {
        val base = grade(1u, "4", true)
        assertEquals(LocalDate.of(2026, 6, 16), gradeDate(base.copy(dateAcquisition = "2026-06-16T10:00:00", dateModified = "2026-07-01T08:00:00")))
        assertEquals(LocalDate.of(2026, 7, 1), gradeDate(base.copy(dateModified = "2026-07-01T08:00:00")))
        assertNull(gradeDate(base))
        assertNull(gradeDate(base.copy(dateAcquisition = "garbage")))
    }
}
