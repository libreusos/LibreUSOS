package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.platform.LocalDensity
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.data.clockTime
import io.gitlab.voxix.libreusos.ui.components.GroupedListCard
import io.gitlab.voxix.libreusos.ui.components.KindBadge
import io.gitlab.voxix.libreusos.ui.components.StatusBadge
import io.gitlab.voxix.libreusos.ui.components.StatusBadgeStyle
import io.gitlab.voxix.libreusos.ui.theme.LibreType
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import io.gitlab.voxix.libreusos.ui.theme.kindCodeOf
import org.libreusos.core.Activity
import org.libreusos.core.ActivityKind

/** The classes of one clash side by side as a list; a row opens that class. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ConflictSheet(group: ClashGroup, lang: String, onOpenClass: (Activity) -> Unit, onDismiss: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    ModalBottomSheet(onDismiss, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        Column(
            Modifier.fillMaxWidth().padding(horizontal = Spacing.s16).padding(bottom = Spacing.s24),
            verticalArrangement = Arrangement.spacedBy(Spacing.s12),
        ) {
            Row(horizontalArrangement = Arrangement.spacedBy(Spacing.s8), verticalAlignment = Alignment.CenterVertically) {
                StatusBadge(stringResource(R.string.today_badge_clash), StatusBadgeStyle.Alert, warningIcon = true)
                Text(
                    "${hm(group.from)}–${hm(group.to)} · ${spanText(group.minutes)}",
                    style = LibreType.monoMedium,
                    color = colors.onSurfaceVariant,
                )
            }
            val count = group.items.size
            Text(pluralStringResource(R.plurals.plan_clash_sheet_title, count, count), style = MaterialTheme.typography.headlineSmall)
            GroupedListCard {
                group.items.forEach { item -> row { ConflictRow(item.activity, lang, onClick = { onDismiss(); onOpenClass(item.activity) }) } }
            }
            Text(stringResource(R.string.plan_clash_sheet_note), style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceVariant)
        }
    }
}

@Composable
private fun ConflictRow(a: Activity, lang: String, onClick: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    val timeWidth = 52.dp * LocalDensity.current.fontScale.coerceAtLeast(1f)
    Row(
        Modifier
            .fillMaxWidth()
            .clickable(role = Role.Button, onClick = onClick)
            .semantics(mergeDescendants = true) { }
            .padding(horizontal = Spacing.s16, vertical = Spacing.s12),
        horizontalArrangement = Arrangement.spacedBy(Spacing.s12),
    ) {
        Column(Modifier.width(timeWidth)) {
            Text(clockTime(a.start), style = LibreType.monoMedium, color = colors.onSurface)
            Text(clockTime(a.end), style = LibreType.monoMedium, color = colors.onSurfaceVariant)
        }
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(Spacing.s4)) {
            Text(activityTitle(a, lang), style = MaterialTheme.typography.bodyLarge)
            Row(horizontalArrangement = Arrangement.spacedBy(Spacing.s6), verticalAlignment = Alignment.CenterVertically) {
                KindBadge(kindCodeOf(a.classtypeId, a.classtypeName.pl ?: a.classtypeName.en, a.kind == ActivityKind.Exam))
                val place = placeLine(a)
                if (place.isNotEmpty()) Text(place, style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceVariant)
            }
        }
        Icon(painterResource(R.drawable.ic_chevron_right), null, Modifier.size(24.dp), tint = colors.onSurfaceVariant)
    }
}
