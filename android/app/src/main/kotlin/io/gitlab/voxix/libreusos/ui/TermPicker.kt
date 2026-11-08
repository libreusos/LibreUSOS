package io.gitlab.voxix.libreusos.ui

import org.libreusos.core.Term
import java.time.LocalDate

/** The state word next to a term in the picker: "trwa", "zakończony", or the season of a term still to come. */
enum class TermWord { Ongoing, Ended, Summer, Winter, None }

/** A term the picker offers. [word] is shown after the id. */
data class TermOption(val id: String, val word: TermWord)

/** The terms the grades and tests know about, and the one the core calls current (null between terms). */
data class TermList(val terms: List<Term> = emptyList(), val currentId: String? = null)

/**
 * The state of [term]: ongoing when it is the core's current one, ended when its end date is past, else
 * the season from the id (`2026/27-Z` is winter, `-L` summer).
 */
fun termWord(term: Term, currentId: String?, today: LocalDate): TermWord {
    if (term.id == currentId) return TermWord.Ongoing
    val end = runCatching { LocalDate.parse(term.endDate) }.getOrNull()
    if (end != null && end < today) return TermWord.Ended
    return when (term.id.substringAfterLast('-', "")) {
        "L" -> TermWord.Summer
        "Z" -> TermWord.Winter
        else -> TermWord.None
    }
}

/**
 * The picker's terms, newest first (by start date, then end date, then id): the current term, so a term
 * without grades yet can be shown, and every term in [withData]. Ids missing from [list] come last.
 */
fun termOptions(list: TermList, withData: Set<String>, today: LocalDate): List<TermOption> {
    val wanted = withData + listOfNotNull(list.currentId)
    val known = list.terms
        .filter { it.id in wanted }
        .distinctBy { it.id }
        .sortedWith(compareByDescending<Term> { it.startDate }.thenByDescending { it.endDate }.thenByDescending { it.id })
        .map { TermOption(it.id, termWord(it, list.currentId, today)) }
    val knownIds = known.map { it.id }.toSet()
    val unknown = (wanted - knownIds).sortedDescending().map { TermOption(it, TermWord.None) }
    return known + unknown
}

/** The term to show: the user's choice while it is still offered, else the current term, else the newest one. */
fun selectedTermId(options: List<TermOption>, chosen: String?, currentId: String?): String? =
    listOf(chosen, currentId).firstOrNull { id -> id != null && options.any { it.id == id } } ?: options.firstOrNull()?.id

/** The term just before [selected] in [options] (older), for "Poprzedni semestr"; null for the oldest. */
fun previousTermId(options: List<TermOption>, selected: String?): String? {
    val i = options.indexOfFirst { it.id == selected }
    return if (i >= 0) options.getOrNull(i + 1)?.id else null
}
