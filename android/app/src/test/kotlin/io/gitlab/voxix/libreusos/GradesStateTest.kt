package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.ui.ErrorKind
import io.gitlab.voxix.libreusos.ui.GradesUiState
import io.gitlab.voxix.libreusos.ui.HomeBanner
import io.gitlab.voxix.libreusos.ui.HomeFailure
import io.gitlab.voxix.libreusos.ui.HomeMode
import io.gitlab.voxix.libreusos.ui.TermList
import io.gitlab.voxix.libreusos.ui.TermSection
import io.gitlab.voxix.libreusos.ui.asHomeState
import io.gitlab.voxix.libreusos.ui.homeBanner
import io.gitlab.voxix.libreusos.ui.homeMode
import io.gitlab.voxix.libreusos.ui.isLive
import io.gitlab.voxix.libreusos.ui.reduceCache
import io.gitlab.voxix.libreusos.ui.reduceFailure
import io.gitlab.voxix.libreusos.ui.reduceLive
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
    private val terms = TermList(emptyList(), "NEW")
    private val offline = HomeFailure(ErrorKind.NETWORK, offline = true, atMs = 1)
    private val failure = HomeFailure(ErrorKind.API, offline = false, atMs = 2)

    @Test fun cacheThenLive() {
        val afterCache = reduceCache(GradesUiState(), old, terms, listOf(cached(100), cached(50)))
        assertEquals(old, afterCache.sections)
        assertNull(afterCache.liveAtMs)
        assertEquals(50L, afterCache.dataAtSec)
        val afterLive = reduceLive(afterCache, fresh, terms, listOf(live, live), 5_000_000, offline)
        assertEquals(fresh, afterLive.sections)
        assertEquals(5_000_000L, afterLive.liveAtMs)
        assertEquals(5_000L, afterLive.dataAtSec)
        assertNull(afterLive.failure)
    }

    @Test fun cacheNeverReplacesDataOnScreen() {
        val shown = GradesUiState(sections = fresh, liveAtMs = 1)
        assertEquals(shown, reduceCache(shown, old, terms, listOf(cached(1))))
    }

    @Test fun aFailedRefreshKeepsTheOldDataAndMarksItNotLive() {
        val shown = reduceLive(GradesUiState(), old, terms, listOf(live), 1_000, offline)
        val failed = reduceFailure(shown, failure)
        assertEquals(old, failed.sections)
        assertNull(failed.liveAtMs)
        assertEquals(failure, failed.failure)
    }

    @Test fun anOfflineFallbackIsNotLiveAndSaysWhy() {
        val state = reduceLive(GradesUiState(), old, terms, listOf(live, cached(700)), 9_000_000, offline)
        assertNull(state.liveAtMs)
        assertEquals(700L, state.dataAtSec)
        assertEquals(old, state.sections)
        assertEquals(offline, state.failure)
    }

    @Test fun anEmptyListIsLiveOnlyWhenTheTermsAreLive() {
        assertTrue(isLive(listOf(live)))
        assertFalse(isLive(listOf(cached(1))))
        assertFalse(isLive(emptyList()))
        assertNull(reduceLive(GradesUiState(), emptyList(), terms, listOf(cached(30)), 9_000, offline).liveAtMs)
        assertEquals(9_000L, reduceLive(GradesUiState(), emptyList(), terms, listOf(live), 9_000, offline).liveAtMs)
    }

    @Test fun aSuccessfulRefreshClearsTheFailure() {
        val failed = reduceFailure(reduceLive(GradesUiState(), old, terms, listOf(live), 1_000, offline), failure)
        assertNull(reduceLive(failed, fresh, terms, listOf(live), 2_000, offline).failure)
    }

    @Test fun theSyncStatesFollowTheSameRulesAsToday() {
        // Nothing known and a failure: a full-screen state. With data: a banner over it.
        assertEquals(HomeMode.NoCacheOffline, homeMode(reduceFailure(GradesUiState(), offline).asHomeState(), expired = false))
        assertEquals(HomeMode.Skeleton, homeMode(GradesUiState().asHomeState(), expired = false))
        val shown = reduceFailure(reduceLive(GradesUiState(), old, terms, listOf(live), 1_000, offline), failure).asHomeState()
        assertEquals(HomeMode.Content, homeMode(shown, expired = false))
        assertEquals(HomeBanner.SyncError, homeBanner(shown, expired = false))
        // An empty list of grades is still data.
        assertEquals(HomeMode.Content, homeMode(reduceLive(GradesUiState(), emptyList(), terms, listOf(live), 1, offline).asHomeState(), false))
    }
}
