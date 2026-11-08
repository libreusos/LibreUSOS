package io.gitlab.voxix.libreusos.ui

import org.libreusos.core.CourseGrades
import org.libreusos.core.DataSource
import java.time.LocalDate
import java.time.ZoneId

data class GradesUiState(
    /** The installation's zone and today in it; null until known. */
    val zone: ZoneId? = null,
    val today: LocalDate? = null,
    /** Null until the cache or USOS answered; empty when there are no grades. */
    val sections: List<TermSection>? = null,
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
)

/** A course the detail screen shows. */
data class OpenCourse(val termId: String, val course: CourseGrades)

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
fun reduceCache(state: GradesUiState, sections: List<TermSection>, terms: TermList, sources: List<DataSource>): GradesUiState =
    if (state.sections != null) {
        state
    } else {
        state.copy(sections = sections, terms = terms, liveAtMs = null, dataAtSec = oldestFetchSec(sources, 0))
    }

/**
 * The live call answered. An offline fallback to the cache is not live: [notLive] says why, and the
 * data it brought stays on screen. A live answer clears the failure of the last refresh.
 */
fun reduceLive(
    state: GradesUiState,
    sections: List<TermSection>,
    terms: TermList,
    sources: List<DataSource>,
    nowMs: Long,
    notLive: HomeFailure,
): GradesUiState {
    val live = isLive(sources)
    return state.copy(
        sections = sections,
        terms = terms,
        liveAtMs = if (live) nowMs else null,
        dataAtSec = oldestFetchSec(sources, nowMs),
        failure = if (live) null else notLive,
    )
}

/** The refresh failed: older data stays on screen, marked as not live. */
fun reduceFailure(state: GradesUiState, failure: HomeFailure): GradesUiState =
    state.copy(failure = failure, liveAtMs = null)

/** The state as the helpers of Today see it, so Oceny shows the same banners, status line and full-screen states. */
fun GradesUiState.asHomeState(): HomeUiState = HomeUiState(
    zone = zone,
    today = today,
    activities = if (sections != null) emptyList() else null,
    dataAtSec = dataAtSec,
    liveAtMs = liveAtMs,
    loading = loading,
    refreshing = refreshing,
    failure = failure,
)
