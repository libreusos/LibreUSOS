package io.gitlab.voxix.libreusos.ui.components

import androidx.annotation.DrawableRes
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.ui.theme.LibreType
import io.gitlab.voxix.libreusos.ui.theme.Radii
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import io.gitlab.voxix.libreusos.ui.theme.groupContainer
import io.gitlab.voxix.libreusos.ui.theme.uppercaseForEyebrow

/** Collects the rows of a [GroupedListCard], so the card can put hairlines between them. */
class GroupedListScope internal constructor() {
    internal val rows = mutableListOf<@Composable () -> Unit>()

    /** Adds one row; usually a [GroupedListRow]. */
    fun row(content: @Composable () -> Unit) {
        rows += content
    }
}

/**
 * Grouped list: `group-bg` fill, 1 px hairline border, radius 16, rows separated by hairlines.
 *
 * ```
 * GroupedListCard {
 *     row { GroupedListRow("Motyw", icon = R.drawable.ic_contrast) }
 * }
 * ```
 */
@Composable
fun GroupedListCard(modifier: Modifier = Modifier, content: GroupedListScope.() -> Unit) {
    val rows = GroupedListScope().apply(content).rows
    val shape = RoundedCornerShape(Radii.group)
    Surface(
        modifier.fillMaxWidth().border(1.dp, MaterialTheme.colorScheme.outlineVariant, shape),
        shape = shape,
        color = MaterialTheme.colorScheme.groupContainer,
    ) {
        Column {
            rows.forEachIndexed { index, row ->
                if (index > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                row()
            }
        }
    }
}

/**
 * One list row: optional leading 24 dp icon, [title] with an optional [subtitle], and a trailing slot
 * (value, switch, badge) or, with [chevron], the forward chevron. Padding 12 x 16, at least 56 dp tall;
 * the whole row is one target when [onClick] is set. [destructive] paints title and icon `error`
 * (Wyloguj). A disabled row reads as unavailable and ignores taps.
 */
@Composable
fun GroupedListRow(
    title: String,
    modifier: Modifier = Modifier,
    subtitle: String? = null,
    @DrawableRes icon: Int? = null,
    onClick: (() -> Unit)? = null,
    enabled: Boolean = true,
    destructive: Boolean = false,
    chevron: Boolean = onClick != null,
    trailing: (@Composable () -> Unit)? = null,
) {
    val colors = MaterialTheme.colorScheme
    val titleColor = when {
        !enabled -> colors.onSurface.copy(alpha = DISABLED_ALPHA)
        destructive -> colors.error
        else -> colors.onSurface
    }
    val subtitleColor = if (enabled) colors.onSurfaceVariant else colors.onSurface.copy(alpha = DISABLED_ALPHA)
    val iconColor = when {
        !enabled -> colors.onSurface.copy(alpha = DISABLED_ALPHA)
        destructive -> colors.error
        else -> colors.onSurfaceVariant
    }
    val clickable = if (onClick != null && enabled) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier
    Row(
        modifier
            .fillMaxWidth()
            .defaultMinSize(minHeight = Spacing.rowMinHeight)
            .then(clickable)
            .semantics(mergeDescendants = true) { if (!enabled) disabled() }
            .padding(horizontal = Spacing.rowPaddingHorizontal, vertical = Spacing.rowPaddingVertical),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(Spacing.iconGap),
    ) {
        if (icon != null) Icon(painterResource(icon), null, Modifier.size(24.dp), tint = iconColor)
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.bodyLarge, color = titleColor)
            if (subtitle != null) Text(subtitle, style = MaterialTheme.typography.bodyMedium, color = subtitleColor)
        }
        if (trailing != null) trailing()
        if (chevron) {
            Icon(painterResource(R.drawable.ic_chevron_right), null, Modifier.size(24.dp), tint = iconColor)
        }
    }
}

private const val DISABLED_ALPHA = 0.38f

/**
 * Section label above a grouped list: mono eyebrow, 20 dp above and 8 dp below. The text is
 * uppercased here; pass it as written in the strings.
 */
@Composable
fun SectionHeader(text: String, modifier: Modifier = Modifier) {
    Text(
        uppercaseForEyebrow(text),
        modifier
            .fillMaxWidth()
            .padding(top = Spacing.sectionAbove, bottom = Spacing.sectionBelow)
            .semantics { heading() },
        style = LibreType.monoEyebrow,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}
