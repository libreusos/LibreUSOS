package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.ui.components.AvatarTile
import io.gitlab.voxix.libreusos.ui.components.GroupedListRow
import io.gitlab.voxix.libreusos.ui.theme.Spacing

/** The two ways out of an account, each behind a confirm dialog. */
enum class AccountAction { Logout, SwitchAccount }

/**
 * The account sheet opened from the Więcej account card: who is signed in and the two account
 * actions. [onAction] only reports the choice; the caller shows [ConfirmAccountActionDialog].
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AccountSheet(account: AccountUiState, onAction: (AccountAction) -> Unit, onDismiss: () -> Unit) {
    ModalBottomSheet(onDismiss) {
        Column(Modifier.navigationBarsPadding().padding(bottom = Spacing.s16)) {
            Row(
                Modifier.padding(horizontal = Spacing.s16, vertical = Spacing.s8),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(Spacing.s16),
            ) {
                AvatarTile(account.initials)
                Column {
                    Text(
                        account.name ?: stringResource(R.string.account_unknown_name),
                        style = MaterialTheme.typography.titleMedium,
                    )
                    if (account.university.isNotBlank()) {
                        Text(account.university, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
            GroupedListRow(
                stringResource(R.string.account_switch),
                icon = R.drawable.ic_swap_vert,
                onClick = { onAction(AccountAction.SwitchAccount) },
            )
            GroupedListRow(
                stringResource(R.string.account_logout),
                icon = R.drawable.ic_logout,
                destructive = true,
                chevron = false,
                onClick = { onAction(AccountAction.Logout) },
            )
        }
    }
}

/** Confirmation before the account's data on this device is deleted. */
@Composable
fun ConfirmAccountActionDialog(action: AccountAction, onConfirm: () -> Unit, onDismiss: () -> Unit) {
    val title = when (action) {
        AccountAction.Logout -> R.string.confirm_logout_title
        AccountAction.SwitchAccount -> R.string.confirm_switch_title
    }
    val body = when (action) {
        AccountAction.Logout -> R.string.confirm_logout_body
        AccountAction.SwitchAccount -> R.string.confirm_switch_body
    }
    val confirm = when (action) {
        AccountAction.Logout -> R.string.account_logout
        AccountAction.SwitchAccount -> R.string.confirm_switch_action
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(title)) },
        text = { Text(stringResource(body)) },
        confirmButton = {
            TextButton(onConfirm) {
                Text(stringResource(confirm), color = MaterialTheme.colorScheme.error)
            }
        },
        dismissButton = { TextButton(onDismiss) { Text(stringResource(R.string.action_cancel)) } },
    )
}
