package io.github.voxix.libreusos

import io.github.voxix.libreusos.ui.ErrorKind
import io.github.voxix.libreusos.ui.GradesUiState
import io.github.voxix.libreusos.ui.TermSection
import io.github.voxix.libreusos.ui.isLive
import io.github.voxix.libreusos.ui.reduceCache
import io.github.voxix.libreusos.ui.reduceCacheFailure
import io.github.voxix.libreusos.ui.reduceFailure
import io.github.voxix.libreusos.ui.reduceLive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.libreusos.core.DataSource
import org.libreusos.core.TermGrades

class GradesStateTest {
    private val live = DataSource.Live
    private fun cached(at: Long) = DataSource.Cached(at)
    private fun section(id: String) =
        TermSection(id, null, TermGrades(id, emptyList(), null, null, DataSource.Live))

    private val old = listOf(section("OLD"))
    private val fresh = listOf(section("NEW"))

    @Test fun cacheThenLive() {
        val afterCache = reduceCache(GradesUiState(), old, listOf(cached(100), cached(50)))
        assertEquals(old, afterCache.sections)
        assertNull(afterCache.liveAtMs)
        assertEquals(50L, afterCache.dataAtSec)
        val afterLive = reduceLive(afterCache, fresh, listOf(live, live), nowMs = 5_000_000)
        assertEquals(fresh, afterLive.sections)
        assertEquals(5_000_000L, afterLive.liveAtMs)
        assertEquals(5_000L, afterLive.dataAtSec)
        assertFalse(afterLive.refreshFailed)
    }

    @Test fun cacheNeverReplacesDataOnScreen() {
        val shown = GradesUiState(sections = fresh, liveAtMs = 1)
        assertEquals(shown, reduceCache(shown, old, listOf(cached(1))))
    }

    @Test fun aFailedRefreshKeepsTheOldDataAndMarksItNotLive() {
        val shown = reduceLive(GradesUiState(), old, listOf(live), nowMs = 1_000)
        val failed = reduceFailure(shown, ErrorKind.NETWORK)
        assertEquals(old, failed.sections)
        assertNull(failed.liveAtMs)
        assertTrue(failed.refreshFailed)
        assertEquals(ErrorKind.NETWORK, failed.error)
        // With nothing on screen it is a plain error, not a failed refresh.
        val empty = reduceFailure(GradesUiState(), ErrorKind.NETWORK)
        assertNull(empty.sections)
        assertFalse(empty.refreshFailed)
    }

    @Test fun anOfflineFallbackIsNotLive() {
        val state = reduceLive(GradesUiState(), old, listOf(live, cached(700)), nowMs = 9_000_000)
        assertNull(state.liveAtMs)
        assertEquals(700L, state.dataAtSec)
        assertEquals(old, state.sections)
    }

    @Test fun anEmptyListIsLiveOnlyWhenTheTermsAreLive() {
        assertTrue(isLive(listOf(live)))
        assertFalse(isLive(listOf(cached(1))))
        assertFalse(isLive(emptyList()))
        assertNull(reduceLive(GradesUiState(), emptyList(), listOf(cached(30)), 9_000).liveAtMs)
        assertEquals(9_000L, reduceLive(GradesUiState(), emptyList(), listOf(live), 9_000).liveAtMs)
    }

    @Test fun aSuccessfulRefreshClearsTheFailureButKeepsACacheReadError() {
        val failed = reduceFailure(reduceLive(GradesUiState(), old, listOf(live), 1_000), ErrorKind.API)
        assertFalse(reduceLive(failed, fresh, listOf(live), 2_000).refreshFailed)
        assertEquals(ErrorKind.STORAGE, reduceLive(reduceCacheFailure(GradesUiState(), ErrorKind.STORAGE), old, listOf(live), 1).error)
    }
}
