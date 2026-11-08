package io.gitlab.voxix.libreusos.sync

/**
 * What the app knows about its syncs. [lastSuccessMs] is the last live fetch, [failures] the failures since
 * then, [lastFailureMs] the latest of them and [nextAttemptAtMs] the automatic retry that is waiting.
 * [expired] is set when USOS rejected the token; it lets the widget say so without the app running.
 */
data class SyncRecord(
    val lastSuccessMs: Long? = null,
    val failures: Int = 0,
    val lastFailureMs: Long? = null,
    val nextAttemptAtMs: Long? = null,
    val expired: Boolean = false,
)

/** Retry steps the user sees (SYSTEM.md section 11): 15 min, 1 h, 6 h, then back to the normal schedule. */
private val STEPS_MS = listOf(15 * 60_000L, 60 * 60_000L, 6 * 60 * 60_000L)

/** The wait after the [failures]th failure in a row, or null once the steps are used up. */
fun backoffDelayMs(failures: Int): Long? = STEPS_MS.getOrNull(failures - 1)

/**
 * A failed attempt at [nowMs]. While a retry is still waiting the failure is only noted (a manual retry
 * must not push the schedule to the 6 h step); otherwise the next step is planned.
 */
fun SyncRecord.afterFailure(nowMs: Long): SyncRecord {
    if (nextAttemptAtMs != null && nextAttemptAtMs > nowMs) return copy(lastFailureMs = nowMs)
    val count = failures + 1
    return copy(failures = count, lastFailureMs = nowMs, nextAttemptAtMs = backoffDelayMs(count)?.let { nowMs + it })
}

fun SyncRecord.afterSuccess(nowMs: Long): SyncRecord = SyncRecord(lastSuccessMs = nowMs)

/** Whole minutes (at least 1) until [atMs], rounded up. */
fun minutesUntil(atMs: Long, nowMs: Long): Int = ((atMs - nowMs + 59_999L) / 60_000L).toInt().coerceAtLeast(1)
