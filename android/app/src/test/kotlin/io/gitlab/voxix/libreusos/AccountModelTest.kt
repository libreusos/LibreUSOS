package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.data.SYNC_INTERVAL_OPTIONS_HOURS
import io.gitlab.voxix.libreusos.data.nearestSyncInterval
import io.gitlab.voxix.libreusos.ui.MainTab
import io.gitlab.voxix.libreusos.ui.ROUTE_LICENCES
import io.gitlab.voxix.libreusos.ui.ROUTE_SETTINGS
import io.gitlab.voxix.libreusos.ui.hostOf
import io.gitlab.voxix.libreusos.ui.initialsOf
import io.gitlab.voxix.libreusos.ui.universityNameOf
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AccountModelTest {
    @Test
    fun initialsTakeFirstLettersOfBothNames() {
        assertEquals("ZW", initialsOf("Zofia", "Wróblewska"))
        assertEquals("ŁŻ", initialsOf("łukasz", "żak"))
        assertEquals("Z", initialsOf("Zofia", null))
        assertEquals("W", initialsOf("  ", "Wróblewska"))
        assertEquals("?", initialsOf(null, ""))
    }

    @Test
    fun initialsSkipLeadingPunctuationAndKeepSurrogatePairs() {
        assertEquals("AK", initialsOf("\"Anna\"", "-Kowalska"))
        assertEquals("𝐀B", initialsOf("𝐀bc", "Bee"))
    }

    @Test
    fun hostIsExtractedFromTheBaseUrl() {
        assertEquals("usosapi.polsl.pl", hostOf("https://usosapi.polsl.pl/"))
        assertEquals("usosapi.polsl.pl", hostOf("  https://usosapi.polsl.pl/services/  "))
        assertEquals("not a url", hostOf("not a url"))
    }

    @Test
    fun knownServersGetTheirUniversityName() {
        assertEquals("Politechnika Śląska", universityNameOf("https://usosapi.polsl.pl/"))
        assertEquals("Politechnika Śląska", universityNameOf("https://USOSAPI.POLSL.PL"))
        assertEquals("apps.example.edu", universityNameOf("https://apps.example.edu/usos/"))
    }

    @Test
    fun tabsRoundTripThroughTheirRoute() {
        MainTab.entries.forEach { assertEquals(it, MainTab.fromRoute(it.route)) }
        assertNull(MainTab.fromRoute(null))
        assertNull(MainTab.fromRoute("login"))
    }

    @Test
    fun bottomBarShowsOnTabsOnly() {
        assertTrue(MainTab.showsBottomBar("more"))
        assertTrue(MainTab.showsBottomBar("today"))
        assertFalse(MainTab.showsBottomBar(ROUTE_SETTINGS))
        assertFalse(MainTab.showsBottomBar(ROUTE_LICENCES))
        assertFalse(MainTab.showsBottomBar(null))
    }

    @Test
    fun fourTabsInDesignOrder() {
        assertEquals(listOf("today", "timetable", "grades", "more"), MainTab.entries.map { it.route })
    }

    @Test
    fun syncIntervalSnapsToTheNearestOption() {
        assertEquals(6L, nearestSyncInterval(6))
        assertEquals(1L, nearestSyncInterval(0))
        assertEquals(24L, nearestSyncInterval(100))
        assertEquals(12L, nearestSyncInterval(10))
        assertTrue(SYNC_INTERVAL_OPTIONS_HOURS.contains(6L))
    }
}
