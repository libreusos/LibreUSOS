package io.gitlab.voxix.libreusos.ui

import io.gitlab.voxix.libreusos.data.pick
import org.libreusos.core.CourseTest
import org.libreusos.core.Term
import org.libreusos.core.TermTests
import org.libreusos.core.TestNode
import org.libreusos.core.TestNodeKind
import java.math.BigDecimal
import java.math.RoundingMode
import java.text.NumberFormat
import java.time.LocalDate
import java.time.LocalDateTime
import java.util.Locale

/** A term that has at least one test, with the term record when the terms list knows it. */
data class TestTermSection(val termId: String, val term: Term?, val tests: List<CourseTest>)

/**
 * Sections for the terms that have at least one test, newest term first (by start date, then end
 * date, then id). Terms missing from [terms] come last, newest id first. A term that appears twice
 * in [results] keeps its first entry.
 */
fun buildTestSections(terms: List<Term>, results: List<TermTests>): List<TestTermSection> {
    val byId = terms.associateBy { it.id }
    return results
        .distinctBy { it.termId }
        .filter { it.tests.isNotEmpty() }
        .map { TestTermSection(it.termId, byId[it.termId], it.tests) }
        .sortedWith(
            compareBy<TestTermSection> { it.term == null }
                .thenByDescending { it.term?.startDate.orEmpty() }
                .thenByDescending { it.term?.endDate.orEmpty() }
                .thenByDescending { it.termId },
        )
}

/**
 * The terms to fetch live: the [recent] newest by start date (then end date, then id), plus every
 * term in [terms] that has no cached tests. Older terms with a cache are read from the cache only.
 */
fun liveTermIds(terms: List<Term>, cachedTermIds: Set<String>, recent: Int = 2): List<String> {
    val newest = terms
        .sortedWith(compareByDescending<Term> { it.startDate }.thenByDescending { it.endDate }.thenByDescending { it.id })
        .take(recent)
        .map { it.id }
        .toSet()
    return terms.map { it.id }.distinct().filter { it in newest || it !in cachedTermIds }
}

/** The tests to show: [live] for the terms fetched live, [cached] for every other term. */
fun mergeTermTests(cached: List<TermTests>, live: List<TermTests>, liveIds: Collection<String>): List<TermTests> =
    cached.filter { it.termId !in liveIds } + live

/** Why a test shows no points: the lecturer hid them, or the test is for other class groups. */
enum class Unavailable { Hidden, Groups }

/** One partial result under a test: a task or a grade. [value] is blank while USOS has no result for it. */
data class TestCell(val key: String, val label: String, val value: String, val hidden: Boolean)

/**
 * One card of the Sprawdziany list. [title] is the test's own name; [changed] is when its result last
 * changed. [progress] is points over the maximum (0..1), or null when the core gave no maximum.
 * [unavailable] is set for a test whose points the student cannot see.
 */
data class TestCard(
    val key: String,
    val courseName: String,
    val title: String,
    val changed: LocalDate?,
    val points: String?,
    val progress: Float?,
    val cells: List<TestCell>,
    val unavailable: Unavailable?,
)

/**
 * Cards for the tests of [section], by course name (collator of [lang]) then test id. Partial cells
 * are the first level of the tree only. Totals are the core's: nothing is summed here.
 * shortcut: deeper levels of the tree and the lecturer's comments are not shown, until a test detail screen exists.
 */
fun buildTestCards(section: TestTermSection?, lang: String, locale: Locale): List<TestCard> {
    if (section == null) return emptyList()
    val collator = collatorFor(lang)
    return section.tests
        .mapIndexed { index, test -> Triple(index, test, test.courseName.pick(lang)) }
        .sortedWith(
            Comparator<Triple<Int, CourseTest, String>> { a, b -> collator.compare(a.third, b.third) }
                .thenBy { it.second.root.id }
                .thenBy { it.first },
        )
        .map { (index, test, courseName) ->
            val root = test.root
            val unavailable = when {
                root.hidden || root.resultsVisible == false -> Unavailable.Hidden
                test.limitedToGroups && root.points == null -> Unavailable.Groups
                else -> null
            }
            TestCard(
                key = "${section.termId}:${test.courseId}:${root.id}#$index",
                courseName = courseName,
                title = root.name.pick(lang),
                changed = root.lastChanged?.let { runCatching { LocalDateTime.parse(it).toLocalDate() }.getOrNull() },
                points = pointsLabel(root.points, root.pointsMax, locale),
                progress = progressOf(root.points, root.pointsMax),
                cells = if (unavailable == null) root.children.mapIndexed { i, node -> cellOf(node, i, lang, locale) } else emptyList(),
                unavailable = unavailable,
            )
        }
}

private fun cellOf(node: TestNode, index: Int, lang: String, locale: Locale): TestCell {
    val hidden = node.hidden || node.resultsVisible == false
    val value = when {
        hidden -> ""
        node.kind == TestNodeKind.GRADE -> node.grade?.symbol.orEmpty()
        else -> pointsLabel(node.points, node.pointsMax, locale).orEmpty()
    }
    return TestCell("${node.id}#$index", node.name.pick(lang), value, hidden)
}

/** Points as a share of the maximum, kept within 0..1; null without a positive maximum or without points. */
fun progressOf(points: Double?, max: Double?): Float? =
    if (points == null || max == null || !(max > 0.0) || !points.isFinite()) null else (points / max).toFloat().coerceIn(0f, 1f)

/**
 * Points without trailing zeros and without grouping, at most 2 decimals rounded half up:
 * `7`, `7.5`, `7,25` (by [locale]). `-0.0` and values that round to zero show as `0`.
 */
fun formatPoints(value: Double, locale: Locale): String {
    if (!value.isFinite()) return "\u2013"
    val rounded = BigDecimal.valueOf(value).setScale(2, RoundingMode.HALF_UP)
    val format = NumberFormat.getNumberInstance(locale)
    format.maximumFractionDigits = 2
    format.roundingMode = RoundingMode.HALF_UP
    format.isGroupingUsed = false
    return format.format(if (rounded.signum() == 0) BigDecimal.ZERO else rounded)
}

/** `x / max` for a node with both, `x` with only points, `– / max` with only a maximum, else null. */
fun pointsLabel(points: Double?, max: Double?, locale: Locale): String? = when {
    points != null && max != null -> "${formatPoints(points, locale)} / ${formatPoints(max, locale)}"
    points != null -> formatPoints(points, locale)
    max != null -> "– / ${formatPoints(max, locale)}"
    else -> null
}
