package io.gitlab.voxix.libreusos.ui

import org.libreusos.core.DataSource
import java.time.ZoneId

data class TestsUiState(
    /** The installation's zone, for times on screen; null until known. */
    val zone: ZoneId? = null,
    /** Null until the cache or USOS answered; empty when there are no tests. */
    val sections: List<TestTermSection>? = null,
    /** A cache read or a live fetch is running. */
    val loading: Boolean = false,
    /** A user pull-to-refresh is waiting for the running fetch. */
    val refreshing: Boolean = false,
    /** The last attempt failed with this (shown in full only when there is no data to fall back on). */
    val error: ErrorKind? = null,
    /** When the data was last fetched live (epoch ms); null when it is cached, or a refresh failed. */
    val liveAtMs: Long? = null,
    /** When the oldest data on screen was fetched (epoch seconds), live or cached. */
    val dataAtSec: Long? = null,
    /** A refresh failed and [sections] is older data. */
    val refreshFailed: Boolean = false,
    /** How many tests USOS refused in the last live answer (the cached copies of those stay on screen). */
    val skipped: Int = 0,
    /** How many terms could not be loaded at all in the last live answer (failed, and nothing cached). */
    val failedTerms: Int = 0,
    /** Keys of the rows the user opened or closed against their default; see [isExpanded]. */
    val toggled: Set<String> = emptySet(),
)

/** The cache answered before the live call: shown as cached, and only when nothing is on screen yet. */
fun reduceTestsCache(state: TestsUiState, sections: List<TestTermSection>, sources: List<DataSource>): TestsUiState =
    if (state.sections != null) {
        state
    } else {
        state.copy(sections = sections, liveAtMs = null, dataAtSec = oldestFetchSec(sources, 0))
    }

/** The cache read failed although the live call may still work. */
fun reduceTestsCacheFailure(state: TestsUiState, error: ErrorKind): TestsUiState = state.copy(error = error)

/**
 * The live call answered. [sources] are those of the terms fetched live; an offline fallback to the
 * cache is not live, and then the previous [TestsUiState.skipped] stays. Choices for rows that are no
 * longer in [sections] are dropped.
 */
fun reduceTestsLive(
    state: TestsUiState,
    sections: List<TestTermSection>,
    sources: List<DataSource>,
    skipped: Int,
    failedTerms: Int,
    nowMs: Long,
): TestsUiState {
    val live = isLive(sources)
    return state.copy(
        sections = sections,
        liveAtMs = if (live && failedTerms == 0) nowMs else null,
        dataAtSec = oldestFetchSec(sources, nowMs),
        refreshFailed = false,
        skipped = if (live) skipped else state.skipped,
        failedTerms = failedTerms,
        toggled = pruneToggled(state.toggled, sections),
    )
}

/** The refresh failed: older data stays on screen, marked as not live. */
fun reduceTestsFailure(state: TestsUiState, error: ErrorKind): TestsUiState =
    state.copy(error = error, refreshFailed = state.sections != null, liveAtMs = null)

/** The user opened or closed the row under [key]. */
fun reduceToggle(state: TestsUiState, key: String): TestsUiState = state.copy(toggled = toggleKey(state.toggled, key))
