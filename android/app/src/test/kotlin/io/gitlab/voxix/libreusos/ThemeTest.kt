package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.data.ThemeMode
import io.gitlab.voxix.libreusos.ui.theme.BrandDark
import io.gitlab.voxix.libreusos.ui.theme.BrandLight
import io.gitlab.voxix.libreusos.ui.theme.KIND_DELTA_E_MIN
import io.gitlab.voxix.libreusos.ui.theme.KindGroup
import io.gitlab.voxix.libreusos.ui.theme.deltaE76
import io.gitlab.voxix.libreusos.ui.theme.isDark
import io.gitlab.voxix.libreusos.ui.theme.kindCodeOf
import io.gitlab.voxix.libreusos.ui.theme.labNeedsOutline
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ThemeTest {
    @Test
    fun themeModeFallsBackToSystem() {
        assertEquals(ThemeMode.Dark, ThemeMode.fromStored("dark"))
        assertEquals(ThemeMode.Light, ThemeMode.fromStored("light"))
        assertEquals(ThemeMode.System, ThemeMode.fromStored("nonsense"))
        assertEquals(ThemeMode.System, ThemeMode.fromStored(null))
        ThemeMode.entries.forEach { assertEquals(it, ThemeMode.fromStored(it.stored)) }
    }

    @Test
    fun themeModeResolvesDarkness() {
        assertTrue(ThemeMode.System.isDark(systemDark = true))
        assertFalse(ThemeMode.System.isDark(systemDark = false))
        assertTrue(ThemeMode.Dark.isDark(systemDark = false))
        assertFalse(ThemeMode.Light.isDark(systemDark = true))
    }

    @Test
    fun knownClassTypesMapToCodes() {
        assertEquals("WYK" to KindGroup.Lecture, kindCodeOf("WYK", "wykład").let { it.code to it.group })
        assertEquals("LAB" to KindGroup.Lab, kindCodeOf("LAB", "laboratorium").let { it.code to it.group })
        assertEquals("ĆW" to KindGroup.Neutral, kindCodeOf("CW", "ćwiczenia").let { it.code to it.group })
        assertEquals("PRO", kindCodeOf("PRO", null).code)
        assertEquals("SEM", kindCodeOf("sem", null).code)
    }

    @Test
    fun examOverridesClassType() {
        val kind = kindCodeOf("WYK", "wykład", isExam = true)
        assertEquals("EGZ", kind.code)
        assertEquals(KindGroup.Exam, kind.group)
    }

    @Test
    fun unknownClassTypePrintsFirstThreeLettersOfName() {
        assertEquals("KON", kindCodeOf("KONS", "konwersatorium").code)
        assertEquals(KindGroup.Neutral, kindCodeOf("KONS", "konwersatorium").group)
        assertEquals("KON", kindCodeOf("KONS", null).code)
        assertEquals("—", kindCodeOf(null, null).code)
    }

    @Test
    fun deltaEIsZeroForSameColourAndLargeForBlackWhite() {
        assertEquals(0.0, deltaE76(0xFF3F5AA6.toInt(), 0xFF3F5AA6.toInt()), 1e-9)
        assertEquals(100.0, deltaE76(0xFF000000.toInt(), 0xFFFFFFFF.toInt()), 0.01)
    }

    @Test
    fun deltaEIsSymmetricAndIgnoresAlpha() {
        val a = 0xFFDCE1FF.toInt()
        val b = 0x80E6E1EC.toInt()
        assertEquals(deltaE76(a, b), deltaE76(b, a), 1e-9)
        assertEquals(deltaE76(a, 0xFFE6E1EC.toInt()), deltaE76(a, b), 1e-9)
    }

    @Test
    fun brandSchemesKeepLabApartFromOtherKinds() {
        // The brand fallback is designed so LAB needs no outline.
        assertFalse(labNeedsOutline(BrandLight))
        assertFalse(labNeedsOutline(BrandDark))
        assertTrue(KIND_DELTA_E_MIN > 0)
    }

    @Test
    fun labOutlineAppliesWhenContainersCollide() {
        val colliding = BrandLight.copy(tertiaryContainer = BrandLight.primaryContainer)
        assertTrue(labNeedsOutline(colliding))
        val neutral = BrandLight.copy(tertiaryContainer = BrandLight.surfaceContainerHighest)
        assertTrue(labNeedsOutline(neutral))
    }
}
