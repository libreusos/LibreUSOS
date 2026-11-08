package io.gitlab.voxix.libreusos.ui.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.ui.theme.KindCode
import io.gitlab.voxix.libreusos.ui.theme.LibreType
import io.gitlab.voxix.libreusos.ui.theme.Radii
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import io.gitlab.voxix.libreusos.ui.theme.kindStyle
import io.gitlab.voxix.libreusos.ui.theme.uppercaseForEyebrow

private val BadgeShape = RoundedCornerShape(Radii.badge)

/**
 * Class-kind badge: mono 10.5/16 uppercase, min width 30, radius 4, colours per the kind mapping.
 * [past] switches to the outlined past style. Kind codes are always badges, never plain text.
 */
@Composable
fun KindBadge(kind: KindCode, modifier: Modifier = Modifier, past: Boolean = false) {
    val style = kindStyle(kind.group, past)
    Badge(kind.code, style.container, style.content, style.border, modifier)
}

/** Which look a [StatusBadge] has. */
enum class StatusBadgeStyle {
    /** TERAZ: outlined `primary`. */
    Now,

    /** ODWOŁANE, KOLIZJA: `errorContainer`. */
    Alert,
}

/**
 * TERAZ / ODWOŁANE / KOLIZJA. Always sits next to the kind badge and never inline before a title: in
 * cards the badge row is above the title, in list rows it starts the meta line. KOLIZJA passes
 * [warningIcon] = true.
 */
@Composable
fun StatusBadge(
    label: String,
    style: StatusBadgeStyle,
    modifier: Modifier = Modifier,
    warningIcon: Boolean = false,
) {
    val colors = MaterialTheme.colorScheme
    when (style) {
        StatusBadgeStyle.Now -> Badge(label, Color.Transparent, colors.primary, BorderStroke(1.dp, colors.primary), modifier)
        StatusBadgeStyle.Alert -> Badge(
            label,
            colors.errorContainer,
            colors.onErrorContainer,
            null,
            modifier,
            leadingIcon = if (warningIcon) R.drawable.ic_warning else null,
        )
    }
}

@Composable
private fun Badge(
    text: String,
    container: Color,
    content: Color,
    border: BorderStroke?,
    modifier: Modifier,
    leadingIcon: Int? = null,
) {
    var m = modifier.defaultMinSize(minWidth = 30.dp).background(container, BadgeShape)
    if (border != null) m = m.border(border, BadgeShape)
    Row(
        m.padding(horizontal = Spacing.s4 + Spacing.s2),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(Spacing.s2),
    ) {
        if (leadingIcon != null) Icon(painterResource(leadingIcon), null, Modifier.size(12.dp), tint = content)
        Text(
            uppercaseForEyebrow(text),
            style = LibreType.monoBadge,
            color = content,
            textAlign = TextAlign.Center,
            maxLines = 1,
        )
    }
}
