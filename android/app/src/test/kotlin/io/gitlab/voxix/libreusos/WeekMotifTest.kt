package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.ui.components.DefaultMotifPattern
import io.gitlab.voxix.libreusos.ui.components.MOTIF_COLUMNS
import io.gitlab.voxix.libreusos.ui.components.MOTIF_SLOTS
import org.junit.Assert.assertTrue
import org.junit.Test

class WeekMotifTest {
    @Test
    fun defaultPatternStaysInsideTheGrid() {
        assertTrue(DefaultMotifPattern.all { (col, slot) -> col in 0 until MOTIF_COLUMNS && slot in 0 until MOTIF_SLOTS })
    }

    @Test
    fun everyColumnHasAClassAndASpareSlot() {
        for (col in 0 until MOTIF_COLUMNS) {
            val n = DefaultMotifPattern.count { it.first == col }
            assertTrue("column $col", n in 1 until MOTIF_SLOTS)
        }
    }
}
