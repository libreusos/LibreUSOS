package io.github.voxix.libreusos.ui

import org.libreusos.core.DataSource
import java.time.ZoneId

data class GradesUiState(
    /** The installation's zone, for times on screen; null until known. */
    val zone: ZoneId? = null,
    /** Null until the cache or USOS answered; empty when there are no grades. */
    val sections: List<TermSection>? = null,
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
)

/**
 * The data is live only when the terms and every term's grades came from USOS. The terms count even
 * when no grades came back: with an empty list, "all grades are live" would hold for any source.
 */
fun isLive(sources: List<DataSource>): Boolean = sources.isNotEmpty() && sources.all { it == DataSource.Live }

/** When the oldest of [sources] was fetched (epoch seconds); live ones count as [liveNowMs]. */
fun oldestFetchSec(sources: List<DataSource>, liveNowMs: Long): Long? = sources.minOfOrNull {
    when (it) {
        DataSource.Live -> liveNowMs / 1000
        is DataSource.Cached -> it.fetchedAt
    }
}

/** The cache answered before the live call: shown as cached, and only when nothing is on screen yet. */
fun reduceCache(state: GradesUiState, sections: List<TermSection>, sources: List<DataSource>): GradesUiState =
    if (state.sections != null) {
        state
    } else {
        state.copy(sections = sections, liveAtMs = null, dataAtSec = oldestFetchSec(sources, 0))
    }

/** The cache read failed although the live call may still work. */
fun reduceCacheFailure(state: GradesUiState, error: ErrorKind): GradesUiState = state.copy(error = error)

/** The live call answered. An offline fallback to the cache is not live. */
fun reduceLive(state: GradesUiState, sections: List<TermSection>, sources: List<DataSource>, nowMs: Long): GradesUiState {
    val live = isLive(sources)
    return state.copy(
        sections = sections,
        liveAtMs = if (live) nowMs else null,
        dataAtSec = oldestFetchSec(sources, nowMs),
        refreshFailed = false,
    )
}

/** The refresh failed: older data stays on screen, marked as not live. */
fun reduceFailure(state: GradesUiState, error: ErrorKind): GradesUiState =
    state.copy(error = error, refreshFailed = state.sections != null, liveAtMs = null)
