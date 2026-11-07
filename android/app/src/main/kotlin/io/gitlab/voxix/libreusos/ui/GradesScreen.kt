package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.data.pick
import org.libreusos.core.CourseGrades
import org.libreusos.core.Grade
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GradesScreen(viewModel: GradesViewModel, modifier: Modifier = Modifier) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    // First visit and every return from the background; the model skips fresh live data.
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { viewModel.ensureLoaded() }

    val locale = LocalConfiguration.current.locales[0]
    val lang = locale.language
    val rows = remember(state.sections, lang) { state.sections?.let { buildRows(it, lang) }.orEmpty() }

    Column(modifier.fillMaxSize()) {
        if (state.loading) LinearProgressIndicator(Modifier.fillMaxWidth()) else Spacer(Modifier.height(4.dp))
        SourceNote(state, locale)
        PullToRefreshBox(
            isRefreshing = state.refreshing,
            onRefresh = viewModel::refresh,
            modifier = Modifier.weight(1f).fillMaxWidth(),
        ) {
            if (state.sections == null && state.error == null) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { CircularProgressIndicator() }
            }
            LazyColumn(
                Modifier.fillMaxSize(),
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                items(rows, key = { it.key }) { row ->
                    when (row) {
                        is GradesRow.Header -> TermHeader(row, locale)
                        is GradesRow.Course -> CourseCard(row, lang)
                    }
                }
                if (state.sections != null && rows.isEmpty()) item(key = "empty") {
                    CenteredNote(stringResource(R.string.grades_empty))
                }
                val error = state.error
                if (state.sections == null && error != null) item(key = "error") {
                    CenteredNote(stringResource(R.string.error_prefix, stringResource(error.messageRes)))
                }
            }
        }
    }
}

@Composable
private fun CenteredNote(text: String) {
    Text(
        text,
        Modifier.fillMaxWidth().padding(vertical = 32.dp),
        style = MaterialTheme.typography.bodyLarge,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        textAlign = TextAlign.Center,
    )
}

/** Says why the data on screen is not live, like the timetable does. Times are in the installation's zone. */
@Composable
private fun SourceNote(state: GradesUiState, locale: Locale) {
    if (state.sections == null) return
    val error = state.error
    if (error != null && !state.refreshFailed) {
        // For example a cache read that failed although the live fetch worked.
        NoteText(stringResource(R.string.error_prefix, stringResource(error.messageRes)), isError = true)
        return
    }
    if (state.liveAtMs != null && !state.refreshFailed) return
    val zone: ZoneId = state.zone ?: return
    val fetched = Instant.ofEpochSecond(state.dataAtSec ?: return).atZone(zone)
    val time = if (fetched.toLocalDate() == LocalDate.now(zone)) {
        DateTimeFormatter.ofPattern("HH:mm").format(fetched)
    } else {
        DateTimeFormatter.ofPattern("d MMM HH:mm", locale).format(fetched)
    }
    val failed = state.refreshFailed && !state.loading
    val text = when {
        failed -> stringResource(R.string.grades_refresh_failed, time)
        state.loading -> stringResource(R.string.grades_cached_at, time)
        else -> stringResource(R.string.grades_offline_cached_at, time)
    }
    NoteText(text, isError = failed)
}

@Composable
private fun NoteText(text: String, isError: Boolean) {
    Text(
        text,
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 2.dp),
        style = MaterialTheme.typography.labelMedium,
        color = if (isError) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@Composable
private fun TermHeader(header: GradesRow.Header, locale: Locale) {
    Column(Modifier.fillMaxWidth().padding(top = 8.dp)) {
        Text(header.title, style = MaterialTheme.typography.titleLarge)
        // The core computes these; they are shown as given.
        val averages = listOfNotNull(
            header.average?.let { stringResource(R.string.grades_average, formatNumber(it, locale)) },
            header.weightedAverage?.let { stringResource(R.string.grades_weighted_average, formatNumber(it, locale)) },
        )
        if (averages.isNotEmpty()) {
            Text(
                averages.joinToString("  ·  "),
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

internal fun formatNumber(value: Double, locale: Locale): String = String.format(locale, "%.2f", value)

/** ECTS points: `6` for whole numbers, else one decimal (`4.5`). */
private fun formatEcts(value: Double, locale: Locale): String =
    if (value % 1.0 == 0.0) value.toInt().toString() else String.format(locale, "%.1f", value)

@Composable
private fun outcomeColors(outcome: Outcome): Pair<Color, Color> {
    val c = MaterialTheme.colorScheme
    return when (outcome) {
        Outcome.PASS -> c.primaryContainer to c.onPrimaryContainer
        Outcome.FAIL -> c.errorContainer to c.onErrorContainer
        Outcome.UNKNOWN -> c.surfaceContainerHigh to c.onSurface
    }
}

@Composable
private fun CourseCard(row: GradesRow.Course, lang: String) {
    val course: CourseGrades = row.course
    val (container, content) = outcomeColors(courseOutcome(course))
    Card(colors = CardDefaults.cardColors(containerColor = container, contentColor = content)) {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(
                row.name.ifBlank { stringResource(R.string.home_unnamed) },
                style = MaterialTheme.typography.titleSmall,
            )
            course.ects?.let {
                Text(
                    stringResource(R.string.grades_ects, formatEcts(it, LocalConfiguration.current.locales[0])),
                    style = MaterialTheme.typography.labelMedium,
                )
            }
            gradeReports(course.grades).flatten().forEach { GradeLine(null, it, lang) }
            course.units.filter { it.grades.isNotEmpty() }.forEach { unit ->
                val type = unit.classtypeName.pick(lang)
                gradeReports(unit.grades).flatten().forEach { GradeLine(type.ifBlank { null }, it, lang) }
            }
        }
    }
}

/** One session of one grade: the sitting, the symbol in a tinted chip, and passed or failed. */
@Composable
private fun GradeLine(classType: String?, grade: Grade, lang: String) {
    val sitting = when (val label = sessionLabel(grade.session)) {
        SessionLabel.First -> stringResource(R.string.grades_session_first)
        SessionLabel.Retake -> stringResource(R.string.grades_session_retake)
        is SessionLabel.Later -> stringResource(R.string.grades_session_later, label.number)
    }
    val outcome = outcomeOf(grade)
    // Stronger than the card tint, so the chip stands out on its card.
    val colors = MaterialTheme.colorScheme
    val (chipColor, onChip) = when (outcome) {
        Outcome.PASS -> colors.primary to colors.onPrimary
        Outcome.FAIL -> colors.error to colors.onError
        Outcome.UNKNOWN -> colors.surfaceVariant to colors.onSurfaceVariant
    }
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Column(Modifier.weight(1f)) {
            Text(
                listOfNotNull(classType, sitting).joinToString(" · "),
                style = MaterialTheme.typography.bodyMedium,
            )
            val detail = listOfNotNull(
                when (outcome) {
                    Outcome.PASS -> stringResource(R.string.grades_passed)
                    Outcome.FAIL -> stringResource(R.string.grades_failed)
                    Outcome.UNKNOWN -> null
                },
                grade.description.pick(lang).ifBlank { null },
            )
            if (detail.isNotEmpty()) {
                Text(detail.joinToString(" · "), style = MaterialTheme.typography.labelSmall)
            }
        }
        Surface(shape = RoundedCornerShape(12.dp), color = chipColor, contentColor = onChip) {
            Text(
                grade.symbol?.ifBlank { null } ?: stringResource(R.string.grades_no_symbol),
                Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}
