package io.gitlab.voxix.libreusos.ui.theme

import androidx.compose.foundation.BorderStroke
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import java.util.Locale

/** Which colour a class kind takes (SYSTEM.md section 3, class-kind mapping). */
enum class KindGroup { Lecture, Lab, Exam, Neutral }

/** The printed code (never colour alone) and its colour group. */
data class KindCode(val code: String, val group: KindGroup)

/**
 * Maps a USOS `classtype_id` to the short Polish code the design prints. [isExam] wins over the
 * class type (calendar exams and colloquia are EGZ). Unknown types print the first three letters of
 * the USOS name, uppercased; with no name either, the id itself.
 */
fun kindCodeOf(classtypeId: String?, classtypeName: String?, isExam: Boolean = false): KindCode {
    if (isExam) return KindCode("EGZ", KindGroup.Exam)
    val id = classtypeId?.trim()?.uppercase(Locale.ROOT).orEmpty()
    return when (id) {
        "WYK" -> KindCode("WYK", KindGroup.Lecture)
        "LAB" -> KindCode("LAB", KindGroup.Lab)
        "CW" -> KindCode("ĆW", KindGroup.Neutral)
        "PRO" -> KindCode("PRO", KindGroup.Neutral)
        "SEM" -> KindCode("SEM", KindGroup.Neutral)
        "LEK" -> KindCode("LEK", KindGroup.Neutral)
        "WF" -> KindCode("WF", KindGroup.Neutral)
        else -> {
            val name = classtypeName?.trim().orEmpty()
            val code = when {
                name.isNotEmpty() -> name.take(3).uppercase(Locale.getDefault())
                id.isNotEmpty() -> id.take(3)
                else -> "—"
            }
            KindCode(code, KindGroup.Neutral)
        }
    }
}

/**
 * True when LAB (tertiaryContainer) is within [KIND_DELTA_E_MIN] of WYK (primaryContainer) or of the
 * neutral block (surfaceContainerHighest); the LAB tag then gets an outline so shape, not tone,
 * tells it apart. Computed from the live scheme, so it covers dynamic colour too.
 */
fun labNeedsOutline(scheme: ColorScheme): Boolean {
    val lab = scheme.tertiaryContainer.toArgb()
    return deltaE76(lab, scheme.primaryContainer.toArgb()) < KIND_DELTA_E_MIN ||
        deltaE76(lab, scheme.surfaceContainerHighest.toArgb()) < KIND_DELTA_E_MIN
}

/** Fill, text and optional border of a kind tag or block. */
@Immutable
data class KindStyle(
    val container: Color,
    val content: Color,
    val border: BorderStroke?,
)

private val HAIRLINE: Dp = 1.dp
private val EMPHASIS: Dp = 1.5.dp

/**
 * The colours for [group]. [past] replaces the fill by `surface` and outlines the tag, with no opacity
 * (SYSTEM.md section 3).
 */
@Composable
fun kindStyle(group: KindGroup, past: Boolean = false): KindStyle {
    val scheme = MaterialTheme.colorScheme
    val labOutline = remember(scheme) { labNeedsOutline(scheme) }
    if (past) {
        return KindStyle(scheme.surface, scheme.onSurfaceVariant, BorderStroke(HAIRLINE, scheme.outlineVariant))
    }
    return when (group) {
        KindGroup.Lecture -> KindStyle(scheme.primaryContainer, scheme.onPrimaryContainer, null)
        KindGroup.Lab -> KindStyle(
            scheme.tertiaryContainer,
            scheme.onTertiaryContainer,
            if (labOutline) BorderStroke(EMPHASIS, scheme.onTertiaryContainer) else null,
        )
        KindGroup.Exam -> KindStyle(scheme.primary, scheme.onPrimary, null)
        KindGroup.Neutral -> KindStyle(
            scheme.surfaceContainerHighest,
            scheme.onSurface,
            BorderStroke(HAIRLINE, scheme.outlineVariant),
        )
    }
}
