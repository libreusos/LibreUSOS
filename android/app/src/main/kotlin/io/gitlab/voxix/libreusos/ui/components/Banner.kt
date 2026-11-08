package io.gitlab.voxix.libreusos.ui.components

import androidx.annotation.DrawableRes
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.unit.dp
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.ui.theme.Radii
import io.gitlab.voxix.libreusos.ui.theme.Spacing

enum class BannerStyle {
    /** `surfaceContainerHigh`: offline. */
    Neutral,

    /** `errorContainer`: sync failure, expired session, conflicts. */
    Error,
}

/**
 * Inline banner under the header: full content width, radius 12, an icon, the message and at most one
 * text action ("Spróbuj ponownie" or "Zaloguj ponownie", never "Ponów"), plus an optional quieter
 * secondary one before it ("Szczegóły"). Offline is said once per
 * screen: by this banner when there is one, otherwise by the status line.
 */
@Composable
fun Banner(
    text: String,
    modifier: Modifier = Modifier,
    style: BannerStyle = BannerStyle.Neutral,
    @DrawableRes icon: Int = if (style == BannerStyle.Error) R.drawable.ic_error else R.drawable.ic_cloud_off,
    actionLabel: String? = null,
    onAction: (() -> Unit)? = null,
    secondaryLabel: String? = null,
    onSecondary: (() -> Unit)? = null,
) = Banner(AnnotatedString(text), modifier, style, icon, actionLabel, onAction, secondaryLabel, onSecondary)

/** The same banner with a styled message, for a bold lead such as "Kolizja 12:45–13:30.". */
@Composable
fun Banner(
    text: AnnotatedString,
    modifier: Modifier = Modifier,
    style: BannerStyle = BannerStyle.Neutral,
    @DrawableRes icon: Int = if (style == BannerStyle.Error) R.drawable.ic_error else R.drawable.ic_cloud_off,
    actionLabel: String? = null,
    onAction: (() -> Unit)? = null,
    secondaryLabel: String? = null,
    onSecondary: (() -> Unit)? = null,
) {
    val colors = MaterialTheme.colorScheme
    val container = if (style == BannerStyle.Error) colors.errorContainer else colors.surfaceContainerHigh
    val content = if (style == BannerStyle.Error) colors.onErrorContainer else colors.onSurface
    Surface(modifier.fillMaxWidth(), shape = RoundedCornerShape(Radii.banner), color = container, contentColor = content) {
        Column(Modifier.padding(horizontal = Spacing.s16, vertical = Spacing.s12)) {
            Row(horizontalArrangement = Arrangement.spacedBy(Spacing.iconGap)) {
                Icon(painterResource(icon), null, Modifier.padding(top = Spacing.s2).size(20.dp))
                Text(text, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
            }
            val hasPrimary = actionLabel != null && onAction != null
            val hasSecondary = secondaryLabel != null && onSecondary != null
            if (hasPrimary || hasSecondary) {
                Row(Modifier.align(Alignment.End)) {
                    if (hasSecondary) TextButton(onSecondary!!) { Text(secondaryLabel!!, style = MaterialTheme.typography.labelLarge) }
                    if (hasPrimary) TextButton(onAction!!) { Text(actionLabel!!, style = MaterialTheme.typography.labelLarge) }
                }
            }
        }
    }
}
