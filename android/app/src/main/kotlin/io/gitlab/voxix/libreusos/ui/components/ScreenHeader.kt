package io.gitlab.voxix.libreusos.ui.components

import androidx.annotation.DrawableRes
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import io.gitlab.voxix.libreusos.ui.theme.LibreType
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import io.gitlab.voxix.libreusos.ui.theme.uppercaseForEyebrow

/**
 * Header of a mobile tab page (SYSTEM.md section 10): no app bar chrome, a mono eyebrow, the `h-lg`
 * title, an optional [statusLine] and trailing [actions] centred on the title line.
 *
 * Put it inside a column that supplies the 16 dp screen edge and the status bar inset; it adds only
 * its own vertical rhythm.
 */
@Composable
fun ScreenHeader(
    title: String,
    modifier: Modifier = Modifier,
    eyebrow: String? = null,
    statusLine: (@Composable () -> Unit)? = null,
    actions: (@Composable RowScope.() -> Unit)? = null,
) {
    Column(modifier.fillMaxWidth().padding(top = Spacing.s16, bottom = Spacing.s8)) {
        if (eyebrow != null) {
            Text(
                uppercaseForEyebrow(eyebrow),
                style = LibreType.monoEyebrow,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        Row(
            Modifier.fillMaxWidth().padding(top = Spacing.s4),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Spacing.s8),
        ) {
            Text(
                title,
                Modifier.weight(1f).semantics { heading() },
                style = MaterialTheme.typography.headlineLarge,
                color = MaterialTheme.colorScheme.onSurface,
            )
            if (actions != null) actions()
        }
        if (statusLine != null) {
            Column(Modifier.padding(top = Spacing.s4)) { statusLine() }
        }
    }
}

/** The 40 dp tonal icon button of header actions. [contentDescription] is required: icons carry no text. */
@Composable
fun HeaderIconButton(
    @DrawableRes icon: Int,
    contentDescription: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
) {
    FilledTonalIconButton(onClick, modifier.size(Spacing.headerButton), enabled = enabled) {
        Icon(painterResource(icon), contentDescription)
    }
}
