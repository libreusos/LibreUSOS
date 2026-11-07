package io.gitlab.voxix.libreusos.ui

import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowLeft
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.data.TIMETABLE_PAGES
import io.gitlab.voxix.libreusos.data.dateOfPage
import io.gitlab.voxix.libreusos.data.mondayOf
import io.gitlab.voxix.libreusos.data.pageOf
import kotlinx.coroutines.launch
import org.libreusos.core.DataSource
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.TextStyle
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TimetableScreen(viewModel: TimetableViewModel, modifier: Modifier = Modifier) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { viewModel.updateToday() }

    val today = state.today
    if (today == null) {
        Box(modifier.fillMaxSize(), contentAlignment = Alignment.Center) { CircularProgressIndicator() }
        return
    }

    val locale = LocalConfiguration.current.locales[0]
    val pagerState = rememberPagerState(initialPage = pageOf(today)) { TIMETABLE_PAGES }
    val scope = rememberCoroutineScope()
    val selected = dateOfPage(pagerState.currentPage)
    val weekStart = mondayOf(selected)

    // Load the visible week and the weeks of the neighbouring days once the pager settles.
    LaunchedEffect(pagerState) {
        snapshotFlow { pagerState.settledPage }.collect { page ->
            val weeks = listOf(page - 1, page, page + 1)
                .filter { it in 0 until TIMETABLE_PAGES }
                .map { mondayOf(dateOfPage(it)) }
                .toSet()
            viewModel.ensureWeeks(weeks)
        }
    }
    // Back from the background: refetch the visible week when its live data is old.
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) {
        viewModel.ensureWeeks(setOf(mondayOf(dateOfPage(pagerState.currentPage))))
    }
    // The page index of "today" changes when the day rolls over while the app is open.
    LaunchedEffect(today) { viewModel.ensureWeeks(setOf(mondayOf(today))) }

    fun goTo(date: LocalDate) {
        scope.launch { pagerState.animateScrollToPage(pageOf(date)) }
    }

    Column(modifier.fillMaxSize()) {
        WeekHeader(
            selected = selected,
            today = today,
            locale = locale,
            onPrevious = { goTo(dateOfPage(pagerState.targetPage).minusWeeks(1)) },
            onNext = { goTo(dateOfPage(pagerState.targetPage).plusWeeks(1)) },
            onToday = { goTo(today) },
        )
        WeekStrip(
            weekStart = weekStart,
            selected = selected,
            today = today,
            locale = locale,
            hasClasses = { day -> state.weeks[weekStart]?.byDay?.containsKey(day) == true },
            onSelect = ::goTo,
        )
        val week = state.weeks[weekStart]
        if (week?.loading == true) LinearProgressIndicator(Modifier.fillMaxWidth()) else Spacer(Modifier.height(4.dp))
        SourceNote(week, state.zone ?: ZoneId.systemDefault())
        HorizontalPager(pagerState, Modifier.weight(1f).fillMaxWidth(), key = { it }) { page ->
            val date = dateOfPage(page)
            val start = mondayOf(date)
            DayPage(
                date = date,
                week = state.weeks[start],
                locale = locale,
                onRefresh = { viewModel.refresh(start) },
            )
        }
    }
}

@Composable
private fun WeekHeader(
    selected: LocalDate,
    today: LocalDate,
    locale: Locale,
    onPrevious: () -> Unit,
    onNext: () -> Unit,
    onToday: () -> Unit,
) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        IconButton(onPrevious) {
            Icon(Icons.AutoMirrored.Filled.KeyboardArrowLeft, stringResource(R.string.timetable_previous_week))
        }
        Text(
            DateTimeFormatter.ofPattern("LLLL yyyy", locale).format(selected).replaceFirstChar { it.titlecase(locale) },
            Modifier.weight(1f),
            style = MaterialTheme.typography.titleMedium,
            textAlign = TextAlign.Center,
        )
        IconButton(onNext) {
            Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, stringResource(R.string.timetable_next_week))
        }
        TextButton(onToday, enabled = selected != today) { Text(stringResource(R.string.timetable_today)) }
    }
}

@Composable
private fun WeekStrip(
    weekStart: LocalDate,
    selected: LocalDate,
    today: LocalDate,
    locale: Locale,
    hasClasses: (LocalDate) -> Boolean,
    onSelect: (LocalDate) -> Unit,
) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp)) {
        for (offset in 0L..6L) {
            val day = weekStart.plusDays(offset)
            DayCell(
                day = day,
                isSelected = day == selected,
                isToday = day == today,
                hasClasses = hasClasses(day),
                locale = locale,
                onClick = { onSelect(day) },
                modifier = Modifier.weight(1f),
            )
        }
    }
}

@Composable
private fun DayCell(
    day: LocalDate,
    isSelected: Boolean,
    isToday: Boolean,
    hasClasses: Boolean,
    locale: Locale,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = MaterialTheme.colorScheme
    val circle by animateColorAsState(if (isSelected) colors.primary else colors.surface, label = "dayCircle")
    val textColor = when {
        isSelected -> colors.onPrimary
        isToday -> colors.primary
        else -> colors.onSurface
    }
    Column(
        modifier
            .clip(RoundedCornerShape(16.dp))
            .clickable(onClick = onClick)
            .semantics { selected = isSelected }
            .padding(vertical = 6.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            day.dayOfWeek.getDisplayName(TextStyle.SHORT, locale),
            style = MaterialTheme.typography.labelSmall,
            color = if (isToday) colors.primary else colors.onSurfaceVariant,
            maxLines = 1,
        )
        Box(
            Modifier
                .padding(top = 4.dp)
                .size(36.dp)
                .clip(CircleShape)
                .background(circle)
                .then(if (isToday && !isSelected) Modifier.border(BorderStroke(1.5.dp, colors.primary), CircleShape) else Modifier),
            contentAlignment = Alignment.Center,
        ) {
            Text(day.dayOfMonth.toString(), style = MaterialTheme.typography.titleSmall, color = textColor)
        }
        Box(
            Modifier
                .padding(top = 4.dp)
                .size(5.dp)
                .clip(CircleShape)
                .background(if (hasClasses) colors.primary else colors.surface),
        )
    }
}

/**
 * Says why the data on screen is not live: "Cached at 14:32" while loading, "Offline, cached at 14:32"
 * when the fetch ended without live data, and a failed refresh over older data. Times are in [zone],
 * the installation's zone.
 */
@Composable
private fun SourceNote(week: WeekState?, zone: ZoneId) {
    if (week?.timetable == null) return
    if (week.error != null && !week.refreshFailed) {
        // For example a cache read that failed although the live fetch worked.
        Text(
            stringResource(R.string.error_prefix, stringResource(week.error.messageRes)),
            Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 2.dp),
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.error,
        )
        return
    }
    val cached = week.timetable.source is DataSource.Cached
    if (!cached && !week.refreshFailed) return
    val fetched = Instant.ofEpochSecond(week.dataAtSec ?: return).atZone(zone)
    val time = if (fetched.toLocalDate() == LocalDate.now(zone)) {
        DateTimeFormatter.ofPattern("HH:mm").format(fetched)
    } else {
        DateTimeFormatter.ofPattern("d MMM HH:mm", LocalConfiguration.current.locales[0]).format(fetched)
    }
    val failed = week.refreshFailed && !week.loading
    val text = when {
        failed -> stringResource(R.string.timetable_refresh_failed, time)
        week.loading -> stringResource(R.string.timetable_cached_at, time)
        else -> stringResource(R.string.timetable_offline_cached_at, time)
    }
    Text(
        text,
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 2.dp),
        style = MaterialTheme.typography.labelMedium,
        color = if (failed) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun DayPage(date: LocalDate, week: WeekState?, locale: Locale, onRefresh: () -> Unit) {
    val lang = locale.language
    val activities: List<KeyedActivity> = week?.byDay?.get(date).orEmpty()
    PullToRefreshBox(
        isRefreshing = week?.refreshing == true,
        onRefresh = onRefresh,
        modifier = Modifier.fillMaxSize(),
    ) {
        LazyColumn(
            Modifier.fillMaxSize(),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            item(key = "heading") {
                Text(
                    DateTimeFormatter.ofPattern("EEEE, d MMMM", locale).format(date)
                        .replaceFirstChar { it.titlecase(locale) },
                    style = MaterialTheme.typography.titleLarge,
                )
            }
            items(activities, key = { it.key }) { ActivityCard(it.activity, lang) }
            if (activities.isEmpty()) {
                item(key = "empty") {
                    val message = when {
                        week?.timetable != null -> stringResource(R.string.timetable_empty_day)
                        week?.error != null -> stringResource(R.string.error_prefix, stringResource(week.error.messageRes))
                        else -> null
                    }
                    if (message != null) {
                        Text(
                            message,
                            Modifier.fillMaxWidth().padding(vertical = 32.dp),
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            textAlign = TextAlign.Center,
                        )
                    }
                }
            }
        }
    }
}
