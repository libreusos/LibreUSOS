package io.github.voxix.libreusos.ui

import io.github.voxix.libreusos.data.pick
import org.libreusos.core.CourseTest
import org.libreusos.core.Term
import org.libreusos.core.TermTests
import org.libreusos.core.TestNode
import java.math.BigDecimal
import java.math.RoundingMode
import java.text.NumberFormat
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

/** One line of the list: a term header, a course test, or a node of its tree. Keys are unique and stable. */
sealed interface TestsRow {
    val key: String

    data class Header(override val key: String, val title: String) : TestsRow

    /** The root of one course test. [name] is blank when USOS gave none; [subtitle] is the root's own name. */
    data class Course(
        override val key: String,
        val test: CourseTest,
        val name: String,
        val subtitle: String?,
        val expanded: Boolean,
        val expandable: Boolean,
    ) : TestsRow

    /** [depth] is 0 for the first level of a test. [name] is blank when USOS gave none. */
    data class Node(
        override val key: String,
        val node: TestNode,
        val name: String,
        val depth: Int,
        val expanded: Boolean,
        val expandable: Boolean,
    ) : TestsRow
}

/** Stable, unique key of a course test: the position in its term keeps duplicate API ids apart. */
private fun courseKey(termId: String, test: CourseTest, index: Int) = "course:$termId:${test.courseId}:${test.root.id}#$index"

/** Stable, unique key of a node: its parent's key, its id and its position among its siblings. */
private fun nodeKey(parentKey: String, node: TestNode, index: Int) = "$parentKey/${node.id}#$index"

/** Every key a row of [sections] can have, open or not. Used to drop choices for rows that are gone. */
fun testKeys(sections: List<TestTermSection>): Set<String> = buildSet {
    fun walk(nodes: List<TestNode>, parentKey: String) {
        nodes.forEachIndexed { i, node ->
            val key = nodeKey(parentKey, node, i)
            add(key)
            walk(node.children, key)
        }
    }
    for (section in sections) {
        section.tests.forEachIndexed { i, test ->
            val key = courseKey(section.termId, test, i)
            add(key)
            walk(test.root.children, key)
        }
    }
}

/** [toggled] without the keys that no longer exist in [sections]. */
fun pruneToggled(toggled: Set<String>, sections: List<TestTermSection>): Set<String> =
    if (toggled.isEmpty()) toggled else toggled intersect testKeys(sections)

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

/**
 * Whether the item under [key] is open. [toggled] holds the keys the user flipped away from their
 * default, so new data never loses a choice and an unknown key simply has its default.
 */
fun isExpanded(key: String, defaultExpanded: Boolean, toggled: Set<String>): Boolean =
    defaultExpanded != (key in toggled)

/** Flips [key] between its default and the other state. */
fun toggleKey(toggled: Set<String>, key: String): Set<String> = if (key in toggled) toggled - key else toggled + key

/**
 * Flattens [sections] into rows for a lazy list. Courses sort by name with the collator of [lang]
 * (then by root id); the tree keeps the lecturer's order. A course test opens by default, folders
 * start closed; only open items list their children. Totals are the core's: nothing is summed here.
 */
fun buildTestRows(sections: List<TestTermSection>, toggled: Set<String>, lang: String): List<TestsRow> = buildList {
    val collator = collatorFor(lang)
    for (section in sections) {
        val title = section.term?.name?.pick(lang)?.ifBlank { null } ?: section.termId
        add(TestsRow.Header("term:${section.termId}", title))
        section.tests
            .mapIndexed { index, test -> Triple(index, test, test.courseName.pick(lang)) }
            .sortedWith(
                Comparator<Triple<Int, CourseTest, String>> { a, b -> collator.compare(a.third, b.third) }
                    .thenBy { it.second.root.id }
                    .thenBy { it.first },
            )
            .forEach { (index, test, name) ->
                val key = courseKey(section.termId, test, index)
                val expandable = test.root.children.isNotEmpty()
                val expanded = expandable && isExpanded(key, true, toggled)
                add(
                    TestsRow.Course(
                        key,
                        test,
                        name,
                        test.root.name.pick(lang).ifBlank { null },
                        expanded,
                        expandable,
                    ),
                )
                if (expanded) addNodes(test.root.children, key, 0, toggled, lang)
            }
    }
}

private fun MutableList<TestsRow>.addNodes(
    nodes: List<TestNode>,
    parentKey: String,
    depth: Int,
    toggled: Set<String>,
    lang: String,
) {
    nodes.forEachIndexed { index, node ->
        val key = nodeKey(parentKey, node, index)
        val expandable = node.children.isNotEmpty()
        val expanded = expandable && isExpanded(key, false, toggled)
        add(TestsRow.Node(key, node, node.name.pick(lang), depth, expanded, expandable))
        if (expanded) addNodes(node.children, key, depth + 1, toggled, lang)
    }
}

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
