package io.gitlab.voxix.libreusos.ui

import java.net.URI

/**
 * Up to two initials for the avatar tile: the first letters of the first and last name, uppercased.
 * Falls back to a single letter for one name and to "?" when there is none.
 */
fun initialsOf(firstName: String?, lastName: String?): String {
    val letters = listOf(firstName, lastName).mapNotNull { name ->
        name?.trim()?.codePoints()?.toArray()?.firstOrNull { Character.isLetter(it) }
            ?.let { String(Character.toChars(it)).uppercase() }
    }
    return if (letters.isEmpty()) "?" else letters.joinToString("")
}

/** The host of an API base URL ("https://usosapi.polsl.pl/" gives "usosapi.polsl.pl"); the input when it does not parse. */
fun hostOf(baseUrl: String): String {
    val trimmed = baseUrl.trim()
    return runCatching { URI(trimmed).host }.getOrNull()?.takeIf { it.isNotBlank() } ?: trimmed
}

private val KNOWN_UNIVERSITIES = mapOf(
    "usosapi.polsl.pl" to "Politechnika Śląska",
)

/**
 * The university's name for the Więcej eyebrow. The core exposes no university name, so this is a
 * small table of the servers the app ships a key for; other servers show their host.
 */
fun universityNameOf(baseUrl: String): String = KNOWN_UNIVERSITIES[hostOf(baseUrl).lowercase()] ?: hostOf(baseUrl)

/** What the Więcej account card and the account sheet show. */
data class AccountUiState(
    val name: String? = null,
    val initials: String = "?",
    val university: String = "",
    val host: String = "",
)
