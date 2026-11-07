package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
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
import androidx.compose.ui.draw.alpha
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
import org.libreusos.core.TestNode
import org.libreusos.core.TestNodeKind
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TestsScreen(viewModel: TestsViewModel, modifier: Modifier = Modifier) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    // First visit and every return from the background; the model skips fresh live data.
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { viewModel.ensureLoaded() }

    val locale = LocalConfiguration.current.locales[0]
    val lang = locale.language
    val rows = remember(state.sections, state.toggled, lang) {
        state.sections?.let { buildTestRows(it, state.toggled, lang) }.orEmpty()
    }

    Column(modifier.fillMaxSize()) {
        if (state.loading) LinearProgressIndicator(Modifier.fillMaxWidth()) else Spacer(Modifier.height(4.dp))
        TestsNote(state, locale)
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
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                items(rows, key = { it.key }) { row ->
                    when (row) {
                        is TestsRow.Header -> TestsTermHeader(row)
                        is TestsRow.Course -> CourseTestHeader(row, locale) { viewModel.toggle(row.key) }
                        is TestsRow.Node -> TestNodeRow(row, locale, lang) { viewModel.toggle(row.key) }
                    }
                }
                if (state.sections != null && rows.isEmpty()) item(key = "empty") {
                    TestsCenteredNote(stringResource(R.string.tests_empty))
                }
                val error = state.error
                if (state.sections == null && error != null) item(key = "error") {
                    TestsCenteredNote(stringResource(R.string.error_prefix, stringResource(error.messageRes)))
                }
            }
        }
    }
}

@Composable
private fun TestsCenteredNote(text: String) {
    Text(
        text,
        Modifier.fillMaxWidth().padding(vertical = 32.dp),
        style = MaterialTheme.typography.bodyLarge,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        textAlign = TextAlign.Center,
    )
}

/** Says why the data on screen is not live, like the grades do, and how many tests USOS refused. */
@Composable
private fun TestsNote(state: TestsUiState, locale: Locale) {
    if (state.sections == null) return
    if (state.skipped + state.failedTerms > 0 && !state.loading) TestsNoteText(stringResource(R.string.tests_skipped), isError = false)
    val error = state.error
    if (error != null && !state.refreshFailed) {
        TestsNoteText(stringResource(R.string.error_prefix, stringResource(error.messageRes)), isError = true)
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
        failed -> stringResource(R.string.tests_refresh_failed, time)
        state.loading -> stringResource(R.string.tests_cached_at, time)
        else -> stringResource(R.string.tests_offline_cached_at, time)
    }
    TestsNoteText(text, isError = failed)
}

@Composable
private fun TestsNoteText(text: String, isError: Boolean) {
    Text(
        text,
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 2.dp),
        style = MaterialTheme.typography.labelMedium,
        color = if (isError) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@Composable
private fun TestsTermHeader(header: TestsRow.Header) {
    Text(header.title, Modifier.fillMaxWidth().padding(top = 12.dp, bottom = 4.dp), style = MaterialTheme.typography.titleLarge)
}

@Composable
private fun ExpandIcon(expandable: Boolean, expanded: Boolean) {
    if (!expandable) {
        Spacer(Modifier.size(24.dp))
        return
    }
    Icon(
        if (expanded) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown,
        contentDescription = null,
        Modifier.size(24.dp),
    )
}

@Composable
private fun CourseTestHeader(row: TestsRow.Course, locale: Locale, onToggle: () -> Unit) {
    val root = row.test.root
    // The core totals the root; with nothing to show (no points, no maximum) there is no sum.
    val total = pointsLabel(root.points, root.pointsMax, locale)
    val toggleLabel = stringResource(if (row.expanded) R.string.tests_collapse else R.string.tests_expand)
    Surface(
        Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(12.dp),
        color = MaterialTheme.colorScheme.secondaryContainer,
        contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
    ) {
        Row(
            Modifier
                .then(if (row.expandable) Modifier.clickable(onClickLabel = toggleLabel, onClick = onToggle) else Modifier)
                .padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            ExpandIcon(row.expandable, row.expanded)
            Column(Modifier.weight(1f)) {
                Text(row.name.ifBlank { stringResource(R.string.home_unnamed) }, style = MaterialTheme.typography.titleSmall)
                val details = listOfNotNull(
                    row.subtitle,
                    if (root.hidden) stringResource(R.string.tests_hidden) else null,
                    if (row.test.limitedToGroups) stringResource(R.string.tests_limited) else null,
                )
                if (details.isNotEmpty()) {
                    Text(details.joinToString(" · "), style = MaterialTheme.typography.labelSmall)
                }
            }
            if (total != null) {
                Text(
                    stringResource(R.string.tests_points_total, total),
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                )
            }
        }
    }
}

@Composable
private fun TestNodeRow(row: TestsRow.Node, locale: Locale, lang: String, onToggle: () -> Unit) {
    val node: TestNode = row.node
    val toggleLabel = stringResource(if (row.expanded) R.string.tests_collapse else R.string.tests_expand)
    Column {
        Row(
            Modifier
                .fillMaxWidth()
                .then(if (row.expandable) Modifier.clickable(onClickLabel = toggleLabel, onClick = onToggle) else Modifier)
                .padding(start = (row.depth * 20).dp, top = 8.dp, bottom = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            ExpandIcon(row.expandable, row.expanded)
            Column(Modifier.weight(1f)) {
                Text(
                    row.name.ifBlank { stringResource(R.string.home_unnamed) },
                    style = if (node.kind == TestNodeKind.FOLDER) MaterialTheme.typography.titleSmall else MaterialTheme.typography.bodyMedium,
                )
                val details = listOfNotNull(
                    if (node.hidden) stringResource(R.string.tests_hidden) else null,
                    node.grade?.let { g ->
                        when (g.passes) {
                            true -> stringResource(R.string.grades_passed)
                            false -> stringResource(R.string.grades_failed)
                            null -> null
                        }
                    },
                    node.grade?.description?.pick(lang)?.ifBlank { null },
                    node.comment?.ifBlank { null },
                )
                if (details.isNotEmpty()) {
                    Text(
                        details.joinToString(" · "),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            NodeValue(node, locale)
            Spacer(Modifier.width(4.dp))
        }
        HorizontalDivider(Modifier.padding(start = (row.depth * 20).dp), color = MaterialTheme.colorScheme.outlineVariant)
    }
}

/** The right-hand side of a node: the grade in a tinted chip for a grade node, else "x / max". */
@Composable
private fun NodeValue(node: TestNode, locale: Locale) {
    val colors = MaterialTheme.colorScheme
    if (node.kind == TestNodeKind.GRADE) {
        val grade = node.grade
        val (chip, onChip) = when (grade?.passes) {
            true -> colors.primary to colors.onPrimary
            false -> colors.error to colors.onError
            null -> colors.surfaceVariant to colors.onSurfaceVariant
        }
        Surface(shape = RoundedCornerShape(12.dp), color = chip, contentColor = onChip) {
            Text(
                grade?.symbol?.ifBlank { null } ?: stringResource(R.string.grades_no_symbol),
                Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.Bold,
            )
        }
        return
    }
    val label = pointsLabel(node.points, node.pointsMax, locale) ?: return
    // A node the lecturer hid is not part of any total: its numbers are dimmed.
    Text(
        label,
        Modifier.alpha(if (node.hidden) 0.5f else 1f),
        style = MaterialTheme.typography.titleSmall,
        fontWeight = FontWeight.Bold,
    )
}
