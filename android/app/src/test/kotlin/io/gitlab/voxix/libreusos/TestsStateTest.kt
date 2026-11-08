package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.ui.ErrorKind
import io.gitlab.voxix.libreusos.ui.HomeFailure
import io.gitlab.voxix.libreusos.ui.HomeMode
import io.gitlab.voxix.libreusos.ui.TermList
import io.gitlab.voxix.libreusos.ui.TestTermSection
import io.gitlab.voxix.libreusos.ui.TestsUiState
import io.gitlab.voxix.libreusos.ui.asHomeState
import io.gitlab.voxix.libreusos.ui.homeMode
import io.gitlab.voxix.libreusos.ui.reduceTestsCache
import io.gitlab.voxix.libreusos.ui.reduceTestsFailure
import io.gitlab.voxix.libreusos.ui.reduceTestsLive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import org.libreusos.core.DataSource

class TestsStateTest {
    private val live = DataSource.Live
    private fun cached(at: Long) = DataSource.Cached(at)
    private fun section(id: String) = TestTermSection(id, null, emptyList())

    private val old = listOf(section("OLD"))
    private val fresh = listOf(section("NEW"))
    private val terms = TermList(emptyList(), "NEW")
    private val offline = HomeFailure(ErrorKind.NETWORK, offline = true, atMs = 1)
    private val failure = HomeFailure(ErrorKind.API, offline = false, atMs = 2)

    @Test fun cacheThenLive() {
        val afterCache = reduceTestsCache(TestsUiState(), old, terms, listOf(cached(100), cached(50)))
        assertEquals(old, afterCache.sections)
        assertNull(afterCache.liveAtMs)
        assertEquals(50L, afterCache.dataAtSec)
        val afterLive = reduceTestsLive(afterCache, fresh, terms, listOf(live, live), skipped = 2, failedTerms = 0, nowMs = 5_000_000, notLive = offline)
        assertEquals(fresh, afterLive.sections)
        assertEquals(5_000_000L, afterLive.liveAtMs)
        assertEquals(5_000L, afterLive.dataAtSec)
        assertEquals(2, afterLive.skipped)
        assertNull(afterLive.failure)
    }

    @Test fun cacheNeverReplacesDataOnScreen() {
        val shown = TestsUiState(sections = fresh, liveAtMs = 1)
        assertEquals(shown, reduceTestsCache(shown, old, terms, listOf(cached(1))))
    }

    @Test fun aFailedRefreshKeepsTheOldDataAndMarksItNotLive() {
        val shown = reduceTestsLive(TestsUiState(), old, terms, listOf(live), 0, 0, 1_000, offline)
        val failed = reduceTestsFailure(shown, failure)
        assertEquals(old, failed.sections)
        assertNull(failed.liveAtMs)
        assertEquals(failure, failed.failure)
        // With nothing on screen the sync states show a full-screen failure instead.
        val empty = reduceTestsFailure(TestsUiState(), offline)
        assertNull(empty.sections)
        assertEquals(HomeMode.NoCacheOffline, homeMode(empty.asHomeState(), expired = false))
    }

    @Test fun anOfflineFallbackIsNotLiveAndSaysWhy() {
        val state = reduceTestsLive(TestsUiState(), old, terms, listOf(live, cached(700)), 0, 0, 9_000_000, offline)
        assertNull(state.liveAtMs)
        assertEquals(700L, state.dataAtSec)
        assertEquals(offline, state.failure)
    }

    @Test fun anOfflineFallbackKeepsThePreviousSkippedCount() {
        val first = reduceTestsLive(TestsUiState(), old, terms, listOf(live), 3, 0, 1, offline)
        val fallback = reduceTestsLive(first, old, terms, listOf(cached(5)), 0, 0, 2, offline)
        assertEquals(3, fallback.skipped)
        assertEquals(1, reduceTestsLive(fallback, old, terms, listOf(live), 1, 0, 3, offline).skipped)
    }

    @Test fun aTermThatFailedMeansNotFullyLive() {
        val state = reduceTestsLive(TestsUiState(), old, terms, listOf(live), 0, 2, 1, failure)
        assertNull(state.liveAtMs)
        assertEquals(failure, state.failure)
        assertNull(reduceTestsLive(state, old, terms, listOf(live), 0, 0, 2, failure).failure)
    }
}
