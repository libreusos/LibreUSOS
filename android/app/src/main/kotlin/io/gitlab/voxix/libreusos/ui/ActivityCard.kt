package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.data.clockTime
import io.gitlab.voxix.libreusos.data.pick
import io.gitlab.voxix.libreusos.data.shortBuildingLabel
import org.libreusos.core.Activity
import org.libreusos.core.ActivityKind
import org.libreusos.core.Frequency

/** One timetable entry: time on the left, the course as the title, and small labels below it. */
@Composable
fun ActivityCard(activity: Activity, lang: String, modifier: Modifier = Modifier) {
    Row(modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Column(Modifier.width(52.dp).padding(top = 12.dp)) {
            Text(
                clockTime(activity.start),
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.primary,
            )
            Text(
                clockTime(activity.end),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Card(
            Modifier.weight(1f),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh),
        ) {
            Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                val title = activity.courseName.pick(lang)
                    .ifBlank { activity.name.pick(lang) }
                    .ifBlank { stringResource(R.string.home_unnamed) }
                Text(title, style = MaterialTheme.typography.titleSmall)
                Labels(activity, lang)
                Place(activity, lang)
            }
        }
    }
}

@Composable
private fun Labels(activity: Activity, lang: String) {
    val type = activity.classtypeName.pick(lang).ifBlank {
        when (activity.kind) {
            ActivityKind.Exam -> stringResource(R.string.kind_exam)
            ActivityKind.Meeting -> stringResource(R.string.kind_meeting)
            else -> ""
        }
    }
    val frequency = activity.frequency?.let { frequencyLabel(it) }
    val group = activity.groupNumber?.let { stringResource(R.string.activity_group, it.toInt()) }
    if (type.isBlank() && frequency == null && group == null && !activity.clashes) return
    FlowRow(
        horizontalArrangement = Arrangement.spacedBy(6.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        if (type.isNotBlank()) {
            Tag(type, MaterialTheme.colorScheme.secondaryContainer, MaterialTheme.colorScheme.onSecondaryContainer)
        }
        if (group != null) Tag(group, Color.Transparent, MaterialTheme.colorScheme.onSurfaceVariant)
        if (frequency != null) Tag(frequency, Color.Transparent, MaterialTheme.colorScheme.onSurfaceVariant)
        if (activity.clashes) {
            Tag(
                stringResource(R.string.activity_overlaps),
                MaterialTheme.colorScheme.tertiaryContainer,
                MaterialTheme.colorScheme.onTertiaryContainer,
            )
        }
    }
}

@Composable
private fun Tag(text: String, container: Color, content: Color) {
    Surface(color = container, contentColor = content, shape = RoundedCornerShape(8.dp)) {
        Text(
            text,
            Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
            style = MaterialTheme.typography.labelMedium,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

/** Room number and short building label on one line, the long building name below it. */
@Composable
private fun Place(activity: Activity, lang: String) {
    val room = activity.roomNumber?.takeIf { it.isNotBlank() }
        ?.let { stringResource(R.string.activity_room, it) }
    val building = shortBuildingLabel(activity.buildingId)
    val line = listOfNotNull(room, building).joinToString(" · ")
    val buildingName = activity.buildingName.pick(lang)
    if (line.isNotEmpty()) {
        Text(line, style = MaterialTheme.typography.bodyMedium)
    }
    if (buildingName.isNotBlank()) {
        Text(
            buildingName,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

/** The chip text for how often a class meets; null for every week, which is the usual case. */
@Composable
private fun frequencyLabel(frequency: Frequency): String? = when (frequency) {
    Frequency.EveryWeek -> null
    Frequency.EveryFortnight -> stringResource(R.string.frequency_fortnight)
    Frequency.FortnightOdd -> stringResource(R.string.frequency_fortnight_odd)
    Frequency.FortnightEven -> stringResource(R.string.frequency_fortnight_even)
    Frequency.EveryMonth -> stringResource(R.string.frequency_month)
    Frequency.EveryWorkingDay -> stringResource(R.string.frequency_working_day)
    Frequency.EveryWeekend -> stringResource(R.string.frequency_weekend)
    Frequency.EveryDay -> stringResource(R.string.frequency_day)
    Frequency.Once -> stringResource(R.string.frequency_once)
    Frequency.Irregular -> stringResource(R.string.frequency_irregular)
    is Frequency.Other -> frequency.raw.takeIf { it.isNotBlank() }
}
