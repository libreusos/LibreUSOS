package io.gitlab.voxix.libreusos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringArrayResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.ui.theme.LibreType
import io.gitlab.voxix.libreusos.ui.theme.Spacing

/** Monday to Friday. */
const val MOTIF_COLUMNS = 5
const val MOTIF_SLOTS = 6

/** A class slot as (column, slot), both zero-based. */
typealias MotifSlot = Pair<Int, Int>

/** A plausible week for the empty state: it only has to read as "a timetable", it is not the user's data. */
val DefaultMotifPattern: Set<MotifSlot> = setOf(
    0 to 0, 0 to 1, 0 to 3,
    1 to 1, 1 to 2, 1 to 4,
    2 to 0, 2 to 2, 2 to 3,
    3 to 1, 3 to 3, 3 to 4,
    4 to 0, 4 to 2,
)

/**
 * The week as slots (SYSTEM.md section 10): Mon-Fri columns by six slots of [square] size, filled
 * `surfaceContainerHighest` = a class, outlined `outlineVariant` = free, and [todayColumn] outlined in
 * 1.5 dp `primary` with its day label in `primary`. With [legend] a three-line key sits beside it so the
 * motif explains itself. Decorative: it is hidden from accessibility, the screen's text says the same.
 */
@Composable
fun WeekMotif(
    modifier: Modifier = Modifier,
    todayColumn: Int? = null,
    pattern: Set<MotifSlot> = DefaultMotifPattern,
    legend: Boolean = true,
    square: Dp = 12.dp,
) {
    val days = stringArrayResource(R.array.week_motif_days)
    Row(
        modifier.clearAndSetSemantics { },
        horizontalArrangement = Arrangement.spacedBy(Spacing.s20),
        verticalAlignment = Alignment.Top,
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(Spacing.s4)) {
            Row(horizontalArrangement = Arrangement.spacedBy(Spacing.s4)) {
                repeat(MOTIF_COLUMNS) { col ->
                    Column(verticalArrangement = Arrangement.spacedBy(Spacing.s4)) {
                        repeat(MOTIF_SLOTS) { slot ->
                            Slot(square, filled = (col to slot) in pattern, today = col == todayColumn)
                        }
                        Text(
                            days.getOrElse(col) { "" },
                            Modifier.width(square),
                            style = LibreType.monoBadge,
                            color = if (col == todayColumn) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 1,
                            softWrap = false,
                        )
                    }
                }
            }
        }
        if (legend) {
            Column(verticalArrangement = Arrangement.spacedBy(Spacing.s8)) {
                LegendLine(stringResource(R.string.week_motif_legend_class)) { Slot(Spacing.s12, filled = true, today = false) }
                LegendLine(stringResource(R.string.week_motif_legend_free)) { Slot(Spacing.s12, filled = false, today = false) }
                LegendLine(stringResource(R.string.week_motif_legend_today)) { Slot(Spacing.s12, filled = false, today = true) }
            }
        }
    }
}

@Composable
private fun LegendLine(text: String, swatch: @Composable () -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Spacing.s8)) {
        swatch()
        Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun Slot(size: Dp, filled: Boolean, today: Boolean) {
    val colors = MaterialTheme.colorScheme
    val shape = RoundedCornerShape(3.dp)
    var m = Modifier.size(size)
    m = when {
        today -> m.border(1.5.dp, colors.primary, shape)
        filled -> m.background(colors.surfaceContainerHighest, shape)
        else -> m.border(1.dp, colors.outlineVariant, shape)
    }
    Box(m)
}
