package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.ui.components.GroupedListCard
import io.gitlab.voxix.libreusos.ui.components.GroupedListRow
import io.gitlab.voxix.libreusos.ui.theme.Radii
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import io.gitlab.voxix.libreusos.ui.theme.tabular
import java.time.format.DateTimeFormatter
import java.util.Locale

/** Cells per row of partial results. */
private const val CELLS_PER_ROW = 4

/** The tests of one term: cards with points, then the tests whose points are not available (SYSTEM.md section 12). */
@Composable
internal fun TestsContent(
    section: TestTermSection?,
    skipped: Int,
    previousTerm: String?,
    onPreviousTerm: (String) -> Unit,
    locale: Locale,
) {
    val cards = remember(section, locale) { buildTestCards(section, locale.language, locale) }
    if (cards.isEmpty()) {
        EmptyTermState(
            title = stringResource(R.string.tests_empty_title),
            body = stringResource(R.string.tests_empty_body),
            previousTerm = previousTerm,
            onPreviousTerm = onPreviousTerm,
        )
        return
    }
    val (unavailable, available) = cards.partition { it.unavailable != null }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(bottom = Spacing.s24)) {
        if (available.isNotEmpty()) {
            ListHeader(
                stringResource(R.string.tests_section_points),
                trailing = pluralStringResource(R.plurals.tests_count, available.size, available.size),
            )
            GroupedListCard { available.forEach { card -> row { TestCardItem(card, locale) } } }
        }
        if (unavailable.isNotEmpty()) {
            Spacer(Modifier.height(Spacing.s20))
            ListHeader(stringResource(R.string.tests_section_unavailable))
            GroupedListCard {
                unavailable.forEach { card ->
                    row {
                        val hidden = card.unavailable == Unavailable.Hidden
                        GroupedListRow(
                            title = listOf(card.courseName, card.title).filter { it.isNotBlank() }.joinToString(" · "),
                            subtitle = stringResource(if (hidden) R.string.tests_reason_hidden else R.string.tests_reason_groups),
                            icon = if (hidden) R.drawable.ic_visibility_off else R.drawable.ic_lock_clock,
                        )
                    }
                }
            }
        }
        if (skipped > 0) {
            Text(
                stringResource(R.string.tests_skipped),
                Modifier.padding(top = Spacing.s12),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun TestCardItem(card: TestCard, locale: Locale) {
    val colors = MaterialTheme.colorScheme
    Column(
        Modifier.fillMaxWidth().padding(horizontal = Spacing.rowPaddingHorizontal, vertical = Spacing.rowPaddingVertical),
        verticalArrangement = Arrangement.spacedBy(Spacing.s8),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Spacing.s8)) {
            Text(
                card.courseName,
                Modifier.weight(1f),
                style = MaterialTheme.typography.bodyMedium,
                color = colors.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            card.changed?.let {
                Text(
                    DateTimeFormatter.ofPattern("d MMM", locale).format(it),
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.onSurfaceVariant,
                )
            }
        }
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Spacing.s12)) {
            Text(card.title, Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge)
            card.points?.let { Text(it, style = MaterialTheme.typography.titleLarge.tabular()) }
        }
        card.progress?.let { ProgressBar(it) }
        // shortcut: only the first level of the test tree, until a test detail screen exists.
        card.cells.chunked(CELLS_PER_ROW).forEach { line ->
            Row(horizontalArrangement = Arrangement.spacedBy(Spacing.s8)) {
                line.forEach { cell -> PartialCell(cell, Modifier.weight(1f)) }
                repeat(CELLS_PER_ROW - line.size) { Spacer(Modifier.weight(1f)) }
            }
        }
    }
}

/** The 4 dp bar of points over the maximum. Decorative: the points are said in text next to it. */
@Composable
private fun ProgressBar(progress: Float) {
    Box(
        Modifier.fillMaxWidth().height(4.dp).clip(CircleShape).background(MaterialTheme.colorScheme.surfaceContainerHigh)
            .clearAndSetSemantics { },
    ) {
        Box(Modifier.fillMaxWidth(progress).fillMaxHeight().background(MaterialTheme.colorScheme.primary))
    }
}

@Composable
private fun PartialCell(cell: TestCell, modifier: Modifier = Modifier) {
    val hiddenText = stringResource(R.string.tests_a11y_hidden)
    Column(
        modifier
            .background(MaterialTheme.colorScheme.surfaceContainerHigh, RoundedCornerShape(Radii.control))
            .padding(vertical = Spacing.s8, horizontal = Spacing.s4)
            .semantics(mergeDescendants = true) { if (cell.hidden) contentDescription = "${cell.label}, $hiddenText" },
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            cell.label,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            textAlign = TextAlign.Center,
        )
        if (cell.hidden) {
            Icon(painterResource(R.drawable.ic_visibility_off), null, Modifier.size(20.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
        } else {
            Text(cell.value.ifBlank { "–" }, style = MaterialTheme.typography.titleMedium.tabular(), maxLines = 1)
        }
    }
}
