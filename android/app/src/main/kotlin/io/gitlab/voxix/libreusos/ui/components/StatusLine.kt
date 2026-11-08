package io.gitlab.voxix.libreusos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.clearAndSetSemantics
import io.gitlab.voxix.libreusos.ui.theme.Spacing

/** How fresh the data on a screen is; the dot colour is the second channel, the text is the first. */
enum class SyncState { Fresh, Offline, Failed }

/** The 6 dp dot colour of [state]: primary = fresh, outline = offline, error = failed. */
@Composable
fun SyncState.dotColor(): Color = when (this) {
    SyncState.Fresh -> MaterialTheme.colorScheme.primary
    SyncState.Offline -> MaterialTheme.colorScheme.outline
    SyncState.Failed -> MaterialTheme.colorScheme.error
}

/** "Zsynchronizowano 10:52" with its sync dot, under a [ScreenHeader] title. The text is supplied by the screen. */
@Composable
fun StatusLine(state: SyncState, text: String, modifier: Modifier = Modifier) {
    Row(modifier, verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Spacing.s8)) {
        Box(Modifier.size(Spacing.s6).clip(CircleShape).background(state.dotColor()).clearAndSetSemantics { })
        Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
