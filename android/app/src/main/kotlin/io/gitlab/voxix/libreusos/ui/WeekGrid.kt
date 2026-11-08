package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.BoxWithConstraintsScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.wrapContentHeight
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringArrayResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.intl.LocaleList
import androidx.compose.ui.text.style.Hyphens
import androidx.compose.ui.text.style.LineBreak
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.ui.components.KindBadge
import io.gitlab.voxix.libreusos.ui.theme.KindCode
import io.gitlab.voxix.libreusos.ui.theme.KindGroup
import io.gitlab.voxix.libreusos.ui.theme.LibreType
import io.gitlab.voxix.libreusos.ui.theme.Radii
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import io.gitlab.voxix.libreusos.ui.theme.kindCodeOf
import io.gitlab.voxix.libreusos.ui.theme.kindStyle
import org.libreusos.core.Activity
import org.libreusos.core.ActivityKind
import java.time.LocalDate
import java.time.LocalDateTime

private val HourHeight = 52.dp
private val ClashEdge = 3.dp
private val BlockGap = 2.dp

/** Grid cells are small: 11/14 sp, scaled with the user's font size like every other text. */
private val CellText = TextStyle(fontSize = 11.sp, lineHeight = 14.sp)

private fun kindOf(a: Activity) = kindCodeOf(a.classtypeId, a.classtypeName.pl ?: a.classtypeName.en, a.kind == ActivityKind.Exam)

/**
 * The week as hours down and days across (SYSTEM.md section 10). Blocks show the kind code and the full name
 * with Polish hyphenation, as many lines as the block is tall; a clash is one cell that opens [onOpenClash].
 * From 150 % font scale a block shows its kind code only.
 */
@Composable
fun WeekGridView(
    grid: WeekGrid,
    today: LocalDate,
    now: LocalDateTime,
    lang: String,
    onOpenClass: (Activity) -> Unit,
    onOpenClash: (ClashGroup) -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = MaterialTheme.colorScheme
    val fontScale = LocalDensity.current.fontScale.coerceAtLeast(1f)
    val range = grid.range
    val labelWidth = 24.dp * fontScale
    val abbreviations = stringArrayResource(R.array.plan_day_abbr)
    val nowMinute = if (now.toLocalDate() == today) now.hour * 60 + now.minute else null
    Column(modifier.fillMaxSize()) {
        Row(Modifier.fillMaxWidth().padding(bottom = Spacing.s4)) {
            Spacer(Modifier.width(labelWidth))
            grid.days.forEach { day ->
                Text(
                    abbreviations[day.date.dayOfWeek.value - 1] + " " + day.date.dayOfMonth,
                    Modifier.weight(1f),
                    style = MaterialTheme.typography.labelLarge,
                    color = if (day.date == today) colors.primary else colors.onSurface,
                    textAlign = TextAlign.Center,
                    maxLines = 1,
                )
            }
        }
        Column(Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(top = Spacing.s12, bottom = Spacing.s12)) {
            val hours = range.end - range.start
            Row(Modifier.fillMaxWidth().height(HourHeight * hours)) {
                Box(Modifier.width(labelWidth).fillMaxHeight()) {
                    for (h in range.start..range.end) {
                        Text(
                            "%02d".format(h),
                            Modifier
                                .offset(y = HourHeight * (h - range.start))
                                .height(0.dp)
                                .wrapContentHeight(Alignment.CenterVertically, unbounded = true),
                            style = LibreType.monoMedium,
                            color = colors.onSurfaceVariant,
                            maxLines = 1,
                        )
                    }
                }
                val line = colors.outlineVariant
                Row(
                    Modifier.weight(1f).fillMaxHeight().drawBehind {
                        for (h in 0..hours) {
                            val y = HourHeight.toPx() * h
                            drawLine(line, Offset(0f, y), Offset(size.width, y), strokeWidth = 1.dp.toPx())
                        }
                    },
                ) {
                    grid.days.forEach { day ->
                        DayColumn(
                            day,
                            range,
                            now,
                            lang,
                            nowMinute = nowMinute.takeIf { day.date == today },
                            onOpenClass = onOpenClass,
                            onOpenClash = onOpenClash,
                            modifier = Modifier.weight(1f).fillMaxHeight(),
                        )
                    }
                }
            }
            Legend(Modifier.padding(top = Spacing.s16))
        }
    }
}

private fun top(startMin: Int, range: HourRange): Dp = HourHeight * ((startMin - range.start * 60) / 60f)

private fun height(startMin: Int, endMin: Int): Dp = HourHeight * ((endMin - startMin) / 60f)

@Composable
private fun DayColumn(
    day: GridDay,
    range: HourRange,
    now: LocalDateTime,
    lang: String,
    nowMinute: Int?,
    onOpenClass: (Activity) -> Unit,
    onOpenClash: (ClashGroup) -> Unit,
    modifier: Modifier,
) {
    BoxWithConstraints(modifier.padding(horizontal = BlockGap)) {
        day.cells.forEach { cell ->
            if (cell.isClash) {
                ClashCell(cell, range, now, lang, onOpen = { onOpenClash(ClashGroup(cell.blocks.map { it.item })) })
            } else {
                val block = cell.blocks.single()
                val a = block.item.activity
                GridBlockView(
                    block,
                    now,
                    lang,
                    kindOnly = false,
                    onClick = { onOpenClass(a) },
                    modifier = Modifier.offset(y = top(block.startMin, range)).fillMaxWidth().height(height(block.startMin, block.endMin) - BlockGap),
                )
            }
        }
        if (nowMinute != null && nowMinute in range.start * 60..range.end * 60) {
            val color = MaterialTheme.colorScheme.primary
            Box(Modifier.offset(y = top(nowMinute, range)).fillMaxWidth().height(2.dp).background(color))
            Box(Modifier.offset(x = (-4).dp, y = top(nowMinute, range) - 3.dp).size(8.dp).background(color))
        }
    }
}

/** Overlapping classes side by side behind one 3 dp error edge, with the warning icon; the cell is one target of 48 dp or more. */
@Composable
private fun BoxWithConstraintsScope.ClashCell(
    cell: GridCell,
    range: HourRange,
    now: LocalDateTime,
    lang: String,
    onOpen: () -> Unit,
) {
    val colors = MaterialTheme.colorScheme
    val cellHeight = maxOf(height(cell.startMin, cell.endMin), Spacing.touchTarget)
    val cellWidth = maxWidth
    val spoken = cell.blocks.map { classDescription(it.item.activity, lang, clash = true) }.joinToString(". ")
    Box(
        Modifier
            .offset(y = top(cell.startMin, range))
            .fillMaxWidth()
            .height(cellHeight)
            .clickable(role = Role.Button, onClick = onOpen)
            .semantics(mergeDescendants = true) { contentDescription = spoken },
    ) {
        Box(Modifier.width(ClashEdge).fillMaxHeight().background(colors.error))
        val inner = cellWidth - ClashEdge - BlockGap
        val columns = cell.blocks.first().columns
        cell.blocks.forEach { block ->
            GridBlockView(
                block,
                now,
                lang,
                kindOnly = true,
                onClick = null,
                modifier = Modifier
                    .offset(x = ClashEdge + BlockGap + inner / columns * block.column, y = top(block.startMin, range) - top(cell.startMin, range))
                    .width(inner / columns - BlockGap)
                    .height(height(block.startMin, block.endMin) - BlockGap),
            )
        }
        Box(
            Modifier
                .align(Alignment.BottomStart)
                .padding(start = ClashEdge + Spacing.s4, bottom = Spacing.s2)
                .size(24.dp)
                .background(colors.errorContainer, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Icon(painterResource(R.drawable.ic_warning), null, Modifier.size(14.dp), tint = colors.onErrorContainer)
        }
    }
}

/** One class block; [onClick] null inside a clash cell, whose cell is the target. */
@Composable
private fun GridBlockView(
    block: GridBlock,
    now: LocalDateTime,
    lang: String,
    kindOnly: Boolean,
    onClick: (() -> Unit)?,
    modifier: Modifier,
) {
    val colors = MaterialTheme.colorScheme
    val a = block.item.activity
    val kind = kindOf(a)
    val past = block.item.end <= now
    val current = block.item.start <= now && now < block.item.end
    val style = kindStyle(kind.group, past)
    val border = if (current) BorderStroke(2.dp, colors.onSurface) else style.border
    val shape = RoundedCornerShape(Radii.block)
    val fontScale = LocalDensity.current.fontScale.coerceAtLeast(1f)
    val room = a.roomNumber?.takeIf { it.isNotBlank() }?.let { stringResource(R.string.today_room, it).replace(' ', ' ') }
    val description = classDescription(a, lang, clash = false)
    val content: @Composable () -> Unit = {
        // A narrow clash column keeps the whole code: less padding, and the code never wraps or clips.
        val inset = if (kindOnly) Spacing.s2 else Spacing.s6
        BoxWithConstraints(Modifier.padding(horizontal = inset, vertical = Spacing.s4).clearAndSetSemantics { }) {
            val availableHeight = maxHeight
            Column {
                Text(kind.code, style = LibreType.monoBadge, color = style.content, maxLines = 1, softWrap = false, overflow = TextOverflow.Visible)
                if (!kindOnly && fontScale < 1.5f) {
                    val lineHeight = 14.dp * fontScale
                    var lines = ((availableHeight - 16.dp * fontScale) / lineHeight).toInt()
                    val showRoom = room != null && lines >= 3
                    if (showRoom) lines -= 1
                    if (lines >= 1) {
                        Text(
                            activityTitle(a, lang),
                            style = CellText.copy(
                                hyphens = Hyphens.Auto,
                                lineBreak = LineBreak.Paragraph,
                                localeList = LocaleList(lang),
                            ),
                            color = style.content,
                            maxLines = lines,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                    if (showRoom) Text(room!!, style = CellText, color = style.content, maxLines = 1)
                }
            }
        }
    }
    if (onClick != null) {
        Surface(
            onClick,
            modifier.semantics { contentDescription = description },
            shape = shape,
            color = style.container,
            border = border,
            content = content,
        )
    } else {
        Surface(modifier, shape = shape, color = style.container, border = border, content = content)
    }
}

@Composable
private fun Legend(modifier: Modifier = Modifier) {
    FlowRow(
        modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(Spacing.s16),
        verticalArrangement = Arrangement.spacedBy(Spacing.s8),
    ) {
        LegendItem(KindCode("WYK", KindGroup.Lecture), stringResource(R.string.plan_legend_lecture))
        LegendItem(KindCode("LAB", KindGroup.Lab), stringResource(R.string.plan_legend_lab))
        LegendItem(KindCode("ĆW", KindGroup.Neutral), stringResource(R.string.plan_legend_other))
    }
}

@Composable
private fun LegendItem(kind: KindCode, label: String) {
    Row(horizontalArrangement = Arrangement.spacedBy(Spacing.s6), verticalAlignment = Alignment.CenterVertically) {
        KindBadge(kind)
        Text(label, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
