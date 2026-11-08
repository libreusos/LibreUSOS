package io.gitlab.voxix.libreusos.ui

import android.content.ClipData
import android.os.Build
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.data.clockTime
import io.gitlab.voxix.libreusos.sync.minutesUntil
import io.gitlab.voxix.libreusos.ui.components.Banner
import io.gitlab.voxix.libreusos.ui.components.BannerStyle
import io.gitlab.voxix.libreusos.ui.components.GroupedListCard
import io.gitlab.voxix.libreusos.ui.components.GroupedListRow
import io.gitlab.voxix.libreusos.ui.components.HeaderIconButton
import io.gitlab.voxix.libreusos.ui.components.KindBadge
import io.gitlab.voxix.libreusos.ui.components.ScreenHeader
import io.gitlab.voxix.libreusos.ui.components.SectionHeader
import io.gitlab.voxix.libreusos.ui.components.StatusLine
import io.gitlab.voxix.libreusos.ui.components.SyncState
import io.gitlab.voxix.libreusos.ui.components.WeekMotif
import io.gitlab.voxix.libreusos.ui.theme.LibreType
import io.gitlab.voxix.libreusos.ui.theme.Radii
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import io.gitlab.voxix.libreusos.ui.theme.kindCodeOf
import io.gitlab.voxix.libreusos.widget.TimedActivity
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.libreusos.core.Activity
import org.libreusos.core.ActivityKind
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter

/** Today (Dziś): header with the sync status, the now card and the day plan, and every state of SYSTEM.md section 12. */
@Composable
fun HomeScreen(
    viewModel: HomeViewModel,
    sessionExpired: Boolean,
    onRelogin: () -> Unit,
    onOpenTimetable: () -> Unit,
    onOpenClass: (Activity) -> Unit,
    modifier: Modifier = Modifier,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val record by viewModel.syncRecord.collectAsStateWithLifecycle()
    val locale = LocalConfiguration.current.locales[0]
    val zone = state.zone ?: ZoneId.systemDefault()
    var nowMs by remember { mutableStateOf(System.currentTimeMillis()) }
    val today = Instant.ofEpochMilli(nowMs).atZone(zone).toLocalDate()

    // First visit and every return from the background: the model loads when the day changed or the data is old.
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) {
        nowMs = System.currentTimeMillis()
        viewModel.ensureLoaded(Instant.ofEpochMilli(nowMs).atZone(zone).toLocalDate())
    }
    // While the screen is up: keep the clock of the plan current and notice midnight.
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    LaunchedEffect(zone, lifecycle) {
        lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                delay(60_000L - System.currentTimeMillis() % 60_000L)
                nowMs = System.currentTimeMillis()
                val day = Instant.ofEpochMilli(nowMs).atZone(zone).toLocalDate()
                if (viewModel.state.value.today != day) viewModel.ensureLoaded(day)
            }
        }
    }

    val mode = homeMode(state, sessionExpired)
    val banner = homeBanner(state, sessionExpired)
    val status = homeStatus(state, sessionExpired)
    val retryIn = record.nextAttemptAtMs?.let { minutesUntil(it, nowMs) }
    var details by rememberSaveable { mutableStateOf(false) }

    Column(modifier.fillMaxSize().padding(horizontal = Spacing.screenEdge)) {
        ScreenHeader(
            title = stringResource(R.string.home_title),
            eyebrow = DateTimeFormatter.ofPattern("EEEE, d MMMM", locale).format(today),
            statusLine = statusText(status, mode, state, zone, today, locale)?.let { text -> { SyncStatusLine(status, text) } },
            actions = {
                HeaderIconButton(
                    icon = when {
                        state.failure?.offline == true -> R.drawable.ic_cloud_off
                        state.failure != null -> R.drawable.ic_sync_problem
                        else -> R.drawable.ic_refresh
                    },
                    contentDescription = stringResource(R.string.today_refresh),
                    onClick = viewModel::refresh,
                    enabled = !state.loading && !sessionExpired,
                )
            },
        )
        val failure = state.failure
        SyncBanner(
            banner = banner,
            failure = failure,
            dataStamp = state.dataAtSec?.let { stampText(it, zone, today, locale) }.orEmpty(),
            retryIn = retryIn,
            onRetry = viewModel::refresh,
            onDetails = { details = true },
            onRelogin = onRelogin,
        )
        if (banner != HomeBanner.None) Spacer(Modifier.height(Spacing.s8))

        @OptIn(ExperimentalMaterial3Api::class)
        PullToRefreshBox(
            isRefreshing = state.refreshing,
            onRefresh = viewModel::refresh,
            modifier = Modifier.weight(1f).fillMaxWidth(),
        ) {
            when (mode) {
                HomeMode.Skeleton -> Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) { TodaySkeleton() }
                HomeMode.NoCacheOffline, HomeMode.NoCacheError, HomeMode.NoCacheExpired -> NoCacheContent(
                    mode = mode,
                    failure = failure,
                    retryIn = retryIn,
                    onRetry = viewModel::refresh,
                    onDetails = { details = true },
                    onRelogin = onRelogin,
                )
                HomeMode.Content -> DayContent(
                    state = state,
                    today = today,
                    nowMs = nowMs,
                    zone = zone,
                    onOpenTimetable = onOpenTimetable,
                    onOpenClass = onOpenClass,
                )
            }
        }
    }
    val shownFailure = state.failure
    // A cleared failure closes the sheet; the next failure must not reopen it unasked.
    LaunchedEffect(shownFailure == null) { if (shownFailure == null) details = false }
    if (details && shownFailure != null) {
        DetailsSheet(
            text = detailsText(shownFailure, viewModel.baseUrl, zone),
            onDismiss = { details = false },
        )
    }
}

/** The classes of the day: now card and plan, or the empty day. */
@Composable
private fun DayContent(
    state: HomeUiState,
    today: LocalDate,
    nowMs: Long,
    zone: ZoneId,
    onOpenTimetable: () -> Unit,
    onOpenClass: (Activity) -> Unit,
) {
    val lang = LocalConfiguration.current.locales[0].language
    val activities = state.activities.orEmpty()
    val nowLocal = Instant.ofEpochMilli(nowMs).atZone(zone).toLocalDateTime()
    val plan = remember(activities, today, nowLocal) { todayPlan(activities, today, nowLocal) }
    if (plan.rows.isEmpty()) {
        EmptyDay(remember(activities, today) { nextClasses(activities, today) }, today, lang, onOpenTimetable, onOpenClass)
        return
    }
    LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(bottom = Spacing.s24)) {
        if (plan.card != null) item(key = "card") { NowCard(plan, lang, onClick = { onOpenClass(plan.rows[plan.card.index].item.activity) }) }
        item(key = "plan-header") { PlanHeader(plan.rows.size) }
        itemsIndexed(plan.rows, key = { i, _ -> "row-$i" }) { i, row ->
            TimelineEntry(
                row,
                lang,
                isFirst = i == 0,
                isLast = i == plan.rows.lastIndex,
                onClick = { onOpenClass(row.item.activity) },
                isCardClass = i == plan.card?.index,
            )
        }
    }
}

@Composable
private fun EmptyDay(
    next: NextClasses?,
    today: LocalDate,
    lang: String,
    onOpenTimetable: () -> Unit,
    onOpenClass: (Activity) -> Unit,
) {
    val locale = LocalConfiguration.current.locales[0]
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(top = Spacing.s24, bottom = Spacing.s24),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        val column = today.dayOfWeek.value - 1
        WeekMotif(todayColumn = column.takeIf { it in 0..4 })
        Spacer(Modifier.height(Spacing.s20))
        Text(stringResource(R.string.today_empty_title), style = MaterialTheme.typography.headlineSmall, textAlign = TextAlign.Center)
        Spacer(Modifier.height(Spacing.s8))
        Text(
            stringResource(if (next == null) R.string.today_empty_none_soon else R.string.today_empty_body),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
        if (next != null) {
            val (shown, more) = splitNextClasses(next.items)
            val day = DateTimeFormatter.ofPattern("EEE d MMM", locale).format(next.date)
            SectionHeader(
                stringResource(R.string.today_next_classes) + " · " +
                    if (next.date == today.plusDays(1)) stringResource(R.string.today_tomorrow, day) else day,
            )
            GroupedListCard {
                shown.forEach { item -> row { NextClassRow(item, lang, onClick = { onOpenClass(item.activity) }) } }
                if (more > 0) row {
                    GroupedListRow(pluralStringResource(R.plurals.today_more_classes, more, more))
                }
            }
            Spacer(Modifier.height(Spacing.s16))
            OutlinedButton(onOpenTimetable) { Text(stringResource(R.string.today_whole_week)) }
        }
    }
}

@Composable
private fun NextClassRow(item: TimedActivity, lang: String, onClick: () -> Unit) {
    val a = item.activity
    val place = placeLine(a, includeGroup = false)
    GroupedListRow(
        title = activityTitle(a, lang),
        subtitle = listOf("${clockTime(a.start)}–${clockTime(a.end)}", place).filter { it.isNotEmpty() }.joinToString(" · "),
        onClick = onClick,
        chevron = false,
        trailing = {
            KindBadge(kindCodeOf(a.classtypeId, a.classtypeName.pl ?: a.classtypeName.en, a.kind == ActivityKind.Exam))
        },
    )
}
