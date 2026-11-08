package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.data.pick
import io.gitlab.voxix.libreusos.ui.components.Banner
import io.gitlab.voxix.libreusos.ui.components.DrillDownScreen
import io.gitlab.voxix.libreusos.ui.components.GroupedListCard
import io.gitlab.voxix.libreusos.ui.components.KindBadge
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import io.gitlab.voxix.libreusos.ui.theme.kindCodeOf
import io.gitlab.voxix.libreusos.ui.theme.tabular
import org.libreusos.core.Grade
import java.time.format.DateTimeFormatter
import java.util.Locale

/**
 * One course of a term: the final grade and every sitting by class type. Shows what the core gives and
 * nothing computed. shortcut: the course is the copy from when it was opened, it does not follow a refresh.
 */
@Composable
fun GradeDetailScreen(viewModel: GradesViewModel, onBack: () -> Unit, modifier: Modifier = Modifier) {
    val open by viewModel.openCourse.collectAsStateWithLifecycle()
    val locale = LocalConfiguration.current.locales[0]
    val lang = locale.language
    val course = open?.course
    DrillDownScreen(course?.name?.pick(lang).orEmpty().ifBlank { stringResource(R.string.home_unnamed) }, onBack, modifier) { padding ->
        val shown = open ?: return@DrillDownScreen
        val grades = shown.course
        val final = finalGradeOf(grades)
        val meta = listOfNotNull(shown.termId, grades.ects?.let { stringResource(R.string.grades_ects, formatEcts(it, locale)) })
        Column(
            Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(horizontal = Spacing.screenEdge),
            verticalArrangement = Arrangement.spacedBy(Spacing.s16),
        ) {
            Text(meta.joinToString(" · "), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            if (final == null) {
                Text(stringResource(R.string.grade_detail_none), style = MaterialTheme.typography.bodyLarge)
            } else {
                FinalHeader(final, locale, lang)
                val earlier = final.earlier
                if (earlier != null && earlier.passes == false) {
                    Banner(
                        stringResource(R.string.grade_detail_retake_note, earlier.symbol.orEmpty(), final.grade.symbol.orEmpty()),
                        icon = R.drawable.ic_info,
                    )
                }
            }
            val courseSessions = gradeReports(grades.grades).flatten()
            val units = grades.units.filter { it.grades.isNotEmpty() }
            if (courseSessions.size > 1 || units.isNotEmpty()) {
                GroupedListCard {
                    if (courseSessions.size > 1) row { SessionGroup(null, stringResource(R.string.grade_detail_course_level), courseSessions, locale, lang) }
                    units.forEach { unit ->
                        row {
                            SessionGroup(
                                kindCodeOf(unit.classtypeId, unit.classtypeName.pick(lang).ifBlank { null }),
                                unit.classtypeName.pick(lang),
                                gradeReports(unit.grades).flatten(),
                                locale,
                                lang,
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun FinalHeader(final: FinalGrade, locale: Locale, lang: String) {
    val grade = final.grade
    val date = gradeDate(grade)?.let { DateTimeFormatter.ofPattern("d MMM yyyy", locale).format(it) }
    val description = grade.description.pick(lang).ifBlank { null }
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Spacing.s16)) {
        Text(
            grade.symbol?.ifBlank { null } ?: "—",
            style = MaterialTheme.typography.displaySmall.tabular(),
        )
        Column(Modifier.weight(1f)) {
            Text(
                if (description != null) stringResource(R.string.grade_detail_final_with, description) else stringResource(R.string.grade_detail_final),
                style = MaterialTheme.typography.bodyLarge,
            )
            if (date != null) {
                Text(
                    stringResource(if (final.earlier != null) R.string.grade_detail_after_retake else R.string.grade_detail_issued, date),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

/** A class type with the sittings of its grades, one line each: the sitting, its date, and the grade tile. */
@Composable
private fun SessionGroup(kind: io.gitlab.voxix.libreusos.ui.theme.KindCode?, title: String, sessions: List<Grade>, locale: Locale, lang: String) {
    Column(
        Modifier.fillMaxWidth().padding(horizontal = Spacing.rowPaddingHorizontal, vertical = Spacing.rowPaddingVertical),
        verticalArrangement = Arrangement.spacedBy(Spacing.s12),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Spacing.s8)) {
            if (kind != null) KindBadge(kind)
            if (title.isNotBlank()) Text(title, style = MaterialTheme.typography.bodyLarge)
        }
        sessions.forEach { grade ->
            val label = when (val s = sessionLabel(grade.session)) {
                SessionLabel.First -> stringResource(R.string.grades_session_first)
                SessionLabel.Retake -> stringResource(R.string.grades_session_retake)
                is SessionLabel.Later -> stringResource(R.string.grades_session_later, s.number)
            }
            val detail = listOfNotNull(
                gradeDate(grade)?.let { DateTimeFormatter.ofPattern("d MMM yyyy", locale).format(it) },
                grade.description.pick(lang).ifBlank { null },
            ).joinToString(" · ")
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Spacing.s12)) {
                Column(Modifier.weight(1f)) {
                    Text(label, style = MaterialTheme.typography.bodyLarge)
                    if (detail.isNotEmpty()) {
                        Text(detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
                GradeTile(grade)
            }
        }
    }
}
