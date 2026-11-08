package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringArrayResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.repeatOnLifecycle
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.data.TIMETABLE_PAGES
import io.gitlab.voxix.libreusos.data.dateOfPage
import io.gitlab.voxix.libreusos.data.mondayOf
import io.gitlab.voxix.libreusos.data.pageOf
import io.gitlab.voxix.libreusos.sync.minutesUntil
import io.gitlab.voxix.libreusos.ui.components.Banner
import io.gitlab.voxix.libreusos.ui.components.BannerStyle
import io.gitlab.voxix.libreusos.ui.components.HeaderIconButton
import io.gitlab.voxix.libreusos.ui.components.ScreenHeader
import io.gitlab.voxix.libreusos.ui.theme.Radii
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import kotlinx.coroutines.delay
import org.libreusos.core.Activity
import java.time.Instant
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

/** Plan (Plan): a day pager with a week strip, or the week grid; both open a class and show the sync states of Today. */
@Composable
fun TimetableScreen(
    viewModel: TimetableViewModel,
    sessionExpired: Boolean,
    onRelogin: () -> Unit,
    onOpenClass: (Activity) -> Unit,
    modifier: Modifier = Modifier,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { viewModel.updateToday() }
    val zone = state.zone
    val today = state.today
    if (zone == null || today == null) {
        Box(modifier.fillMaxSize(), contentAlignment = Alignment.Center) { CircularProgressIndicator() }
        return
    }
    PlanScreen(viewModel, state, zone, today, sessionExpired, onRelogin, onOpenClass, modifier)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PlanScreen(
    viewModel: TimetableViewModel,
    state: TimetableUiState,
    zone: ZoneId,
    today: LocalDate,
    sessionExpired: Boolean,
    onRelogin: () -> Unit,
    onOpenClass: (Activity) -> Unit,
    modifier: Modifier,
) {
    val locale = LocalConfiguration.current.locales[0]
    val lang = locale.language
    val record by viewModel.syncRecord.collectAsStateWithLifecycle()

    // The minute clock: classes in progress and the now-line follow it while the screen is up.
    var nowMs by remember { mutableStateOf(System.currentTimeMillis()) }
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    LaunchedEffect(zone, lifecycle) {
        lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                nowMs = System.currentTimeMillis()
                viewModel.updateToday()
                delay(60_000L - System.currentTimeMillis() % 60_000L)
            }
        }
    }
    val now = Instant.ofEpochMilli(nowMs).atZone(zone).toLocalDateTime()

    var selectedEpoch by rememberSaveable { mutableLongStateOf(today.toEpochDay()) }
    var weekView by rememberSaveable { mutableStateOf(false) }
    var details by rememberSaveable { mutableStateOf(false) }
    var clash by remember { mutableStateOf<ClashGroup?>(null) }
    val selected = LocalDate.ofEpochDay(selectedEpoch)
    val weekStart = mondayOf(selected)

    // The weeks on screen: the selected day and its neighbours, or the selected week.
    fun visibleWeeks(): Set<LocalDate> =
        if (weekView) setOf(weekStart) else setOf(mondayOf(selected.minusDays(1)), weekStart, mondayOf(selected.plusDays(1)))
    LaunchedEffect(selected, weekView) { viewModel.ensureWeeks(visibleWeeks()) }
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { viewModel.ensureWeeks(visibleWeeks()) }

    val weekState = state.weeks[weekStart]
    val home = weekState.asHomeState(zone, today)
    val mode = homeMode(home, sessionExpired)
    val banner = homeBanner(home, sessionExpired)
    val status = homeStatus(home, sessionExpired)
    val retryIn = record.nextAttemptAtMs?.let { minutesUntil(it, nowMs) }
    val failure = weekState?.failure

    Column(modifier.fillMaxSize().padding(horizontal = Spacing.screenEdge)) {
        ScreenHeader(
            title = stringResource(R.string.nav_timetable),
            eyebrow = DateTimeFormatter.ofPattern("LLLL yyyy", locale).format(selected).let { month ->
                val week = state.term?.let { t ->
                    runCatching { termWeek(LocalDate.parse(t.startDate), LocalDate.parse(t.endDate), selected) }.getOrNull()
                }
                if (week == null) month else stringResource(R.string.plan_eyebrow_week, month, week)
            },
            statusLine = if (status == HomeStatus.Synced) {
                null
            } else {
                statusText(status, mode, home, zone, today, locale)?.let { text -> { SyncStatusLine(status, text) } }
            },
            actions = {
                HeaderIconButton(
                    icon = R.drawable.ic_calendar_today,
                    contentDescription = stringResource(R.string.timetable_today),
                    onClick = { selectedEpoch = today.toEpochDay() },
                    enabled = selected != today,
                )
                HeaderIconButton(
                    icon = when {
                        failure?.offline == true -> R.drawable.ic_cloud_off
                        failure != null -> R.drawable.ic_sync_problem
                        else -> R.drawable.ic_refresh
                    },
                    contentDescription = stringResource(R.string.today_refresh),
                    onClick = { viewModel.refresh(weekStart) },
                    enabled = weekState?.loading != true && !sessionExpired,
                )
            },
        )
        SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
            listOf(R.string.plan_view_day, R.string.plan_view_week).forEachIndexed { index, label ->
                SegmentedButton(
                    selected = (index == 1) == weekView,
                    onClick = { weekView = index == 1 },
                    shape = SegmentedButtonDefaults.itemShape(index, 2),
                ) { Text(stringResource(label)) }
            }
        }
        Spacer(Modifier.padding(top = Spacing.s8))
        SyncBanner(
            banner = banner,
            failure = failure,
            dataStamp = weekState?.dataAtSec?.let { stampText(it, zone, today, locale) }.orEmpty(),
            retryIn = retryIn,
            onRetry = { viewModel.refresh(weekStart) },
            onDetails = { details = true },
            onRelogin = onRelogin,
        )
        if (banner != HomeBanner.None) Spacer(Modifier.padding(top = Spacing.s8))

        if (weekView) {
            WeekView(viewModel, state, zone, today, now, lang, locale, selected, { selectedEpoch = it.toEpochDay() }, sessionExpired, retryIn, { details = true }, onRelogin, onOpenClass, { clash = it })
        } else {
            DayView(viewModel, state, zone, today, now, lang, locale, selected, { selectedEpoch = it.toEpochDay() }, sessionExpired, retryIn, { details = true }, onRelogin, onOpenClass, { clash = it })
        }
    }

    val shownFailure = failure
    // A cleared failure closes the sheet; the next failure must not reopen it unasked.
    LaunchedEffect(shownFailure == null) { if (shownFailure == null) details = false }
    if (details && shownFailure != null) {
        DetailsSheet(detailsText(shownFailure, viewModel.baseUrl, zone), onDismiss = { details = false })
    }
    clash?.let { group ->
        ConflictSheet(group, lang, onOpenClass = onOpenClass, onDismiss = { clash = null })
    }
}

/** The part of a week's state that decides what fills the area below the header. */
@Composable
private fun WeekArea(
    viewModel: TimetableViewModel,
    state: TimetableUiState,
    weekStart: LocalDate,
    zone: ZoneId,
    today: LocalDate,
    sessionExpired: Boolean,
    retryIn: Int?,
    onDetails: () -> Unit,
    onRelogin: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable (List<Activity>) -> Unit,
) {
    val week = state.weeks[weekStart]
    val mode = homeMode(week.asHomeState(zone, today), sessionExpired)
    @OptIn(ExperimentalMaterial3Api::class)
    PullToRefreshBox(
        isRefreshing = week?.refreshing == true,
        onRefresh = { viewModel.refresh(weekStart) },
        modifier = modifier.fillMaxSize(),
    ) {
        when (mode) {
            HomeMode.Skeleton -> Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
                TodaySkeleton(rowsOnly = true)
            }
            HomeMode.NoCacheOffline, HomeMode.NoCacheError, HomeMode.NoCacheExpired -> NoCacheContent(
                mode = mode,
                failure = week?.failure,
                retryIn = retryIn,
                onRetry = { viewModel.refresh(weekStart) },
                onDetails = onDetails,
                onRelogin = onRelogin,
            )
            HomeMode.Content -> content(week?.activities.orEmpty())
        }
    }
}

@Composable
private fun DayView(
    viewModel: TimetableViewModel,
    state: TimetableUiState,
    zone: ZoneId,
    today: LocalDate,
    now: LocalDateTime,
    lang: String,
    locale: Locale,
    selected: LocalDate,
    onSelect: (LocalDate) -> Unit,
    sessionExpired: Boolean,
    retryIn: Int?,
    onDetails: () -> Unit,
    onRelogin: () -> Unit,
    onOpenClass: (Activity) -> Unit,
    onOpenClash: (ClashGroup) -> Unit,
) {
    val pager = rememberPagerState(initialPage = pageOf(selected)) { TIMETABLE_PAGES }
    // The strip, the today button and a swipe all move the same selected day.
    LaunchedEffect(selected) {
        val page = pageOf(selected)
        if (pager.currentPage != page) pager.animateScrollToPage(page)
    }
    LaunchedEffect(pager) {
        snapshotFlow { pager.settledPage }.collect { onSelect(dateOfPage(it)) }
    }
    val weekStart = mondayOf(selected)
    WeekStrip(weekStart, selected, today, dayCounts(state.weeks[weekStart]?.activities.orEmpty()), locale, onSelect)
    HorizontalPager(pager, Modifier.fillMaxSize()) { page ->
        val date = dateOfPage(page)
        WeekArea(viewModel, state, mondayOf(date), zone, today, sessionExpired, retryIn, onDetails, onRelogin) { activities ->
            DayList(activities, date, now, lang, locale, onOpenClass, onOpenClash)
        }
    }
}

@Composable
private fun WeekView(
    viewModel: TimetableViewModel,
    state: TimetableUiState,
    zone: ZoneId,
    today: LocalDate,
    now: LocalDateTime,
    lang: String,
    locale: Locale,
    selected: LocalDate,
    onSelect: (LocalDate) -> Unit,
    sessionExpired: Boolean,
    retryIn: Int?,
    onDetails: () -> Unit,
    onRelogin: () -> Unit,
    onOpenClass: (Activity) -> Unit,
    onOpenClash: (ClashGroup) -> Unit,
) {
    val weekStart = mondayOf(selected)
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        IconButton({ onSelect(selected.minusWeeks(1)) }) {
            Icon(painterResource(R.drawable.ic_chevron_left), stringResource(R.string.timetable_previous_week))
        }
        Text(
            weekRangeLabel(weekStart, locale),
            Modifier.weight(1f),
            style = MaterialTheme.typography.titleMedium,
            textAlign = TextAlign.Center,
        )
        IconButton({ onSelect(selected.plusWeeks(1)) }) {
            Icon(painterResource(R.drawable.ic_chevron_right), stringResource(R.string.timetable_next_week))
        }
    }
    WeekArea(viewModel, state, weekStart, zone, today, sessionExpired, retryIn, onDetails, onRelogin) { activities ->
        val grid = remember(activities, weekStart) { weekGrid(activities, weekStart) }
        WeekGridView(grid, today, now, lang, onOpenClass, onOpenClash)
    }
}

/** Seven days: the selected one filled, today's numeral in primary, up to three dots for its classes. */
@Composable
private fun WeekStrip(
    weekStart: LocalDate,
    selected: LocalDate,
    today: LocalDate,
    counts: Map<LocalDate, Int>,
    locale: Locale,
    onSelect: (LocalDate) -> Unit,
) {
    val colors = MaterialTheme.colorScheme
    val abbreviations = stringArrayResource(R.array.plan_day_abbr)
    // At large font sizes the dots go: the cell has no room (SYSTEM.md section 15).
    val dots = LocalDensity.current.fontScale < 1.5f
    Row(Modifier.fillMaxWidth().padding(vertical = Spacing.s8)) {
        for (i in 0..6) {
            val date = weekStart.plusDays(i.toLong())
            val isSelected = date == selected
            val count = counts[date] ?: 0
            val weekend = i >= 5
            val description = DateTimeFormatter.ofPattern("EEEE d MMMM", locale).format(date) + ", " + classesCount(count)
            Column(
                Modifier
                    .weight(1f)
                    .heightIn(min = Spacing.touchTarget)
                    .clip(RoundedCornerShape(Radii.banner))
                    .background(if (isSelected) colors.primary else colors.surface)
                    .clickable(role = Role.Tab) { onSelect(date) }
                    .semantics(mergeDescendants = true) {
                        this.selected = isSelected
                        contentDescription = description
                    }
                    .padding(vertical = Spacing.s6),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                val muted = if (weekend) colors.onSurfaceVariant else colors.onSurface
                Text(
                    abbreviations[i],
                    style = MaterialTheme.typography.labelMedium,
                    color = if (isSelected) colors.onPrimary else colors.onSurfaceVariant,
                )
                Text(
                    date.dayOfMonth.toString(),
                    style = MaterialTheme.typography.titleLarge,
                    color = when {
                        isSelected -> colors.onPrimary
                        date == today -> colors.primary
                        else -> muted
                    },
                )
                if (dots && count > 0) {
                    Row(Modifier.padding(top = Spacing.s2), horizontalArrangement = Arrangement.spacedBy(Spacing.s2)) {
                        repeat(minOf(count, 3)) {
                            Box(Modifier.size(Spacing.s4).background(if (isSelected) colors.onPrimary else colors.onSurfaceVariant, CircleShape))
                        }
                    }
                }
            }
        }
    }
}

/** The classes of one day: heading, one banner per clash, the timeline and a hint. */
@Composable
private fun DayList(
    activities: List<Activity>,
    date: LocalDate,
    now: LocalDateTime,
    lang: String,
    locale: Locale,
    onOpenClass: (Activity) -> Unit,
    onOpenClash: (ClashGroup) -> Unit,
) {
    val plan = remember(activities, date, now) { todayPlan(activities, date, now) }
    val groups = remember(plan) { clashGroups(plan.rows.map { it.item }) }
    LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(top = Spacing.s8, bottom = Spacing.s24)) {
        item(key = "heading") {
            Row(Modifier.fillMaxWidth().padding(bottom = Spacing.s8), verticalAlignment = Alignment.CenterVertically) {
                Text(
                    DateTimeFormatter.ofPattern("EEEE, d MMMM", locale).format(date).replaceFirstChar { it.titlecase(locale) },
                    Modifier.weight(1f),
                    style = MaterialTheme.typography.titleMedium,
                )
                if (plan.rows.isNotEmpty()) {
                    Text(classesCount(plan.rows.size), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
        if (plan.rows.isEmpty()) {
            item(key = "empty") {
                Text(
                    stringResource(R.string.timetable_empty_day),
                    Modifier.padding(vertical = Spacing.s24).fillMaxWidth(),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
            }
            return@LazyColumn
        }
        itemsIndexed(groups, key = { i, _ -> "clash-$i" }) { _, group ->
            Banner(
                clashText(group, lang),
                Modifier.padding(bottom = Spacing.s8).clip(RoundedCornerShape(Radii.banner)).clickable(role = Role.Button) { onOpenClash(group) },
                style = BannerStyle.Error,
                icon = R.drawable.ic_warning,
            )
        }
        itemsIndexed(plan.rows, key = { i, _ -> "row-$i" }) { i, row ->
            TimelineEntry(
                row,
                lang,
                isFirst = i == 0,
                isLast = i == plan.rows.lastIndex,
                onClick = {
                    val group = groups.firstOrNull { row.item in it.items }
                    if (group != null) onOpenClash(group) else onOpenClass(row.item.activity)
                },
                isCardClass = date == now.toLocalDate() && i == plan.card?.index,
            )
        }
        if (groups.isNotEmpty()) {
            item(key = "hint") {
                Text(
                    stringResource(R.string.plan_clash_hint),
                    Modifier.padding(start = Spacing.s24 * 3, top = Spacing.s4),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

/** "Kolizja 12:45–13:30." in bold, then who overlaps and by how much. */
@Composable
private fun clashText(group: ClashGroup, lang: String): AnnotatedString {
    val lead = stringResource(R.string.plan_clash_lead, hm(group.from), hm(group.to))
    val rest = if (group.items.size == 2) {
        stringResource(
            R.string.plan_clash_pair,
            activityTitle(group.items[0].activity, lang),
            activityTitle(group.items[1].activity, lang),
            spanText(group.minutes),
        )
    } else {
        pluralStringResource(R.plurals.plan_clash_many, group.items.size, group.items.size)
    }
    return buildAnnotatedString {
        withStyle(SpanStyle(fontWeight = FontWeight.Bold)) { append(lead) }
        append(" ")
        append(rest)
    }
}
