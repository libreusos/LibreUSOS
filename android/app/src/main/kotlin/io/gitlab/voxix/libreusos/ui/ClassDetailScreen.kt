package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.data.clockTime
import io.gitlab.voxix.libreusos.data.pick
import io.gitlab.voxix.libreusos.data.shortBuildingLabel
import io.gitlab.voxix.libreusos.ui.components.GroupedListCard
import io.gitlab.voxix.libreusos.ui.components.GroupedListRow
import io.gitlab.voxix.libreusos.ui.components.KindBadge
import io.gitlab.voxix.libreusos.ui.components.SectionHeader
import io.gitlab.voxix.libreusos.ui.components.StatusBadge
import io.gitlab.voxix.libreusos.ui.components.StatusBadgeStyle
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import io.gitlab.voxix.libreusos.ui.theme.kindCodeOf
import io.gitlab.voxix.libreusos.widget.timed
import kotlinx.coroutines.delay
import org.libreusos.core.Activity
import org.libreusos.core.ActivityKind
import org.libreusos.core.Frequency
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

/** How many of the group's later meetings are listed before "Jeszcze N". */
private const val UPCOMING_SHOWN = 4

/** One class: time, place, group and the other meetings of its group from the cached timetable. */
@Composable
fun ClassDetailScreen(viewModel: TimetableViewModel, onBack: () -> Unit, modifier: Modifier = Modifier) {
    val activity by viewModel.openClass.collectAsStateWithLifecycle()
    val state by viewModel.state.collectAsStateWithLifecycle()
    val a = activity
    if (a == null) {
        LaunchedEffect(Unit) { onBack() }
        return
    }
    val locale = LocalConfiguration.current.locales[0]
    val lang = locale.language
    val zone = state.zone ?: ZoneId.systemDefault()
    var now by remember { mutableStateOf(LocalDateTime.now(zone)) }
    LaunchedEffect(zone) {
        while (true) {
            now = LocalDateTime.now(zone)
            delay(30_000L)
        }
    }
    val meetings by produceState<GroupMeetings?>(null, a) { value = viewModel.meetingsOf(a) }
    val span = timed(listOf(a)).firstOrNull()
    val card = span?.let { todayPlan(listOf(a), it.start.toLocalDate(), now).card }
    val colors = MaterialTheme.colorScheme

    Column(modifier.fillMaxSize()) {
        IconButton(onBack, Modifier.padding(start = Spacing.s4, top = Spacing.s8)) {
            Icon(painterResource(R.drawable.ic_arrow_back), stringResource(R.string.action_back))
        }
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = Spacing.screenEdge).padding(bottom = Spacing.s24),
            verticalArrangement = Arrangement.spacedBy(Spacing.s8),
        ) {
            Row(horizontalArrangement = Arrangement.spacedBy(Spacing.s6), verticalAlignment = Alignment.CenterVertically) {
                KindBadge(kindCodeOf(a.classtypeId, a.classtypeName.pl ?: a.classtypeName.en, a.kind == ActivityKind.Exam))
                if (card?.current == true) StatusBadge(stringResource(R.string.today_badge_now), StatusBadgeStyle.Now)
                val group = a.groupNumber?.let { stringResource(R.string.today_group, it.toInt()) }
                val type = listOfNotNull(a.classtypeName.pick(lang).takeIf { it.isNotBlank() }?.replaceFirstChar { it.titlecase(locale) }, group)
                    .joinToString(" · ")
                if (type.isNotEmpty()) Text(type, style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceVariant)
            }
            Text(activityTitle(a, lang), style = MaterialTheme.typography.headlineLarge)
            Text(
                whenText(a, span?.start, locale),
                style = MaterialTheme.typography.titleMedium,
                color = colors.onSurfaceVariant,
            )
            if (card != null && card.current) {
                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.s16), verticalAlignment = Alignment.CenterVertically) {
                    LinearProgressIndicator(
                        progress = { card.progress },
                        Modifier.weight(1f).semantics { progressBarRangeInfo = ProgressBarRangeInfo(card.progress, 0f..1f) },
                    )
                    Text(stringResource(R.string.today_remaining, spanText(card.minutes)), style = MaterialTheme.typography.labelLarge)
                }
            }
            Spacer(Modifier.height(Spacing.s8))
            DetailRows(a, lang, meetings, locale)
            val later = meetings?.upcoming.orEmpty()
            if (later.isNotEmpty()) {
                SectionHeader(stringResource(R.string.detail_next_meetings))
                GroupedListCard {
                    later.take(UPCOMING_SHOWN).forEach { next ->
                        row {
                            GroupedListRow(
                                title = shortDate(next, locale),
                                subtitle = listOf("${clockTime(next.start)}–${clockTime(next.end)}", placeLine(next, includeGroup = false))
                                    .filter { it.isNotEmpty() }.joinToString(" · "),
                                onClick = { viewModel.showClass(next) },
                            )
                        }
                    }
                    val more = later.size - UPCOMING_SHOWN
                    if (more > 0) row { GroupedListRow(pluralStringResource(R.plurals.today_more_classes, more, more)) }
                }
            }
        }
    }
}

/** Place, group and how often the group meets: what the cache holds, and nothing invented. */
@Composable
private fun DetailRows(a: Activity, lang: String, meetings: GroupMeetings?, locale: Locale) {
    val room = a.roomNumber?.takeIf { it.isNotBlank() }?.let { stringResource(R.string.detail_room, it) }
    val building = shortBuildingLabel(a.buildingId)?.let { stringResource(R.string.detail_building, it) }
    val where = listOfNotNull(room, building).joinToString(", ")
    val group = a.groupNumber
    val total = meetings?.total ?: 0
    if (where.isEmpty() && group == null && total <= 1) return
    GroupedListCard {
        if (where.isNotEmpty()) row {
            GroupedListRow(where, icon = R.drawable.ic_location_on, subtitle = a.buildingName.pick(lang).takeIf { it.isNotBlank() })
        }
        if (group != null) row {
            GroupedListRow(stringResource(R.string.detail_group, group.toInt()), icon = R.drawable.ic_group)
        }
        if (meetings != null && total > 1) row {
            val frequency = a.frequency
            val title = when (frequency) {
                null -> stringResource(R.string.detail_group_meetings)
                Frequency.EveryWeek -> stringResource(R.string.detail_every_week)
                else -> frequencyLabel(frequency) ?: stringResource(R.string.detail_group_meetings)
            }
            val next = meetings.upcoming.firstOrNull()?.let { timed(listOf(it)).firstOrNull() }
                ?.let { stringResource(R.string.detail_next, DateTimeFormatter.ofPattern("d MMM, HH:mm", locale).format(it.start)) }
            GroupedListRow(
                title,
                icon = R.drawable.ic_calendar_today,
                subtitle = listOfNotNull(stringResource(R.string.detail_position, meetings.position, total), next).joinToString(" · "),
            )
        }
    }
}

private fun capitalised(text: String, locale: Locale) = text.replaceFirstChar { it.titlecase(locale) }

/** "Wtorek, 10 listopada · 10:15–11:45". */
private fun whenText(a: Activity, start: LocalDateTime?, locale: Locale): String {
    val day = start?.let { capitalised(DateTimeFormatter.ofPattern("EEEE, d MMMM", locale).format(it), locale) }
    return listOfNotNull(day, "${clockTime(a.start)}–${clockTime(a.end)}").joinToString(" · ")
}

/** "czw. 17 lis" for a meeting row. */
private fun shortDate(a: Activity, locale: Locale): String {
    val start = timed(listOf(a)).firstOrNull()?.start ?: return a.date
    return capitalised(DateTimeFormatter.ofPattern("EEE d MMM", locale).format(start), locale)
}
