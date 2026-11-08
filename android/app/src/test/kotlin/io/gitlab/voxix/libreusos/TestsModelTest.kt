package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.ui.TestTermSection
import io.gitlab.voxix.libreusos.ui.Unavailable
import io.gitlab.voxix.libreusos.ui.buildTestCards
import io.gitlab.voxix.libreusos.ui.buildTestSections
import io.gitlab.voxix.libreusos.ui.formatPoints
import io.gitlab.voxix.libreusos.ui.liveTermIds
import io.gitlab.voxix.libreusos.ui.mergeTermTests
import io.gitlab.voxix.libreusos.ui.progressOf
import io.gitlab.voxix.libreusos.ui.pointsLabel
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
import java.time.LocalDate
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
        visible: Boolean? = null,
        changed: String? = null,
        vararg children: TestNode,
    ) = TestNode(
        id = id,
        kind = kind,
        name = LangText(name, null),
        description = LangText(null, null),
        resultsVisible = visible,
        hidden = hidden,
        points = points,
        pointsMax = max,
        pointsMin = null,
        grade = null,
        comment = null,
        lastChanged = changed,
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

    private fun cards(vararg tests: CourseTest, lang: String = "en") =
        buildTestCards(TestTermSection("T", null, tests.toList()), lang, Locale.ENGLISH)

    @Test fun cardsSortWithThePolishCollator() {
        val names = cards(test("1", "Zeszyt"), test("2", "Łacina"), test("3", "Logika"), test("4", "Algebra"), lang = "pl").map { it.courseName }
        // A plain code-point compare would put the Ł after Z.
        assertEquals(listOf("Algebra", "Logika", "Łacina", "Zeszyt"), names)
        assertTrue(buildTestCards(null, "pl", Locale.ENGLISH).isEmpty())
    }

    @Test fun equalCourseNamesFallBackToTheRootIdThenTheirOrder() {
        val ids = listOf(test("1", "Same", rootId = "B"), test("2", "Same", rootId = "A"), test("3", "Same", rootId = "A"))
        assertEquals(listOf("2", "3", "1"), cards(*ids.toTypedArray()).map { it.key.split(":")[1] })
    }

    @Test fun aCardKeepsTheCoreTotalsAndTheFirstLevelCells() {
        val card = cards(
            test(
                "1", "Analiza", "T",
                node("F1", TestNodeKind.FOLDER, name = "Part", points = 4.0, max = 10.0, changed = "2026-06-01T10:00:00"),
                node("G1", TestNodeKind.GRADE, name = "Grade"),
            ),
        ).single()
        assertEquals("7.5 / 20", card.points)
        assertEquals(0.375f, card.progress!!, 0.0001f)
        assertEquals(listOf("Part", "Grade"), card.cells.map { it.label })
        assertEquals(listOf("4 / 10", ""), card.cells.map { it.value })
        assertNull(card.unavailable)
        // Totals are the core's: nothing is summed from the cells.
        assertEquals("Analiza", card.courseName)
    }

    @Test fun aGradeCellShowsItsSymbol() {
        val grade = node("G", TestNodeKind.GRADE, name = "Final").copy(grade = org.libreusos.core.NodeGrade(symbol = "4,5", description = LangText(null, null), passes = true, value = 4.5))
        val card = cards(test("1", "A", "T", grade)).single()
        assertEquals("4,5", card.cells.single().value)
    }

    @Test fun hiddenAndGroupRestrictedTestsAreUnavailable() {
        val hidden = cards(test("1", "A", "T", node("H", TestNodeKind.TASK, points = 1.0, max = 2.0), rootHidden = true)).single()
        assertEquals(Unavailable.Hidden, hidden.unavailable)
        assertTrue(hidden.cells.isEmpty())
        val groups = buildTestCards(
            TestTermSection("T", null, listOf(test("2", "B", limited = true).let { it.copy(root = it.root.copy(points = null)) })),
            "en",
            Locale.ENGLISH,
        ).single()
        assertEquals(Unavailable.Groups, groups.unavailable)
        val limitedWithPoints = cards(test("3", "C", limited = true)).single()
        assertNull(limitedWithPoints.unavailable)
        val notVisible = cards(test("4", "D").let { it.copy(root = it.root.copy(resultsVisible = false)) }).single()
        assertEquals(Unavailable.Hidden, notVisible.unavailable)
    }

    @Test fun aHiddenCellKeepsItsLabelButNoValue() {
        val card = cards(test("1", "A", "T", node("H", TestNodeKind.TASK, name = "Quiz", points = 1.0, max = 2.0, hidden = true))).single()
        assertTrue(card.cells.single().hidden)
        assertEquals("", card.cells.single().value)
    }

    @Test fun theChangedDateComesFromTheRootTimestamp() {
        val t = test("1", "A")
        assertEquals(LocalDate.of(2026, 6, 1), cards(t.copy(root = t.root.copy(lastChanged = "2026-06-01T10:00:00"))).single().changed)
        assertNull(cards(t.copy(root = t.root.copy(lastChanged = "oops"))).single().changed)
    }

    @Test fun progressStaysWithinZeroAndOne() {
        assertEquals(0.5f, progressOf(5.0, 10.0)!!, 0.0001f)
        assertEquals(1f, progressOf(12.0, 10.0)!!, 0.0001f)
        assertEquals(0f, progressOf(-1.0, 10.0)!!, 0.0001f)
        assertNull(progressOf(null, 10.0))
        assertNull(progressOf(5.0, null))
        assertNull(progressOf(5.0, 0.0))
        assertNull(progressOf(Double.NaN, 10.0))
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
