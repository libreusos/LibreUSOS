package io.gitlab.voxix.libreusos.data

/** The background sync intervals Settings offers, in hours. WorkManager would allow 15 minutes, but a student does not need it. */
val SYNC_INTERVAL_OPTIONS_HOURS: List<Long> = listOf(1L, 3L, 6L, 12L, 24L)

/** The offered interval closest to [hours] (an old or hand-edited value still selects something). */
fun nearestSyncInterval(hours: Long): Long =
    SYNC_INTERVAL_OPTIONS_HOURS.minBy { kotlin.math.abs(it - hours) }
