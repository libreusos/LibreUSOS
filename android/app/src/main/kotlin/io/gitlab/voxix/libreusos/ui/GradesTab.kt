package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.BorderStroke
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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.sync.minutesUntil
import io.gitlab.voxix.libreusos.ui.components.HeaderIconButton
import io.gitlab.voxix.libreusos.ui.components.ScreenHeader
import io.gitlab.voxix.libreusos.ui.theme.Radii
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import org.libreusos.core.CourseGrades
import java.time.LocalDate
import java.time.ZoneId

/**
 * Oceny: the header with the sync status, the tabs Przedmioty and Sprawdziany, the term picker both
 * share, and every sync state of SYSTEM.md section 12. The status line, refresh button and banners
 * follow the tab on screen.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GradesTab(
    gradesViewModel: GradesViewModel,
    testsViewModel: TestsViewModel,
    sessionExpired: Boolean,
    onRelogin: () -> Unit,
    onOpenCourse: (termId: String, course: CourseGrades) -> Unit,
    modifier: Modifier = Modifier,
) {
    val grades by gradesViewModel.state.collectAsStateWithLifecycle()
    val tests by testsViewModel.state.collectAsStateWithLifecycle()
    val record by gradesViewModel.syncRecord.collectAsStateWithLifecycle()
    val locale = LocalConfiguration.current.locales[0]
    var tab by rememberSaveable { mutableIntStateOf(0) }
    var chosen by rememberSaveable { mutableStateOf<String?>(null) }
    var details by rememberSaveable { mutableStateOf(false) }

    // First visit, every switch of the tab and every return from the background; the models skip fresh live data.
    fun ensureLoaded() = if (tab == 0) gradesViewModel.ensureLoaded() else testsViewModel.ensureLoaded()
    LaunchedEffect(tab) { ensureLoaded() }
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { ensureLoaded() }

    val home = if (tab == 0) grades.asHomeState() else tests.asHomeState()
    val zone = grades.zone ?: tests.zone ?: ZoneId.systemDefault()
    val today = grades.today ?: tests.today ?: LocalDate.now(zone)
    val nowMs = System.currentTimeMillis()
    val mode = homeMode(home, sessionExpired)
    val banner = homeBanner(home, sessionExpired)
    val status = homeStatus(home, sessionExpired)
    val retryIn = record.nextAttemptAtMs?.let { minutesUntil(it, nowMs) }
    val failure = home.failure
    val refresh = if (tab == 0) gradesViewModel::refresh else testsViewModel::refresh

    val termList = if (grades.terms.terms.isNotEmpty()) grades.terms else tests.terms
    val withData = grades.sections.orEmpty().filter { it.hasGrades }.map { it.termId } + tests.sections.orEmpty().map { it.termId }
    val options = remember(termList, withData, today) { termOptions(termList, withData.toSet(), today) }
    val selectedId = selectedTermId(options, chosen, termList.currentId)

    Column(modifier.fillMaxSize()) {
        Column(Modifier.padding(horizontal = Spacing.screenEdge)) {
            ScreenHeader(
                title = stringResource(R.string.nav_grades),
                statusLine = if (status == HomeStatus.Synced) {
                    null
                } else {
                    statusText(status, mode, home, zone, today, locale, GradesCopy)?.let { text -> { SyncStatusLine(status, text) } }
                },
                actions = {
                    HeaderIconButton(
                        icon = when {
                            failure?.offline == true -> R.drawable.ic_cloud_off
                            failure != null -> R.drawable.ic_sync_problem
                            else -> R.drawable.ic_refresh
                        },
                        contentDescription = stringResource(R.string.today_refresh),
                        onClick = refresh,
                        enabled = !home.loading && !sessionExpired,
                    )
                },
            )
        }
        PrimaryTabRow(selectedTabIndex = tab) {
            Tab(tab == 0, onClick = { tab = 0 }, text = { Text(stringResource(R.string.grades_tab_courses)) })
            Tab(tab == 1, onClick = { tab = 1 }, text = { Text(stringResource(R.string.grades_tab_tests)) })
        }
        Column(Modifier.weight(1f).fillMaxWidth().padding(horizontal = Spacing.screenEdge)) {
            if (selectedId != null) {
                TermChip(options, selectedId, onSelect = { chosen = it }, Modifier.padding(vertical = Spacing.s12))
            } else {
                Spacer(Modifier.height(Spacing.s12))
            }
            SyncBanner(
                banner = banner,
                failure = failure,
                dataStamp = home.dataAtSec?.let { stampText(it, zone, today, locale) }.orEmpty(),
                retryIn = retryIn,
                onRetry = refresh,
                onDetails = { details = true },
                onRelogin = onRelogin,
                copy = GradesCopy,
            )
            if (banner != HomeBanner.None) Spacer(Modifier.height(Spacing.s8))

            PullToRefreshBox(
                isRefreshing = home.refreshing,
                onRefresh = refresh,
                modifier = Modifier.weight(1f).fillMaxWidth(),
            ) {
                when (mode) {
                    HomeMode.Skeleton -> Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
                        TodaySkeleton(rowsOnly = true)
                    }
                    HomeMode.NoCacheOffline, HomeMode.NoCacheError, HomeMode.NoCacheExpired -> NoCacheContent(
                        mode = mode,
                        failure = failure,
                        retryIn = retryIn,
                        onRetry = refresh,
                        onDetails = { details = true },
                        onRelogin = onRelogin,
                        copy = GradesCopy,
                    )
                    HomeMode.Content -> if (tab == 0) {
                        GradesContent(
                            section = grades.sections.orEmpty().firstOrNull { it.termId == selectedId },
                            previousTerm = previousTermId(options, selectedId),
                            onPreviousTerm = { chosen = it },
                            onOpenCourse = { onOpenCourse(selectedId.orEmpty(), it) },
                            locale = locale,
                        )
                    } else {
                        TestsContent(
                            section = tests.sections.orEmpty().firstOrNull { it.termId == selectedId },
                            skipped = tests.skipped,
                            previousTerm = previousTermId(options, selectedId),
                            onPreviousTerm = { chosen = it },
                            locale = locale,
                        )
                    }
                }
            }
        }
    }

    val shownFailure = failure
    // A cleared failure closes the sheet; the next failure must not reopen it unasked.
    LaunchedEffect(shownFailure == null) { if (shownFailure == null) details = false }
    if (details && shownFailure != null) {
        DetailsSheet(detailsText(shownFailure, gradesViewModel.baseUrl, zone), onDismiss = { details = false })
    }
}

@Composable
private fun termWordText(word: TermWord): String? = when (word) {
    TermWord.Ongoing -> stringResource(R.string.term_word_ongoing)
    TermWord.Ended -> stringResource(R.string.term_word_ended)
    TermWord.Summer -> stringResource(R.string.term_word_summer)
    TermWord.Winter -> stringResource(R.string.term_word_winter)
    TermWord.None -> null
}

/** The term picker: an outlined chip with the term and its state word that opens a menu of the terms. */
@Composable
private fun TermChip(options: List<TermOption>, selectedId: String, onSelect: (String) -> Unit, modifier: Modifier = Modifier) {
    var open by remember { mutableStateOf(false) }
    val selected = options.firstOrNull { it.id == selectedId }
    val label = listOfNotNull(selectedId, selected?.word?.let { termWordText(it) }).joinToString(" · ")
    val description = stringResource(R.string.grades_term_picker) + ": " + label
    Box(modifier) {
        Surface(
            onClick = { open = true },
            modifier = Modifier.semantics { contentDescription = description },
            shape = RoundedCornerShape(Radii.control),
            color = MaterialTheme.colorScheme.surface,
            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
        ) {
            Row(
                Modifier.height(40.dp).padding(start = Spacing.s16, end = Spacing.s12),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(Spacing.s8),
            ) {
                Text(selectedId, style = MaterialTheme.typography.bodyLarge)
                termWordText(selected?.word ?: TermWord.None)?.let {
                    Text("· $it", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Icon(painterResource(R.drawable.ic_expand_more), null, Modifier.size(Spacing.s24))
            }
        }
        DropdownMenu(open, onDismissRequest = { open = false }) {
            options.forEach { option ->
                val word = termWordText(option.word)
                DropdownMenuItem(
                    text = {
                        Text(
                            listOfNotNull(option.id, word).joinToString(" · "),
                            color = if (option.id == selectedId) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
                        )
                    },
                    onClick = {
                        open = false
                        onSelect(option.id)
                    },
                )
            }
        }
    }
}
