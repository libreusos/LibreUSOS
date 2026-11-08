package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.ui.TermList
import io.gitlab.voxix.libreusos.ui.TermOption
import io.gitlab.voxix.libreusos.ui.TermWord
import io.gitlab.voxix.libreusos.ui.previousTermId
import io.gitlab.voxix.libreusos.ui.selectedTermId
import io.gitlab.voxix.libreusos.ui.termOptions
import io.gitlab.voxix.libreusos.ui.termWord
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import org.libreusos.core.LangText
import org.libreusos.core.Term
import java.time.LocalDate

class TermPickerTest {
    private fun term(id: String, start: String, end: String) =
        Term(id, LangText(id, id), start, end, null, null, null)

    private val a = term("2024/25-Z", "2024-10-01", "2025-02-16")
    private val b = term("2025/26-L", "2025-02-17", "2025-09-30")
    private val c = term("2026/27-Z", "2026-10-01", "2027-02-14")
    private val d = term("2027/28-L", "2027-02-15", "2027-09-30")
    private val today = LocalDate.of(2026, 11, 8)

    @Test fun aTermIsOngoingEndedOrNamedBySeason() {
        assertEquals(TermWord.Ongoing, termWord(c, "2026/27-Z", today))
        assertEquals(TermWord.Ended, termWord(b, "2026/27-Z", today))
        assertEquals(TermWord.Summer, termWord(d, "2026/27-Z", today))
        assertEquals(TermWord.Winter, termWord(term("2028/29-Z", "2028-10-01", "2029-02-14"), null, today))
        assertEquals(TermWord.None, termWord(term("X", "2030-01-01", "2030-02-01"), null, today))
    }

    @Test fun optionsListTheCurrentTermAndTheOnesWithDataNewestFirst() {
        val list = TermList(listOf(a, b, c, d), "2026/27-Z")
        val options = termOptions(list, setOf("2024/25-Z", "2025/26-L", "GONE"), today)
        assertEquals(listOf("2026/27-Z", "2025/26-L", "2024/25-Z", "GONE"), options.map { it.id })
        assertEquals(TermWord.Ongoing, options.first().word)
        assertEquals(TermWord.None, options.last().word)
        assertEquals(emptyList<TermOption>(), termOptions(TermList(), emptySet(), today))
    }

    @Test fun theSelectionFollowsTheChoiceThenTheCurrentTermThenTheNewest() {
        val options = termOptions(TermList(listOf(a, b, c), "2026/27-Z"), setOf("2025/26-L"), today)
        assertEquals("2025/26-L", selectedTermId(options, "2025/26-L", "2026/27-Z"))
        assertEquals("2026/27-Z", selectedTermId(options, "gone", "2026/27-Z"))
        assertEquals("2026/27-Z", selectedTermId(options, null, null))
        assertNull(selectedTermId(emptyList(), "x", "y"))
    }

    @Test fun thePreviousTermIsTheNextOlderOption() {
        val options = termOptions(TermList(listOf(a, b, c), "2026/27-Z"), setOf("2025/26-L", "2024/25-Z"), today)
        assertEquals("2025/26-L", previousTermId(options, "2026/27-Z"))
        assertNull(previousTermId(options, "2024/25-Z"))
        assertNull(previousTermId(options, "missing"))
    }
}
