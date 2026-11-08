package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.data.LIVE_MAX_AGE_MS
import io.gitlab.voxix.libreusos.ui.ErrorKind
import io.gitlab.voxix.libreusos.ui.HomeBanner
import io.gitlab.voxix.libreusos.ui.HomeFailure
import io.gitlab.voxix.libreusos.ui.HomeMode
import io.gitlab.voxix.libreusos.ui.HomeStatus
import io.gitlab.voxix.libreusos.ui.HomeUiState
import io.gitlab.voxix.libreusos.ui.StampDay
import io.gitlab.voxix.libreusos.ui.failureOf
import io.gitlab.voxix.libreusos.ui.homeBanner
import io.gitlab.voxix.libreusos.ui.homeMode
import io.gitlab.voxix.libreusos.ui.homeStatus
import io.gitlab.voxix.libreusos.ui.needsRefresh
import io.gitlab.voxix.libreusos.ui.reduceCache
import io.gitlab.voxix.libreusos.ui.reduceFailure
import io.gitlab.voxix.libreusos.ui.reduceLive
import io.gitlab.voxix.libreusos.ui.stampDay
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.libreusos.core.UsosException
import java.time.LocalDate
import java.time.ZoneId

class HomeStateTest {
    private val today = LocalDate.of(2026, 11, 9)
    private val now = 1_800_000_000_000L
    private val offline = HomeFailure(ErrorKind.NETWORK, offline = true, atMs = now)
    private val serverError = HomeFailure(ErrorKind.NETWORK, offline = false, atMs = now)
    private val withData = HomeUiState(today = today, activities = emptyList(), dataAtSec = now / 1000 - 600, liveAtMs = now)

    @Test
    fun noDataYetIsASkeletonUntilItFails() {
        val empty = HomeUiState()
        assertEquals(HomeMode.Skeleton, homeMode(empty, false))
        assertEquals(HomeMode.NoCacheOffline, homeMode(empty.copy(failure = offline), false))
        assertEquals(HomeMode.NoCacheError, homeMode(empty.copy(failure = serverError), false))
        assertEquals(HomeMode.NoCacheExpired, homeMode(empty, true))
    }

    @Test
    fun cachedDataKeepsTheContentOnScreen() {
        assertEquals(HomeMode.Content, homeMode(withData.copy(failure = serverError), false))
        assertEquals(HomeMode.Content, homeMode(withData, true))
    }

    @Test
    fun oneBannerAtATime() {
        assertEquals(HomeBanner.None, homeBanner(withData, false))
        assertEquals(HomeBanner.Offline, homeBanner(withData.copy(failure = offline), false))
        assertEquals(HomeBanner.SyncError, homeBanner(withData.copy(failure = serverError), false))
        assertEquals(HomeBanner.Expired, homeBanner(withData.copy(failure = serverError), true))
        assertEquals(HomeBanner.None, homeBanner(HomeUiState(failure = offline), false))
    }

    @Test
    fun offlineIsSaidOnceByTheBanner() {
        assertEquals(HomeStatus.Hidden, homeStatus(withData.copy(failure = offline), false))
        assertEquals(HomeStatus.OfflineNoData, homeStatus(HomeUiState(failure = offline), false))
    }

    @Test
    fun statusFollowsTheState() {
        assertEquals(HomeStatus.Fetching, homeStatus(HomeUiState(), false))
        assertEquals(HomeStatus.Fetching, homeStatus(withData.copy(loading = true), false))
        assertEquals(HomeStatus.Synced, homeStatus(withData, false))
        assertEquals(HomeStatus.DataFromFailed, homeStatus(withData.copy(failure = serverError), false))
        assertEquals(HomeStatus.LastSync, homeStatus(withData, true))
        assertEquals(HomeStatus.FirstSyncFailed, homeStatus(HomeUiState(failure = serverError), false))
        assertTrue(HomeStatus.DataFromFailed.isFailure)
        assertFalse(HomeStatus.Synced.isFailure)
    }

    @Test
    fun refreshWhenNothingIsShownTheDayChangedOrDataIsStale() {
        assertTrue(needsRefresh(HomeUiState(), today, now))
        assertFalse(needsRefresh(withData, today, now + 1000))
        assertTrue(needsRefresh(withData, today.plusDays(1), now + 1000))
        assertTrue(needsRefresh(withData, today, now + LIVE_MAX_AGE_MS))
        assertTrue(needsRefresh(withData.copy(liveAtMs = null), today, now))
        assertFalse(needsRefresh(withData.copy(loading = true, liveAtMs = null), today, now))
    }

    @Test
    fun cacheFillsOnlyAnEmptyScreen() {
        val filled = reduceCache(HomeUiState(), emptyList(), 100)
        assertEquals(100L, filled.dataAtSec)
        assertEquals(withData, reduceCache(withData, emptyList(), 5))
    }

    @Test
    fun liveClearsTheFailure() {
        val s = reduceLive(withData.copy(failure = serverError), emptyList(), now + 5000)
        assertNull(s.failure)
        assertEquals(now + 5000, s.liveAtMs)
        assertEquals((now + 5000) / 1000, s.dataAtSec)
    }

    @Test
    fun failureKeepsTheShownData() {
        val s = reduceFailure(withData, serverError)
        assertEquals(withData.dataAtSec, s.dataAtSec)
        assertNull(s.liveAtMs)
        assertEquals(serverError, s.failure)
        val fromCache = reduceFailure(HomeUiState(), serverError, emptyList(), 42)
        assertEquals(42L, fromCache.dataAtSec)
        assertEquals(HomeMode.Content, homeMode(fromCache, false))
    }

    @Test
    fun failuresSeparateOfflineFromServerErrors() {
        assertTrue(failureOf(UsosException.Network("x"), online = false, nowMs = now).offline)
        assertFalse(failureOf(UsosException.Network("x"), online = true, nowMs = now).offline)
        assertFalse(failureOf(UsosException.Api(503u, "c", "Unavailable", emptyList(), ""), online = false, nowMs = now).offline)
        val api = failureOf(UsosException.Api(503u, "c", "Unavailable", emptyList(), ""), online = true, nowMs = now)
        assertEquals(503, api.status)
        assertEquals("Unavailable", api.reason)
    }

    @Test
    fun stampNamesTodayAndYesterday() {
        val zone = ZoneId.of("Europe/Warsaw")
        val noon = today.atTime(12, 0).atZone(zone).toEpochSecond()
        assertEquals(StampDay.Today, stampDay(noon, zone, today))
        assertEquals(StampDay.Yesterday, stampDay(noon - 86_400, zone, today))
        assertEquals(StampDay.Other, stampDay(noon - 3 * 86_400, zone, today))
    }
}
