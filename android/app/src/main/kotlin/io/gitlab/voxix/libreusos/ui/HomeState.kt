package io.gitlab.voxix.libreusos.ui

import io.gitlab.voxix.libreusos.data.isStale
import org.libreusos.core.Activity
import org.libreusos.core.UsosException
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId

/**
 * Why the last refresh failed. [offline] says the device had no connection; [status], [reason] and
 * [errorClass] only feed the "Szczegóły" sheet (never the core's free text, which may name a request).
 */
data class HomeFailure(
    val kind: ErrorKind,
    val offline: Boolean,
    val atMs: Long,
    val status: Int? = null,
    val reason: String? = null,
    val errorClass: String = "",
)

/** Today's screen state. [activities] covers [today] and the week after it; null means nothing is known yet. */
data class HomeUiState(
    val zone: ZoneId? = null,
    val today: LocalDate? = null,
    val activities: List<Activity>? = null,
    /** When the activities were fetched from USOS, epoch seconds. */
    val dataAtSec: Long? = null,
    /** When they were last fetched live by this screen. */
    val liveAtMs: Long? = null,
    val loading: Boolean = false,
    val refreshing: Boolean = false,
    val failure: HomeFailure? = null,
)

fun failureOf(t: Throwable, online: Boolean, nowMs: Long): HomeFailure {
    val unreachable = t is UsosException.Network || t is UsosException.NotCached
    return HomeFailure(
        kind = errorKindOf(t),
        offline = unreachable && !online,
        atMs = nowMs,
        status = (t as? UsosException.Api)?.status?.toInt(),
        reason = (t as? UsosException.Api)?.let { it.reason ?: it.code },
        errorClass = t.javaClass.simpleName,
    )
}

/** The stored copy fills the screen at once; it never replaces data that is already shown. */
fun reduceCache(s: HomeUiState, activities: List<Activity>, fetchedAtSec: Long): HomeUiState =
    if (s.activities != null) s else s.copy(activities = activities, dataAtSec = fetchedAtSec)

fun reduceLive(s: HomeUiState, activities: List<Activity>, nowMs: Long): HomeUiState =
    s.copy(activities = activities, dataAtSec = nowMs / 1000, liveAtMs = nowMs, failure = null)

/** A failed refresh keeps whatever is shown; [cached] is the stored copy the core fell back to, when it has one. */
fun reduceFailure(s: HomeUiState, failure: HomeFailure, cached: List<Activity>? = null, cachedAtSec: Long? = null): HomeUiState =
    s.copy(
        activities = cached ?: s.activities,
        dataAtSec = if (cached != null) cachedAtSec else s.dataAtSec,
        liveAtMs = null,
        failure = failure,
    )

/** What fills the screen. The no-cache states are the only full-screen ones (SYSTEM.md section 12). */
enum class HomeMode { Skeleton, NoCacheOffline, NoCacheError, NoCacheExpired, Content }

fun homeMode(s: HomeUiState, expired: Boolean): HomeMode = when {
    s.activities != null -> HomeMode.Content
    expired -> HomeMode.NoCacheExpired
    s.failure == null -> HomeMode.Skeleton
    s.failure.offline -> HomeMode.NoCacheOffline
    else -> HomeMode.NoCacheError
}

/** The one banner under the header: expired wins, then the failure of the last refresh. Offline is said here, once. */
enum class HomeBanner { None, Offline, SyncError, Expired }

fun homeBanner(s: HomeUiState, expired: Boolean): HomeBanner = when {
    s.activities == null -> HomeBanner.None
    expired -> HomeBanner.Expired
    s.failure == null -> HomeBanner.None
    s.failure.offline -> HomeBanner.Offline
    else -> HomeBanner.SyncError
}

/** The status line under the title; the dot colour follows [HomeStatus.isFailure]. */
enum class HomeStatus(val isFailure: Boolean = false) {
    /** "Pobieranie planu…" on the first load, "Synchronizacja…" later. */
    Fetching,
    Synced,
    /** "Dane z 10:52 · próba o 11:06 nieudana" */
    DataFromFailed(true),
    /** "Ostatnia synchronizacja 7 lis, 18:02" */
    LastSync(true),
    OfflineNoData,
    FirstSyncFailed(true),
    /** The offline banner already says it. */
    Hidden,
}

fun homeStatus(s: HomeUiState, expired: Boolean): HomeStatus = when (homeMode(s, expired)) {
    HomeMode.Skeleton -> HomeStatus.Fetching
    HomeMode.NoCacheOffline -> HomeStatus.OfflineNoData
    HomeMode.NoCacheError, HomeMode.NoCacheExpired -> HomeStatus.FirstSyncFailed
    HomeMode.Content -> when {
        expired -> HomeStatus.LastSync
        s.loading -> HomeStatus.Fetching
        s.failure?.offline == true -> HomeStatus.Hidden
        s.failure != null -> HomeStatus.DataFromFailed
        else -> HomeStatus.Synced
    }
}

/** Load when nothing is shown, the day changed or the live data is old (the app came back from the background). */
fun needsRefresh(s: HomeUiState, today: LocalDate, nowMs: Long): Boolean =
    !s.loading && (s.activities == null || s.today != today || isStale(s.liveAtMs, nowMs))

enum class StampDay { Today, Yesterday, Other }

/** Which day word a date stamp of [atSec] takes: "dziś, 10:52", "wczoraj, 22:14" or the date. */
fun stampDay(atSec: Long, zone: ZoneId, today: LocalDate): StampDay {
    val day = Instant.ofEpochSecond(atSec).atZone(zone).toLocalDate()
    return when (day) {
        today -> StampDay.Today
        today.minusDays(1) -> StampDay.Yesterday
        else -> StampDay.Other
    }
}
