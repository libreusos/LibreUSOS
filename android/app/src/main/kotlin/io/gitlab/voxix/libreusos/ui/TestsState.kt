package io.gitlab.voxix.libreusos.ui

import org.libreusos.core.DataSource
import java.time.LocalDate
import java.time.ZoneId

data class TestsUiState(
    /** The installation's zone and today in it; null until known. */
    val zone: ZoneId? = null,
    val today: LocalDate? = null,
    /** Null until the cache or USOS answered; empty when there are no tests. */
    val sections: List<TestTermSection>? = null,
    val terms: TermList = TermList(),
    /** A cache read or a live fetch is running. */
    val loading: Boolean = false,
    /** A user pull-to-refresh is waiting for the running fetch. */
    val refreshing: Boolean = false,
    /** Why the last refresh failed; [sections] is then older data, or null when there is none. */
    val failure: HomeFailure? = null,
    /** When the data was last fetched live (epoch ms); null when it is cached, or a refresh failed. */
    val liveAtMs: Long? = null,
    /** When the oldest data on screen was fetched (epoch seconds), live or cached. */
    val dataAtSec: Long? = null,
    /** How many tests USOS refused in the last live answer (the cached copies of those stay on screen). */
    val skipped: Int = 0,
)

/** The cache answered before the live call: shown as cached, and only when nothing is on screen yet. */
fun reduceTestsCache(state: TestsUiState, sections: List<TestTermSection>, terms: TermList, sources: List<DataSource>): TestsUiState =
    if (state.sections != null) {
        state
    } else {
        state.copy(sections = sections, terms = terms, liveAtMs = null, dataAtSec = oldestFetchSec(sources, 0))
    }

/**
 * The live call answered. [sources] are those of the terms fetched live; an offline fallback to the
 * cache, or a term that could not be loaded ([failedTerms]), is not live and says [notLive]. After
 * a fallback the previous [TestsUiState.skipped] stays.
 */
fun reduceTestsLive(
    state: TestsUiState,
    sections: List<TestTermSection>,
    terms: TermList,
    sources: List<DataSource>,
    skipped: Int,
    failedTerms: Int,
    nowMs: Long,
    notLive: HomeFailure,
): TestsUiState {
    val live = isLive(sources) && failedTerms == 0
    return state.copy(
        sections = sections,
        terms = terms,
        liveAtMs = if (live) nowMs else null,
        dataAtSec = oldestFetchSec(sources, nowMs),
        failure = if (live) null else notLive,
        skipped = if (isLive(sources)) skipped else state.skipped,
    )
}

/** The refresh failed: older data stays on screen, marked as not live. */
fun reduceTestsFailure(state: TestsUiState, failure: HomeFailure): TestsUiState =
    state.copy(failure = failure, liveAtMs = null)

/** The state as the helpers of Today see it, so Sprawdziany shows the same banners, status line and full-screen states. */
fun TestsUiState.asHomeState(): HomeUiState = HomeUiState(
    zone = zone,
    today = today,
    activities = if (sections != null) emptyList() else null,
    dataAtSec = dataAtSec,
    liveAtMs = liveAtMs,
    loading = loading,
    refreshing = refreshing,
    failure = failure,
)
