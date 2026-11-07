package io.github.voxix.libreusos

import io.github.voxix.libreusos.ui.ErrorKind
import io.github.voxix.libreusos.ui.TestTermSection
import io.github.voxix.libreusos.ui.TestsUiState
import io.github.voxix.libreusos.ui.reduceTestsCache
import io.github.voxix.libreusos.ui.reduceTestsCacheFailure
import io.github.voxix.libreusos.ui.reduceTestsFailure
import io.github.voxix.libreusos.ui.reduceTestsLive
import io.github.voxix.libreusos.ui.reduceToggle
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.libreusos.core.DataSource

class TestsStateTest {
    private val live = DataSource.Live
    private fun cached(at: Long) = DataSource.Cached(at)
    private fun section(id: String) = TestTermSection(id, null, emptyList())

    private val old = listOf(section("OLD"))
    private val fresh = listOf(section("NEW"))

    @Test fun cacheThenLive() {
        val afterCache = reduceTestsCache(TestsUiState(), old, listOf(cached(100), cached(50)))
        assertEquals(old, afterCache.sections)
        assertNull(afterCache.liveAtMs)
        assertEquals(50L, afterCache.dataAtSec)
        val afterLive = reduceTestsLive(afterCache, fresh, listOf(live, live), skipped = 2, failedTerms = 0, nowMs = 5_000_000)
        assertEquals(fresh, afterLive.sections)
        assertEquals(5_000_000L, afterLive.liveAtMs)
        assertEquals(5_000L, afterLive.dataAtSec)
        assertEquals(2, afterLive.skipped)
        assertFalse(afterLive.refreshFailed)
    }

    @Test fun cacheNeverReplacesDataOnScreen() {
        val shown = TestsUiState(sections = fresh, liveAtMs = 1)
        assertEquals(shown, reduceTestsCache(shown, old, listOf(cached(1))))
    }

    @Test fun aFailedRefreshKeepsTheOldDataAndMarksItNotLive() {
        val shown = reduceTestsLive(TestsUiState(), old, listOf(live), skipped = 0, failedTerms = 0, nowMs = 1_000)
        val failed = reduceTestsFailure(shown, ErrorKind.NETWORK)
        assertEquals(old, failed.sections)
        assertNull(failed.liveAtMs)
        assertTrue(failed.refreshFailed)
        assertEquals(ErrorKind.NETWORK, failed.error)
        // With nothing on screen it is a plain error, not a failed refresh.
        val empty = reduceTestsFailure(TestsUiState(), ErrorKind.NETWORK)
        assertNull(empty.sections)
        assertFalse(empty.refreshFailed)
    }

    @Test fun anOfflineFallbackIsNotLive() {
        val state = reduceTestsLive(TestsUiState(), old, listOf(live, cached(700)), skipped = 0, failedTerms = 0, nowMs = 9_000_000)
        assertNull(state.liveAtMs)
        assertEquals(700L, state.dataAtSec)
    }

    @Test fun aCacheReadFailureKeepsTheScreenAndRecordsTheError() {
        val state = reduceTestsCacheFailure(TestsUiState(), ErrorKind.STORAGE)
        assertEquals(ErrorKind.STORAGE, state.error)
        assertNull(state.sections)
    }

    @Test fun toggleFlipsAndSurvivesACacheAnswer() {
        val opened = reduceToggle(reduceTestsCache(TestsUiState(), old, listOf(cached(1))), "k")
        assertEquals(setOf("k"), opened.toggled)
        // A cache answer never touches the choices; only a live tree prunes them.
        assertEquals(setOf("k"), reduceTestsCache(opened, fresh, listOf(cached(2))).toggled)
        assertEquals(emptySet<String>(), reduceToggle(opened, "k").toggled)
    }

    @Test fun anOfflineFallbackKeepsThePreviousSkippedCount() {
        val first = reduceTestsLive(TestsUiState(), old, listOf(live), skipped = 3, failedTerms = 0, nowMs = 1)
        val offline = reduceTestsLive(first, old, listOf(cached(5)), skipped = 0, failedTerms = 0, nowMs = 2)
        assertEquals(3, offline.skipped)
        val again = reduceTestsLive(offline, old, listOf(live), skipped = 1, failedTerms = 0, nowMs = 3)
        assertEquals(1, again.skipped)
    }

    @Test fun failedTermsAreRecordedAndMeanNotFullyLive() {
        val state = reduceTestsLive(TestsUiState(), old, listOf(live), skipped = 0, failedTerms = 2, nowMs = 1)
        assertEquals(2, state.failedTerms)
        assertNull(state.liveAtMs)
        assertEquals(0, reduceTestsLive(state, old, listOf(live), skipped = 0, failedTerms = 0, nowMs = 2).failedTerms)
    }

    @Test fun aLiveResultDropsTogglesOfRowsThatAreGone() {
        val tests = listOf(
            org.libreusos.core.CourseTest(
                "1", org.libreusos.core.LangText("A", null), "NEW", false,
                org.libreusos.core.TestNode(
                    "R", org.libreusos.core.TestNodeKind.ROOT, org.libreusos.core.LangText(null, null),
                    org.libreusos.core.LangText(null, null), null, false, null, null, null, null, null, null,
                    listOf(
                        org.libreusos.core.TestNode(
                            "F", org.libreusos.core.TestNodeKind.FOLDER, org.libreusos.core.LangText("F", null),
                            org.libreusos.core.LangText(null, null), null, false, null, null, null, null, null, null, emptyList(),
                        ),
                    ),
                ),
            ),
        )
        val sections = listOf(TestTermSection("NEW", null, tests))
        val kept = "course:NEW:1:R#0/F#0"
        val state = reduceToggle(reduceToggle(TestsUiState(), kept), "course:OLD:1:R#0")
        val reloaded = reduceTestsLive(state, sections, listOf(live), skipped = 0, failedTerms = 0, nowMs = 1)
        assertEquals(setOf(kept), reloaded.toggled)
    }
}
