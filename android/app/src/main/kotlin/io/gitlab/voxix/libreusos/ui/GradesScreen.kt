package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.ui.components.GroupedListCard
import io.gitlab.voxix.libreusos.ui.components.KindBadge
import io.gitlab.voxix.libreusos.ui.components.WeekMotif
import io.gitlab.voxix.libreusos.ui.theme.LibreType
import io.gitlab.voxix.libreusos.ui.theme.Radii
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import io.gitlab.voxix.libreusos.ui.theme.tabular
import org.libreusos.core.CourseGrades
import org.libreusos.core.Grade
import java.util.Locale

/** The course list of one term, or the empty state while the term has no grades yet (SYSTEM.md section 12). */
@Composable
internal fun GradesContent(
    section: TermSection?,
    previousTerm: String?,
    onPreviousTerm: (String) -> Unit,
    onOpenCourse: (CourseGrades) -> Unit,
    locale: Locale,
) {
    val lang = locale.language
    val rows = remember(section, lang) { buildCourseRows(section, lang) }
    if (section == null || !section.hasGrades) {
        EmptyTermState(
            title = stringResource(R.string.grades_empty_title),
            body = if (rows.isEmpty()) {
                stringResource(R.string.grades_empty_body_none)
            } else {
                stringResource(R.string.grades_empty_body, pluralStringResource(R.plurals.grades_courses_count, rows.size, rows.size))
            },
            previousTerm = previousTerm,
            onPreviousTerm = onPreviousTerm,
        )
        return
    }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(bottom = Spacing.s24)) {
        ListHeader(pluralStringResource(R.plurals.grades_courses_count, rows.size, rows.size))
        GroupedListCard {
            rows.forEach { row -> row { CourseRowItem(row, locale) { onOpenCourse(row.course) } } }
        }
    }
}

/** The title above a list: what it holds, as plain text (the section label style is for settings-like lists). */
@Composable
internal fun ListHeader(title: String, modifier: Modifier = Modifier, trailing: String? = null) {
    Row(
        modifier.fillMaxWidth().padding(top = Spacing.s8, bottom = Spacing.s12),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(title, Modifier.weight(1f).semantics { heading() }, style = MaterialTheme.typography.titleMedium)
        if (trailing != null) {
            Text(trailing, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

/** A term with nothing to show: the week motif, a title, one sentence and a way to the term before. */
@Composable
internal fun EmptyTermState(title: String, body: String, previousTerm: String?, onPreviousTerm: (String) -> Unit) {
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(top = Spacing.s24)) {
        WeekMotif(legend = false)
        Spacer(Modifier.height(Spacing.s24))
        Text(title, style = MaterialTheme.typography.headlineSmall)
        Spacer(Modifier.height(Spacing.s8))
        Text(body, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (previousTerm != null) {
            Spacer(Modifier.height(Spacing.s24))
            FilledTonalButton({ onPreviousTerm(previousTerm) }) { Text(stringResource(R.string.grades_previous_term, previousTerm)) }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun CourseRowItem(row: CourseRow, locale: Locale, onClick: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    Row(
        Modifier
            .fillMaxWidth()
            .defaultMinSize(minHeight = Spacing.rowMinHeight)
            .clickable(role = Role.Button, onClick = onClick)
            .padding(horizontal = Spacing.rowPaddingHorizontal, vertical = Spacing.rowPaddingVertical),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(Spacing.s12),
    ) {
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(Spacing.s4)) {
            Text(row.name.ifBlank { stringResource(R.string.home_unnamed) }, style = MaterialTheme.typography.bodyLarge)
            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(Spacing.s8),
                verticalArrangement = Arrangement.spacedBy(Spacing.s4),
                itemVerticalAlignment = Alignment.CenterVertically,
            ) {
                row.kinds.forEach { KindBadge(it) }
                row.course.ects?.let {
                    Text(
                        stringResource(R.string.grades_ects, formatEcts(it, locale)),
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.onSurfaceVariant,
                    )
                }
                row.final?.earlier?.symbol?.takeIf { it.isNotBlank() }?.let { earlier ->
                    Text(
                        buildAnnotatedString {
                            append(stringResource(R.string.grades_session_first) + " ")
                            withStyle(SpanStyle(textDecoration = TextDecoration.LineThrough)) { append(earlier) }
                        },
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.onSurfaceVariant,
                    )
                }
            }
        }
        GradeTile(row.final?.grade, highlight = row.best)
    }
}

/** ECTS points: `6` for whole numbers, else one decimal (`4.5`). */
internal fun formatEcts(value: Double, locale: Locale): String =
    if (value % 1.0 == 0.0) value.toInt().toString() else String.format(locale, "%.1f", value)

/**
 * The grade tile: 44 x 40, radius 10. Default `surfaceContainerHigh`, [highlight] (the term's best) `primaryContainer`,
 * a failed grade `errorContainer`; no grade yet is a dashed outline with a dash. ZAL and NZAL are set in mono.
 */
@Composable
internal fun GradeTile(grade: Grade?, modifier: Modifier = Modifier, highlight: Boolean = false) {
    val colors = MaterialTheme.colorScheme
    val shape = RoundedCornerShape(Radii.control)
    val symbol = grade?.symbol?.takeIf { it.isNotBlank() }
    val description = if (symbol != null) stringResource(R.string.grades_a11y_grade, symbol) else stringResource(R.string.grades_a11y_no_grade)
    val (container, content) = when {
        symbol == null -> Color.Transparent to colors.onSurfaceVariant
        grade.passes == false -> colors.errorContainer to colors.onErrorContainer
        highlight -> colors.primaryContainer to colors.onPrimaryContainer
        else -> colors.surfaceContainerHigh to colors.onSurface
    }
    val dashColor = colors.outlineVariant
    val dashed = if (symbol == null) {
        Modifier.drawBehind {
            drawRoundRect(
                dashColor,
                cornerRadius = CornerRadius(Radii.control.toPx()),
                style = Stroke(1.dp.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(4.dp.toPx(), 3.dp.toPx()))),
            )
        }
    } else {
        Modifier
    }
    Box(
        modifier
            .semantics(mergeDescendants = true) { contentDescription = description }
            .widthIn(min = 44.dp)
            .height(40.dp)
            .background(container, shape)
            .then(dashed)
            .padding(horizontal = Spacing.s8),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            symbol ?: "\u2014",
            color = content,
            style = if (grade?.numericValue == null && symbol != null) LibreType.monoLarge else MaterialTheme.typography.titleLarge.tabular(),
            textAlign = TextAlign.Center,
        )
    }
}
