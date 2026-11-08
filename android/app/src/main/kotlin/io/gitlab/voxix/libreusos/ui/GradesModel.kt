package io.gitlab.voxix.libreusos.ui

import io.gitlab.voxix.libreusos.data.pick
import io.gitlab.voxix.libreusos.ui.theme.KindCode
import io.gitlab.voxix.libreusos.ui.theme.kindCodeOf
import org.libreusos.core.CourseGrades
import org.libreusos.core.Grade
import org.libreusos.core.Term
import org.libreusos.core.TermGrades
import java.text.Collator
import java.time.LocalDate
import java.time.LocalDateTime
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

/** A term with its courses: the ones that have a grade or are among the user's course editions. */
data class TermSection(val termId: String, val term: Term?, val grades: TermGrades) {
    /** False while no course of the term has a grade yet (the empty state of the screen). */
    val hasGrades: Boolean get() = grades.courses.any(::hasAnyGrade)
}

/** True when the course has a grade at course level or in any unit. */
fun hasAnyGrade(course: CourseGrades): Boolean = course.grades.isNotEmpty() || course.units.any { it.grades.isNotEmpty() }

/** A course is listed when it has a grade or a name (USOS leaves the name empty for courses outside the user's editions). */
fun isListed(course: CourseGrades): Boolean =
    hasAnyGrade(course) || !course.name.pl.isNullOrBlank() || !course.name.en.isNullOrBlank()

/** One section per term that has at least one listed course. A term that appears twice in [grades] keeps its first entry. */
fun buildSections(terms: List<Term>, grades: List<TermGrades>): List<TermSection> {
    val byId = terms.associateBy { it.id }
    return grades
        .distinctBy { it.termId }
        .filter { g -> g.courses.any(::isListed) }
        .map { TermSection(it.termId, byId[it.termId], it) }
}

private val collators = ConcurrentHashMap<String, Collator>()

/** The collator of [lang], created once: it sorts Polish diacritics where a plain string compare would not. */
fun collatorFor(lang: String): Collator = collators.getOrPut(lang) { Collator.getInstance(Locale(lang)) }

/** The grade a course tile shows and, after a retake, the first sitting it replaced. */
data class FinalGrade(val grade: Grade, val earlier: Grade?)

/**
 * The final grade of a course: the latest session of the exam report that passes (the last one when
 * several do), else of the last report. [FinalGrade.earlier] is the first sitting of that report when
 * the shown one is a later session. Null while the course has no course-level grade.
 */
fun finalGradeOf(course: CourseGrades): FinalGrade? {
    val reports = gradeReports(course.grades)
    val report = reports.lastOrNull { it.last().passes == true } ?: reports.lastOrNull() ?: return null
    return FinalGrade(report.last(), report.first().takeIf { report.size > 1 })
}

/** One course of the list. */
data class CourseRow(
    val key: String,
    val course: CourseGrades,
    val name: String,
    /** The class types of the units, one badge each. */
    val kinds: List<KindCode>,
    val final: FinalGrade?,
    /** The one best final grade of the term (highlighted); false when several tie. */
    val best: Boolean,
)

/**
 * The rows of [section]: listed courses sorted by name with the collator of [lang] (then id). Grades are
 * shown as USOS gives them; the only comparison made here is which final grade is the single best one.
 */
fun buildCourseRows(section: TermSection?, lang: String): List<CourseRow> {
    section ?: return emptyList()
    val collator = collatorFor(lang)
    val named = section.grades.courses
        .filter(::isListed)
        .map { it to it.name.pick(lang) }
        .sortedWith(Comparator<Pair<CourseGrades, String>> { a, b -> collator.compare(a.second, b.second) }.thenBy { it.first.courseId })
    val finals = named.map { (course, _) -> finalGradeOf(course) }
    val values = finals.map { f -> f?.grade?.takeIf { it.passes != false }?.numericValue }
    val top = values.filterNotNull().maxOrNull()
    val bestIndex = top?.let { t -> values.indices.singleOrNull { values[it] == t } }
    return named.mapIndexed { i, (course, name) ->
        CourseRow(
            key = "course:${section.termId}:${course.courseId}",
            course = course,
            name = name,
            kinds = course.units
                .map { kindCodeOf(it.classtypeId, it.classtypeName.pick(lang).ifBlank { null }) }
                .distinctBy { it.code },
            final = finals[i],
            best = i == bestIndex,
        )
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

/** The day a grade was given: its acquisition date, else the last modification; null when USOS gave neither. */
fun gradeDate(grade: Grade): LocalDate? =
    listOf(grade.dateAcquisition, grade.dateModified).firstNotNullOfOrNull { s ->
        s?.let { runCatching { LocalDateTime.parse(it).toLocalDate() }.getOrNull() }
    }
