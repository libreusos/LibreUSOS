package io.gitlab.voxix.libreusos.data

/** Light or dark appearance: follow the system, or force one. */
enum class ThemeMode(val stored: String) {
    System("system"),
    Light("light"),
    Dark("dark");

    companion object {
        /** The mode saved as [stored]; anything unknown (or missing) means [System]. */
        fun fromStored(value: String?): ThemeMode = entries.firstOrNull { it.stored == value } ?: System
    }
}
