package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.ui.TestTermSection
import io.gitlab.voxix.libreusos.ui.TestsRow
import io.gitlab.voxix.libreusos.ui.buildTestRows
import io.gitlab.voxix.libreusos.ui.buildTestSections
import io.gitlab.voxix.libreusos.ui.formatPoints
import io.gitlab.voxix.libreusos.ui.isExpanded
import io.gitlab.voxix.libreusos.ui.liveTermIds
import io.gitlab.voxix.libreusos.ui.mergeTermTests
import io.gitlab.voxix.libreusos.ui.pruneToggled
import io.gitlab.voxix.libreusos.ui.testKeys
import io.gitlab.voxix.libreusos.ui.pointsLabel
import io.gitlab.voxix.libreusos.ui.toggleKey
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.libreusos.core.CourseTest
import org.libreusos.core.DataSource
import org.libreusos.core.LangText
import org.libreusos.core.Term
import org.libreusos.core.TermTests
import org.libreusos.core.TestNode
import org.libreusos.core.TestNodeKind
import java.util.Locale

class TestsModelTest {
    private fun term(id: String, start: String, end: String) = Term(
        id = id,
        name = LangText("Semestr $id", "Term $id"),
        startDate = start,
        endDate = end,
        finishDate = null,
        orderKey = null,
        isActive = null,
    )

    private fun node(
        id: String,
        kind: TestNodeKind,
        name: String? = id,
        points: Double? = null,
        max: Double? = null,
        hidden: Boolean = false,
        vararg children: TestNode,
    ) = TestNode(
        id = id,
        kind = kind,
        name = LangText(name, null),
        description = LangText(null, null),
        resultsVisible = null,
        hidden = hidden,
        points = points,
        pointsMax = max,
        pointsMin = null,
        grade = null,
        comment = null,
        lastChanged = null,
        children = children.toList(),
    )

    private fun test(
        courseId: String,
        name: String?,
        termId: String = "T",
        vararg top: TestNode,
        rootId: String = "R$courseId",
        rootHidden: Boolean = false,
        limited: Boolean = false,
    ) = CourseTest(
        courseId = courseId,
        courseName = LangText(name, null),
        termId = termId,
        limitedToGroups = limited,
        root = node(rootId, TestNodeKind.ROOT, name = null, points = 7.5, max = 20.0, hidden = rootHidden, children = top),
    )

    private fun termTests(id: String, vararg tests: CourseTest) =
        TermTests(id, tests.toList(), emptyList(), DataSource.Live)

    @Test fun sectionsDropEmptyTermsAndSortNewestFirst() {
        val terms = listOf(
            term("A", "2024-10-01", "2025-02-28"),
            term("B", "2025-10-01", "2026-02-28"),
            term("C", "2026-03-01", "2026-09-30"),
        )
        val results = listOf(
            termTests("A", test("1", "x")),
            termTests("B"),
            termTests("C", test("1", "x")),
            termTests("A", test("2", "y")),
            termTests("Z", test("3", "z")),
        )
        val sections = buildTestSections(terms, results)
        // B has no tests, the second A is dropped, the unknown Z comes last.
        assertEquals(listOf("C", "A", "Z"), sections.map { it.termId })
        assertEquals(listOf("1"), sections[1].tests.map { it.courseId })
        assertNull(sections[2].term)
    }

    @Test fun coursesSortWithThePolishCollator() {
        val tests = listOf(test("1", "Zeszyt"), test("2", "Łacina"), test("3", "Logika"), test("4", "Algebra"))
        val rows = buildTestRows(listOf(TestTermSection("T", null, tests)), emptySet(), "pl")
        val names = rows.filterIsInstance<TestsRow.Course>().map { it.name }
        // A plain code-point compare would put the Ł after Z.
        assertEquals(listOf("Algebra", "Logika", "Łacina", "Zeszyt"), names)
        assertEquals("T", (rows.first() as TestsRow.Header).title.removePrefix("Semestr "))
    }

    private val tree = test(
        "1",
        "Analiza",
        "T",
        node("F1", TestNodeKind.FOLDER, points = 4.0, max = 10.0, children = arrayOf(
            node("K1", TestNodeKind.TASK, points = 4.0, max = 10.0),
            node("F2", TestNodeKind.FOLDER, children = arrayOf(node("K2", TestNodeKind.TASK, max = 5.0))),
        )),
        node("G1", TestNodeKind.GRADE, name = null),
    )

    private fun rowsOf(toggled: Set<String>) = buildTestRows(listOf(TestTermSection("T", null, listOf(tree))), toggled, "en")

    @Test fun aCourseOpensByDefaultAndFoldersStartClosed() {
        val rows = rowsOf(emptySet())
        assertEquals(listOf("term:T", "course:T:1:R1#0", "course:T:1:R1#0/F1#0", "course:T:1:R1#0/G1#1"), rows.map { it.key })
        val course = rows[1] as TestsRow.Course
        assertTrue(course.expanded)
        assertTrue(course.expandable)
        val folder = rows[2] as TestsRow.Node
        assertTrue(folder.expandable)
        assertFalse(folder.expanded)
        assertEquals(0, folder.depth)
        val leaf = rows[3] as TestsRow.Node
        assertFalse(leaf.expandable)
        assertEquals("", leaf.name)
    }

    @Test fun openingFoldersFlattensTheTreeWithDepths() {
        val f1 = "course:T:1:R1#0/F1#0"
        val rows = rowsOf(setOf(f1, "$f1/F2#1"))
        val nodes = rows.filterIsInstance<TestsRow.Node>()
        assertEquals(listOf("F1", "K1", "F2", "K2", "G1"), nodes.map { it.node.id })
        assertEquals(listOf(0, 1, 1, 2, 0), nodes.map { it.depth })
        assertTrue(nodes[0].expanded)
        assertTrue(nodes[2].expanded)
        // Nodes under a closed parent stay hidden even when they were toggled themselves.
        val inner = rowsOf(setOf("$f1/F2#1")).filterIsInstance<TestsRow.Node>()
        assertEquals(listOf("F1", "G1"), inner.map { it.node.id })
    }

    @Test fun closingACourseHidesItsTree() {
        val rows = rowsOf(setOf("course:T:1:R1#0"))
        assertEquals(2, rows.size)
        assertFalse((rows[1] as TestsRow.Course).expanded)
    }

    @Test fun aCourseWithoutNodesIsNotExpandable() {
        val rows = buildTestRows(listOf(TestTermSection("T", null, listOf(test("1", "x")))), emptySet(), "en")
        val course = rows[1] as TestsRow.Course
        assertFalse(course.expandable)
        assertFalse(course.expanded)
        assertEquals(2, rows.size)
    }

    @Test fun toggleFlipsAgainstTheDefault() {
        assertTrue(isExpanded("k", true, emptySet()))
        assertFalse(isExpanded("k", true, setOf("k")))
        assertFalse(isExpanded("k", false, emptySet()))
        assertTrue(isExpanded("k", false, setOf("k")))
        assertEquals(setOf("a", "k"), toggleKey(setOf("a"), "k"))
        assertEquals(setOf("a"), toggleKey(setOf("a", "k"), "k"))
    }

    @Test fun pointsAreShownAsGivenByTheCore() {
        val en = Locale.ENGLISH
        assertEquals("7.5", formatPoints(7.5, en))
        assertEquals("10", formatPoints(10.0, en))
        assertEquals("7,25", formatPoints(7.25, Locale.forLanguageTag("pl")))
        assertEquals("1000", formatPoints(1000.0, en))
        assertEquals("4 / 10", pointsLabel(4.0, 10.0, en))
        assertEquals("4", pointsLabel(4.0, null, en))
        assertEquals("– / 5", pointsLabel(null, 5.0, en))
        assertNull(pointsLabel(null, null, en))
        // The course row keeps the core's root totals; nothing is summed from the children.
        val course = rowsOf(emptySet())[1] as TestsRow.Course
        assertEquals(7.5, course.test.root.points)
        assertEquals(20.0, course.test.root.pointsMax)
    }

    @Test fun pointsRoundHalfUpAndNeverShowNegativeZero() {
        val en = Locale.ENGLISH
        assertEquals("0.13", formatPoints(0.125, en))
        assertEquals("2.68", formatPoints(2.675, en))
        assertEquals("0", formatPoints(-0.0, en))
        assertEquals("0", formatPoints(-0.001, en))
        assertEquals("0", formatPoints(0.004, en))
        assertEquals("-1.5", formatPoints(-1.5, en))
        assertEquals("\u2013", formatPoints(Double.NaN, en))
        assertEquals("0 / 5", pointsLabel(-0.0, 5.0, en))
    }

    @Test fun equalCourseNamesFallBackToTheRootIdThenTheirOrder() {
        val tests = listOf(
            test("1", "Same", rootId = "B"),
            test("2", "Same", rootId = "A"),
            test("3", "Same", rootId = "A"),
        )
        val rows = buildTestRows(listOf(TestTermSection("T", null, tests)), emptySet(), "en").filterIsInstance<TestsRow.Course>()
        assertEquals(listOf("2", "3", "1"), rows.map { it.test.courseId })
    }

    @Test fun nodesWithoutANameGetAnEmptyName() {
        val rows = buildTestRows(
            listOf(TestTermSection("T", null, listOf(test("1", null, "T", node("X", TestNodeKind.TASK, name = null))))),
            emptySet(),
            "en",
        )
        assertEquals("", (rows[1] as TestsRow.Course).name)
        assertEquals("", (rows[2] as TestsRow.Node).name)
    }

    @Test fun hiddenAndLimitedFlagsReachTheRows() {
        val tests = listOf(
            test("1", "A", "T", node("H", TestNodeKind.TASK, hidden = true, points = 1.0, max = 2.0), rootHidden = true, limited = true),
        )
        val rows = buildTestRows(listOf(TestTermSection("T", null, tests)), emptySet(), "en")
        val course = rows[1] as TestsRow.Course
        assertTrue(course.test.root.hidden)
        assertTrue(course.test.limitedToGroups)
        assertTrue((rows[2] as TestsRow.Node).node.hidden)
    }

    @Test fun duplicateIdsStillGiveUniqueKeys() {
        val dup = test("1", "A", "T", node("N", TestNodeKind.TASK), node("N", TestNodeKind.TASK), rootId = "R")
        val sections = listOf(TestTermSection("T", null, listOf(dup, dup)))
        val rows = buildTestRows(sections, emptySet(), "en")
        assertEquals(rows.size, rows.map { it.key }.toSet().size)
        assertEquals(6, testKeys(sections).size)
    }

    @Test fun pruningKeepsOnlyKeysOfTheCurrentTree() {
        val sections = listOf(TestTermSection("T", null, listOf(tree)))
        val live = "course:T:1:R1#0/F1#0"
        assertEquals(setOf(live), pruneToggled(setOf(live, "course:T:1:R1#0/GONE#0", "course:X:9:R#0"), sections))
        assertEquals(emptySet<String>(), pruneToggled(emptySet(), sections))
    }

    @Test fun onlyTheTwoNewestTermsAndUncachedOnesAreFetchedLive() {
        val terms = listOf(
            term("A", "2023-10-01", "2024-02-28"),
            term("B", "2024-10-01", "2025-02-28"),
            term("C", "2025-10-01", "2026-02-28"),
            term("D", "2026-03-01", "2026-09-30"),
        )
        assertEquals(listOf("C", "D"), liveTermIds(terms, setOf("A", "B", "C", "D")))
        assertEquals(listOf("A", "C", "D"), liveTermIds(terms, setOf("B", "C", "D")))
        assertEquals(listOf("A", "B", "C", "D"), liveTermIds(terms, emptySet()))
        assertEquals(emptyList<String>(), liveTermIds(emptyList(), emptySet()))
    }

    @Test fun liveResultsReplaceCachedOnesForTheSameTerms() {
        val cached = listOf(termTests("A", test("1", "old")), termTests("B", test("1", "old")))
        val live = listOf(termTests("B", test("1", "new")))
        val merged = mergeTermTests(cached, live, listOf("B"))
        assertEquals(listOf("A", "B"), merged.map { it.termId })
        assertEquals("new", merged[1].tests[0].courseName.pl)
        // A term that was asked for but failed keeps its cache out of the merge only if it has none.
        assertEquals(listOf("A"), mergeTermTests(cached.take(1), emptyList(), listOf("B")).map { it.termId })
    }
}
