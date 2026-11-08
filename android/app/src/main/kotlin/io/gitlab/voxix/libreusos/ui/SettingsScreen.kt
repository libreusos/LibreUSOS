package io.gitlab.voxix.libreusos.ui

import android.content.pm.PackageManager
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.data.SYNC_INTERVAL_OPTIONS_HOURS
import io.gitlab.voxix.libreusos.data.ThemeMode
import io.gitlab.voxix.libreusos.ui.components.DrillDownScreen
import io.gitlab.voxix.libreusos.ui.components.GroupedListCard
import io.gitlab.voxix.libreusos.ui.components.GroupedListRow
import io.gitlab.voxix.libreusos.ui.components.SectionHeader
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import io.gitlab.voxix.libreusos.ui.theme.dynamicColorSupported

/**
 * Settings: only what the app can honour today. Appearance, the background sync interval, the account
 * and "about". Notifications, language and widget options arrive with the features behind them.
 */
@Composable
fun SettingsScreen(
    viewModel: SettingsViewModel,
    account: AccountUiState,
    onBack: () -> Unit,
    onOpenLicences: () -> Unit,
    onAccountAction: (AccountAction) -> Unit,
) {
    val appearance by viewModel.appearance.collectAsStateWithLifecycle()
    val syncHours by viewModel.syncHours.collectAsStateWithLifecycle()
    var showIntervals by rememberSaveable { mutableStateOf(false) }

    DrillDownScreen(stringResource(R.string.settings_title), onBack) { padding ->
        Column(
            Modifier
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = Spacing.screenEdge),
        ) {
            SectionHeader(stringResource(R.string.settings_section_appearance))
            GroupedListCard {
                row { ThemeRow(appearance.themeMode, viewModel::setThemeMode) }
                if (dynamicColorSupported()) {
                    row {
                        GroupedListRow(
                            stringResource(R.string.settings_dynamic_color),
                            subtitle = stringResource(R.string.settings_dynamic_color_hint),
                            icon = R.drawable.ic_palette,
                            chevron = false,
                            modifier = Modifier.toggleable(appearance.dynamicColor, role = Role.Switch, onValueChange = viewModel::setDynamicColor),
                            trailing = { Switch(appearance.dynamicColor, onCheckedChange = null) },
                        )
                    }
                }
            }

            SectionHeader(stringResource(R.string.settings_section_sync))
            GroupedListCard {
                row {
                    GroupedListRow(
                        stringResource(R.string.settings_sync_interval),
                        subtitle = pluralStringResource(R.plurals.settings_every_hours, syncHours.toInt(), syncHours),
                        icon = R.drawable.ic_sync,
                        onClick = { showIntervals = true },
                    )
                }
            }

            SectionHeader(stringResource(R.string.settings_section_account))
            GroupedListCard {
                row {
                    GroupedListRow(
                        account.name ?: stringResource(R.string.account_unknown_name),
                        subtitle = account.university.ifBlank { null },
                        icon = R.drawable.ic_account_circle,
                    )
                }
                row {
                    GroupedListRow(
                        stringResource(R.string.account_switch),
                        subtitle = stringResource(R.string.account_switch_hint),
                        icon = R.drawable.ic_swap_vert,
                        onClick = { onAccountAction(AccountAction.SwitchAccount) },
                    )
                }
                row {
                    GroupedListRow(
                        stringResource(R.string.account_logout),
                        subtitle = stringResource(R.string.account_logout_hint),
                        icon = R.drawable.ic_logout,
                        destructive = true,
                        chevron = false,
                        onClick = { onAccountAction(AccountAction.Logout) },
                    )
                }
            }

            SectionHeader(stringResource(R.string.settings_section_about))
            GroupedListCard {
                row {
                    GroupedListRow(
                        stringResource(R.string.settings_version),
                        subtitle = appVersion(),
                        icon = R.drawable.ic_info,
                    )
                }
                row {
                    GroupedListRow(
                        stringResource(R.string.settings_licences),
                        subtitle = stringResource(R.string.settings_licences_hint),
                        icon = R.drawable.ic_description,
                        onClick = onOpenLicences,
                    )
                }
            }
            Spacer(Modifier.size(Spacing.s24))
        }
    }

    if (showIntervals) {
        SyncIntervalDialog(
            selected = syncHours,
            onSelect = {
                viewModel.setSyncHours(it)
                showIntervals = false
            },
            onDismiss = { showIntervals = false },
        )
    }
}

@Composable
private fun ThemeRow(mode: ThemeMode, onChange: (ThemeMode) -> Unit) {
    Column(Modifier.fillMaxWidth().padding(Spacing.s16), verticalArrangement = Arrangement.spacedBy(Spacing.s12)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Spacing.iconGap)) {
            Icon(painterResource(R.drawable.ic_contrast), null, Modifier.size(24.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
            Text(stringResource(R.string.settings_theme), style = MaterialTheme.typography.bodyLarge)
        }
        val options = listOf(
            ThemeMode.System to R.string.settings_theme_system,
            ThemeMode.Light to R.string.settings_theme_light,
            ThemeMode.Dark to R.string.settings_theme_dark,
        )
        SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
            options.forEachIndexed { index, (value, label) ->
                SegmentedButton(
                    selected = mode == value,
                    onClick = { onChange(value) },
                    shape = SegmentedButtonDefaults.itemShape(index, options.size),
                ) { Text(stringResource(label)) }
            }
        }
    }
}

@Composable
private fun SyncIntervalDialog(selected: Long, onSelect: (Long) -> Unit, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.settings_sync_interval_title)) },
        text = {
            Column {
                SYNC_INTERVAL_OPTIONS_HOURS.forEach { hours ->
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .heightIn(min = Spacing.touchTarget)
                            .selectable(selected == hours, role = Role.RadioButton, onClick = { onSelect(hours) }),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        RadioButton(selected == hours, onClick = null)
                        Text(
                            pluralStringResource(R.plurals.settings_every_hours, hours.toInt(), hours),
                            Modifier.padding(start = Spacing.s12),
                            style = MaterialTheme.typography.bodyLarge,
                        )
                    }
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onDismiss) { Text(stringResource(R.string.action_cancel)) } },
    )
}

@Composable
private fun appVersion(): String {
    val context = LocalContext.current
    return remember(context) {
        try {
            context.packageManager.getPackageInfo(context.packageName, 0).versionName.orEmpty()
        } catch (_: PackageManager.NameNotFoundException) {
            ""
        }
    }
}
