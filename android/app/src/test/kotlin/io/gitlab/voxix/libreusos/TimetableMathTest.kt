package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.data.dateOfPage
import io.gitlab.voxix.libreusos.data.isStale
import io.gitlab.voxix.libreusos.data.mondayOf
import io.gitlab.voxix.libreusos.data.pageOf
import io.gitlab.voxix.libreusos.data.shortBuildingLabel
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.DayOfWeek
import java.time.LocalDate

class TimetableMathTest {
    @Test fun pagesAndDatesRoundTrip() {
        val day = LocalDate.of(2026, 11, 5)
        assertEquals(day, dateOfPage(pageOf(day)))
        assertEquals(DayOfWeek.MONDAY, dateOfPage(0).dayOfWeek)
    }

    @Test fun weekStartsOnMonday() {
        assertEquals(LocalDate.of(2026, 11, 2), mondayOf(LocalDate.of(2026, 11, 5)))
        assertEquals(LocalDate.of(2026, 11, 2), mondayOf(LocalDate.of(2026, 11, 2)))
        assertEquals(LocalDate.of(2026, 11, 2), mondayOf(LocalDate.of(2026, 11, 8)))
    }

    @Test fun onlyShortBuildingIdsAreLabels() {
        assertEquals("WAEI", shortBuildingLabel("WAEI"))
        assertNull(shortBuildingLabel("BU/0016"))
        assertNull(shortBuildingLabel(null))
        assertNull(shortBuildingLabel(" "))
    }

    @Test fun staleAfterFifteenMinutes() {
        assertTrue(isStale(null, 1_000))
        assertFalse(isStale(1_000, 1_000 + 14 * 60_000))
        assertTrue(isStale(1_000, 1_000 + 15 * 60_000))
    }
}
