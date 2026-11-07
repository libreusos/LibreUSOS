package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.data.clockTime
import io.gitlab.voxix.libreusos.data.pick
import io.gitlab.voxix.libreusos.data.todayIn
import org.junit.Assert.assertEquals
import org.junit.Test
import org.libreusos.core.LangText
import java.time.Clock
import java.time.Instant
import java.time.ZoneOffset

class TimeTest {
    @Test fun todayUsesTheInstallationZoneNotTheDeviceZone() {
        // 23:30 UTC on 4 Nov is already 00:30 on 5 Nov in Warsaw (CET, UTC+1).
        val clock = Clock.fixed(Instant.parse("2026-11-04T23:30:00Z"), ZoneOffset.UTC)
        assertEquals("2026-11-05", todayIn("Europe/Warsaw", clock).toString())
        assertEquals("2026-11-04", todayIn("UTC", clock).toString())
    }

    @Test fun pickFallsBackToTheOtherLanguage() {
        val text = LangText(pl = "Analiza", en = null)
        assertEquals("Analiza", text.pick("en"))
        assertEquals("Analiza", text.pick("pl"))
        assertEquals("Analysis", LangText(pl = " ", en = "Analysis").pick("pl"))
        assertEquals("", LangText(pl = null, en = null).pick("pl"))
    }

    @Test fun clockTimeShowsHoursAndMinutes() {
        assertEquals("08:30", clockTime("2026-11-04T08:30:00"))
        assertEquals("nonsense", clockTime("nonsense"))
    }

    @Test fun fetchedLabelUsesTheGivenZone() {
        val warsaw = java.time.ZoneId.of("Europe/Warsaw")
        val at = Instant.parse("2026-11-04T23:30:00Z").epochSecond // 00:30 on 5 Nov in Warsaw
        val en = java.util.Locale.ENGLISH
        assertEquals("00:30", io.gitlab.voxix.libreusos.data.fetchedLabel(at, warsaw, en, java.time.LocalDate.of(2026, 11, 5)))
        assertEquals("5 Nov 00:30", io.gitlab.voxix.libreusos.data.fetchedLabel(at, warsaw, en, java.time.LocalDate.of(2026, 11, 6)))
    }
}
