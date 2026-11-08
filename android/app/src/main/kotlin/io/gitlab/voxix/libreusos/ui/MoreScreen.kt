package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.ui.components.AvatarTile
import io.gitlab.voxix.libreusos.ui.components.GroupedListCard
import io.gitlab.voxix.libreusos.ui.components.GroupedListRow
import io.gitlab.voxix.libreusos.ui.components.ScreenHeader
import io.gitlab.voxix.libreusos.ui.components.SectionHeader
import io.gitlab.voxix.libreusos.ui.theme.Spacing

/** A hub entry whose screen does not exist yet; shown disabled with a "soon" note. */
private class PlannedEntry(val title: Int, val icon: Int)

private val studies = listOf(
    PlannedEntry(R.string.more_deadlines, R.drawable.ic_event_upcoming),
    PlannedEntry(R.string.more_registrations, R.drawable.ic_how_to_reg),
    PlannedEntry(R.string.more_surveys, R.drawable.ic_ballot),
    PlannedEntry(R.string.more_search_plan, R.drawable.ic_travel_explore),
)

private val affairs = listOf(
    PlannedEntry(R.string.more_payments, R.drawable.ic_payments),
    PlannedEntry(R.string.more_mlegitymacja, R.drawable.ic_id_card),
)

private val communication = listOf(
    PlannedEntry(R.string.more_news, R.drawable.ic_newspaper),
    PlannedEntry(R.string.more_messages, R.drawable.ic_mail),
)

/**
 * The Więcej hub: account card on top, then the Studia, Sprawy and Komunikacja groups and Ustawienia
 * last. Entries whose data the core cannot give yet are listed disabled; Prace dyplomowe, Wymiana and
 * Decyzje i wnioski appear only when USOS has such records, so they are absent.
 */
@Composable
fun MoreScreen(account: AccountUiState, onOpenAccount: () -> Unit, onOpenSettings: () -> Unit, modifier: Modifier = Modifier) {
    Column(
        modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = Spacing.screenEdge),
    ) {
        ScreenHeader(
            title = stringResource(R.string.nav_more),
            eyebrow = account.university.ifBlank { null },
        )
        Spacer(Modifier.size(Spacing.s16))
        AccountCard(account, onOpenAccount)
        PlannedGroup(R.string.more_group_studies, studies)
        PlannedGroup(R.string.more_group_affairs, affairs)
        PlannedGroup(R.string.more_group_communication, communication)
        Spacer(Modifier.size(Spacing.s16))
        GroupedListCard {
            row {
                GroupedListRow(
                    stringResource(R.string.more_settings),
                    icon = R.drawable.ic_settings,
                    onClick = onOpenSettings,
                )
            }
        }
        Spacer(Modifier.size(Spacing.s16))
    }
}

@Composable
private fun PlannedGroup(title: Int, entries: List<PlannedEntry>) {
    SectionHeader(stringResource(title))
    GroupedListCard {
        entries.forEach { entry ->
            row {
                GroupedListRow(
                    stringResource(entry.title),
                    icon = entry.icon,
                    enabled = false,
                    chevron = false,
                    trailing = {
                        Text(
                            stringResource(R.string.more_soon),
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    },
                )
            }
        }
    }
}

@Composable
private fun AccountCard(account: AccountUiState, onClick: () -> Unit) {
    GroupedListCard {
        row {
            Row(
                Modifier
                    .fillMaxWidth()
                    .clickable(role = Role.Button, onClick = onClick)
                    .semantics(mergeDescendants = true) { }
                    .padding(Spacing.s16),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(Spacing.s16),
            ) {
                AvatarTile(account.initials, size = 48.dp)
                Column(Modifier.weight(1f)) {
                    Text(
                        account.name ?: stringResource(R.string.account_unknown_name),
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    if (account.host.isNotBlank()) {
                        Text(account.host, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
                Icon(
                    painterResource(R.drawable.ic_chevron_right),
                    null,
                    Modifier.size(24.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
