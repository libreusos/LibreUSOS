package io.gitlab.voxix.libreusos.ui

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.data.clockTime
import io.gitlab.voxix.libreusos.ui.components.KindBadge
import io.gitlab.voxix.libreusos.ui.components.StatusBadge
import io.gitlab.voxix.libreusos.ui.components.StatusBadgeStyle
import io.gitlab.voxix.libreusos.ui.theme.LibreType
import io.gitlab.voxix.libreusos.ui.theme.Radii
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import io.gitlab.voxix.libreusos.ui.theme.kindCodeOf
import io.gitlab.voxix.libreusos.ui.theme.rememberReducedMotion
import io.gitlab.voxix.libreusos.ui.theme.uppercaseForEyebrow
import io.gitlab.voxix.libreusos.widget.TimedActivity
import org.libreusos.core.Activity
import org.libreusos.core.ActivityKind

private fun kindOf(a: Activity) = kindCodeOf(a.classtypeId, a.classtypeName.pl ?: a.classtypeName.en, a.kind == ActivityKind.Exam)

private fun range(item: TimedActivity) = "${clockTime(item.activity.start)}–${clockTime(item.activity.end)}"

/** The class in progress, or else the next one, with the time left and what follows it. */
@Composable
fun NowCard(plan: TodayPlan, lang: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val card = plan.card ?: return
    val item = plan.rows[card.index].item
    val a = item.activity
    val colors = MaterialTheme.colorScheme
    Surface(
        onClick,
        modifier.fillMaxWidth(),
        shape = RoundedCornerShape(Radii.nowCardOuter),
        color = colors.surfaceContainer,
    ) {
        Column(Modifier.padding(Spacing.s4)) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .background(colors.surface, RoundedCornerShape(Radii.nowCardInner))
                    .padding(Spacing.s16),
                verticalArrangement = Arrangement.spacedBy(Spacing.s8),
            ) {
                Row(horizontalArrangement = Arrangement.spacedBy(Spacing.s6), verticalAlignment = Alignment.CenterVertically) {
                    if (card.current) StatusBadge(stringResource(R.string.today_badge_now), StatusBadgeStyle.Now)
                    KindBadge(kindOf(a))
                    Spacer(Modifier.weight(1f))
                    Text(range(item), style = LibreType.monoMedium, color = colors.onSurfaceVariant)
                }
                Text(activityTitle(a, lang), style = MaterialTheme.typography.titleLarge, color = colors.onSurface)
                val place = placeLine(a)
                if (place.isNotEmpty()) Text(place, style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceVariant)
                if (card.current) {
                    LinearProgressIndicator(progress = { card.progress }, Modifier.fillMaxWidth().padding(top = Spacing.s4))
                }
                val words = spanText(card.minutes)
                Text(
                    if (card.current) stringResource(R.string.today_remaining, words) else stringResource(R.string.today_in, words),
                    style = MaterialTheme.typography.labelLarge,
                    color = colors.primary,
                )
            }
            card.nextIndex?.let { next ->
                val n = plan.rows[next].item
                val place = placeLine(n.activity, includeGroup = false)
                val line = listOf(activityTitle(n.activity, lang), place, clockTime(n.activity.start))
                    .filter { it.isNotEmpty() }.joinToString(" · ")
                Row(
                    Modifier.fillMaxWidth().padding(horizontal = Spacing.s12, vertical = Spacing.s8),
                    horizontalArrangement = Arrangement.spacedBy(Spacing.s6),
                ) {
                    Text(stringResource(R.string.today_next_label), style = MaterialTheme.typography.labelLarge, color = colors.onSurfaceVariant)
                    Text(
                        line,
                        Modifier.weight(1f),
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.onSurface,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }
    }
}

/** "PLAN DNIA" with the number of classes at the other end. */
@Composable
fun PlanHeader(count: Int, modifier: Modifier = Modifier) {
    Row(
        modifier.fillMaxWidth().padding(top = Spacing.sectionAbove, bottom = Spacing.sectionBelow),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            uppercaseForEyebrow(stringResource(R.string.today_plan_title)),
            Modifier.weight(1f),
            style = LibreType.monoEyebrow,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(classesCount(count), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

private val TimeColumn = 52.dp
private val RailColumn = 24.dp
private val NodeSize = 10.dp
private val NodeTop = 6.dp
private val ClashBarWidth = 3.dp

/**
 * One class of the day plan: time, rail node, kind badge, title and place; a gap line above it when there is one.
 * Classes of a clash run get a 3 dp error bar along the content, joined from one to the next.
 */
@Composable
fun TimelineEntry(
    row: TimelineRow,
    lang: String,
    isFirst: Boolean,
    isLast: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    isCardClass: Boolean = false,
) {
    val colors = MaterialTheme.colorScheme
    val a = row.item.activity
    val past = row.state == RowState.Past
    // The time column grows with the font scale (SYSTEM.md section 15), so a time never wraps.
    val timeWidth = TimeColumn * LocalDensity.current.fontScale.coerceAtLeast(1f)
    Column(modifier.fillMaxWidth()) {
        row.gapBeforeMin?.let { gap ->
            Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min)) {
                Spacer(Modifier.width(timeWidth))
                Rail(node = false, first = false, last = false)
                Text(
                    stringResource(R.string.today_gap, spanText(gap)),
                    Modifier.padding(vertical = Spacing.s8),
                    style = MaterialTheme.typography.bodySmall,
                    color = colors.onSurfaceVariant,
                )
            }
        }
        Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min).clickable(onClick = onClick)) {
            Column(Modifier.width(timeWidth).padding(top = Spacing.s2)) {
                Text(
                    clockTime(a.start),
                    style = LibreType.monoMedium,
                    color = if (past) colors.onSurfaceVariant else colors.onSurface,
                )
                Text(clockTime(a.end), style = LibreType.monoMedium, color = colors.onSurfaceVariant)
            }
            Rail(node = true, first = isFirst && row.gapBeforeMin == null, last = isLast, state = row.state)
            val barContinues = row.clash.barBelow
            Column(Modifier.weight(1f)) {
                Row(Modifier.height(IntrinsicSize.Min)) {
                    if (row.clash != ClashPart.None) {
                        Box(Modifier.width(ClashBarWidth).fillMaxHeight().background(if (past) colors.outline else colors.error))
                        Spacer(Modifier.width(Spacing.s8 + Spacing.s2))
                    }
                    Column(
                        Modifier.weight(1f).padding(bottom = if (barContinues) Spacing.s16 else 0.dp),
                        verticalArrangement = Arrangement.spacedBy(Spacing.s4),
                    ) {
                        Row(horizontalArrangement = Arrangement.spacedBy(Spacing.s6), verticalAlignment = Alignment.CenterVertically) {
                            KindBadge(kindOf(a), past = past)
                            if (row.clash != ClashPart.None) {
                                StatusBadge(stringResource(R.string.today_badge_clash), StatusBadgeStyle.Alert, warningIcon = true)
                            }
                        }
                        Text(
                            activityTitle(a, lang),
                            style = MaterialTheme.typography.bodyLarge,
                            color = if (past) colors.onSurfaceVariant else colors.onSurface,
                        )
                        val frequency = a.frequency?.let { frequencyLabel(it) }
                        val meta = listOfNotNull(placeLine(a).takeIf { it.isNotEmpty() }, frequency).joinToString(" · ")
                        if (meta.isNotEmpty()) Text(meta, style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceVariant)
                        if (isCardClass) {
                            Text(
                                stringResource(if (row.state == RowState.Now) R.string.today_row_current else R.string.today_row_upcoming),
                                style = MaterialTheme.typography.labelMedium,
                                color = colors.primary,
                            )
                        }
                    }
                }
                if (!barContinues) Spacer(Modifier.height(Spacing.s16))
            }
        }
    }
}

/** The vertical line of the timeline, with a node on class rows. */
@Composable
private fun Rail(node: Boolean, first: Boolean, last: Boolean, state: RowState = RowState.Upcoming) {
    val colors = MaterialTheme.colorScheme
    val line = colors.outlineVariant
    Box(
        Modifier
            .width(RailColumn)
            .fillMaxHeight()
            .drawBehind {
                val x = size.width / 2
                val nodeY = NodeTop.toPx() + NodeSize.toPx() / 2
                val top = if (first) nodeY else 0f
                val bottom = if (last) nodeY else size.height
                drawLine(line, Offset(x, top), Offset(x, bottom), strokeWidth = 2.dp.toPx())
            },
        contentAlignment = Alignment.TopCenter,
    ) {
        if (node) {
            val nodeModifier = Modifier.padding(top = NodeTop).size(NodeSize)
            when (state) {
                RowState.Now -> Box(nodeModifier.background(colors.primary, CircleShape))
                RowState.Upcoming -> Box(nodeModifier.background(colors.surface, CircleShape).border(BorderStroke(2.dp, colors.primary), CircleShape))
                RowState.Past -> Box(nodeModifier.background(colors.outlineVariant, CircleShape))
            }
        }
    }
}

/**
 * First-run placeholder: a week strip, a card and three rows, pulsing unless animations are off.
 * [rowsOnly] keeps the three rows, for Plan, which has its own week strip.
 */
@Composable
fun TodaySkeleton(modifier: Modifier = Modifier, rowsOnly: Boolean = false) {
    val reduced = rememberReducedMotion()
    val alpha = if (reduced) 1f else {
        val transition = rememberInfiniteTransition(label = "skeleton")
        val value by transition.animateFloat(
            initialValue = 0.5f,
            targetValue = 1f,
            animationSpec = infiniteRepeatable(tween(900), RepeatMode.Reverse),
            label = "skeletonAlpha",
        )
        value
    }
    val colors = MaterialTheme.colorScheme
    val shape = RoundedCornerShape(Radii.skeleton)
    @Composable
    fun Bar(width: Float, height: Int = 14) =
        Box(Modifier.fillMaxWidth(width).height(height.dp).background(colors.surfaceContainerHigh, shape))
    Column(modifier.fillMaxWidth().alpha(alpha), verticalArrangement = Arrangement.spacedBy(Spacing.s12)) {
        if (!rowsOnly) {
            Row(horizontalArrangement = Arrangement.spacedBy(Spacing.s8)) {
                repeat(5) { i ->
                    Box(
                        Modifier.weight(1f).height(Spacing.s24 + Spacing.s8)
                            .background(if (i == 1) colors.surfaceContainerHighest else colors.surfaceContainerHigh, shape),
                    )
                }
            }
            Box(Modifier.fillMaxWidth().height(148.dp).background(colors.surfaceContainer, RoundedCornerShape(Radii.nowCardOuter)))
            Spacer(Modifier.height(Spacing.s4))
        }
        repeat(3) {
            Row(horizontalArrangement = Arrangement.spacedBy(Spacing.s12)) {
                Box(Modifier.width(TimeColumn).height(32.dp).background(colors.surfaceContainerHigh, shape))
                Column(verticalArrangement = Arrangement.spacedBy(Spacing.s8), modifier = Modifier.weight(1f)) {
                    Bar(0.35f, 12)
                    Bar(0.8f)
                }
            }
        }
    }
}
