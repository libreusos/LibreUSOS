package io.gitlab.voxix.libreusos.ui

import io.gitlab.voxix.libreusos.data.pick
import org.libreusos.core.CourseGrades
import org.libreusos.core.Grade
import org.libreusos.core.Term
import org.libreusos.core.TermGrades
import java.text.Collator
import java.util.Locale
import java.util.concurrent.ConcurrentHashMap

/** Whether a grade passes, as USOS says; `UNKNOWN` when it does not say. */
enum class Outcome { PASS, FAIL, UNKNOWN }

fun outcomeOf(grade: Grade): Outcome = when (grade.passes) {
    true -> Outcome.PASS
    false -> Outcome.FAIL
    null -> Outcome.UNKNOWN
}

/**
 * [grades] grouped by exam report (`examId`), reports in order of first appearance, each sorted by
 * session. A grade without an exam id is its own report.
 */
fun gradeReports(grades: List<Grade>): List<List<Grade>> {
    val reports = LinkedHashMap<String, MutableList<Grade>>()
    grades.forEachIndexed { i, g -> reports.getOrPut(g.examId ?: "\u0000#$i") { mutableListOf() } += g }
    return reports.values.map { report -> report.sortedBy { it.session } }
}

/** The latest session of each exam report. */
fun latestPerReport(grades: List<Grade>): List<Grade> = gradeReports(grades).map { it.last() }

/**
 * The outcome of a course. With course-level grades, take the latest session of each exam report:
 * PASS if any report passes, FAIL if every report fails, else UNKNOWN. Without them, look at the
 * units (each unit's latest session): FAIL if any unit fails, PASS if all pass, else UNKNOWN.
 */
fun courseOutcome(course: CourseGrades): Outcome {
    if (course.grades.isNotEmpty()) {
        val latest = latestPerReport(course.grades).map(::outcomeOf)
        return when {
            Outcome.PASS in latest -> Outcome.PASS
            latest.all { it == Outcome.FAIL } -> Outcome.FAIL
            else -> Outcome.UNKNOWN
        }
    }
    val units = course.units.mapNotNull { u -> u.grades.maxByOrNull { it.session }?.let(::outcomeOf) }
    return when {
        units.isEmpty() -> Outcome.UNKNOWN
        Outcome.FAIL in units -> Outcome.FAIL
        units.all { it == Outcome.PASS } -> Outcome.PASS
        else -> Outcome.UNKNOWN
    }
}

/** A term that has graded courses, with the term record when the terms list knows it. */
data class TermSection(val termId: String, val term: Term?, val grades: TermGrades)

/** True when the course has a grade at course level or in any unit. */
fun hasAnyGrade(course: CourseGrades): Boolean = course.grades.isNotEmpty() || course.units.any { it.grades.isNotEmpty() }

/**
 * Sections for the terms that have at least one graded course, newest term first (by start date,
 * then end date, then id). Terms missing from [terms] come last, newest id first. A term that
 * appears twice in [grades] keeps its first entry.
 */
fun buildSections(terms: List<Term>, grades: List<TermGrades>): List<TermSection> {
    val byId = terms.associateBy { it.id }
    return grades
        .distinctBy { it.termId }
        .filter { g -> g.courses.any(::hasAnyGrade) }
        .map { TermSection(it.termId, byId[it.termId], it) }
        .sortedWith(
            compareBy<TermSection> { it.term == null }
                .thenByDescending { it.term?.startDate.orEmpty() }
                .thenByDescending { it.term?.endDate.orEmpty() }
                .thenByDescending { it.termId },
        )
}

private val collators = ConcurrentHashMap<String, Collator>()

/** The collator of [lang], created once: it sorts Polish diacritics where a plain string compare would not. */
fun collatorFor(lang: String): Collator = collators.getOrPut(lang) { Collator.getInstance(Locale(lang)) }

/** One line of the list: a term header or a course. Keys are unique and stable. */
sealed interface GradesRow {
    val key: String

    data class Header(
        override val key: String,
        val title: String,
        val average: Double?,
        val weightedAverage: Double?,
    ) : GradesRow

    data class Course(override val key: String, val course: CourseGrades, val name: String) : GradesRow
}

/**
 * Flattens [sections] into rows for a lazy list. Only graded courses are listed, sorted by name with the collator of
 * [lang] (then id). The averages are the core's; nothing is computed here.
 */
fun buildRows(sections: List<TermSection>, lang: String): List<GradesRow> = buildList {
    val collator = collatorFor(lang)
    for (section in sections) {
        val title = section.term?.name?.pick(lang)?.ifBlank { null } ?: section.termId
        add(
            GradesRow.Header(
                "term:${section.termId}",
                title,
                section.grades.average,
                section.grades.weightedAverage,
            ),
        )
        section.grades.courses
            .filter(::hasAnyGrade)
            .map { it to it.name.pick(lang) }
            .sortedWith(Comparator<Pair<CourseGrades, String>> { a, b -> collator.compare(a.second, b.second) }.thenBy { it.first.courseId })
            .forEach { (course, name) -> add(GradesRow.Course("course:${section.termId}:${course.courseId}", course, name)) }
    }
}

/** The session of a grade as a label key: first sitting, retake, or a numbered later one. */
sealed interface SessionLabel {
    data object First : SessionLabel
    data object Retake : SessionLabel
    data class Later(val number: Int) : SessionLabel
}

fun sessionLabel(session: UInt): SessionLabel = when (session) {
    1u -> SessionLabel.First
    2u -> SessionLabel.Retake
    else -> SessionLabel.Later(session.toInt())
}
