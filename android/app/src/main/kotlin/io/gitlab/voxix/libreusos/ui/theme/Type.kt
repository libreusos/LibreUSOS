package io.gitlab.voxix.libreusos.ui.theme

import androidx.compose.material3.Typography
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.text.ExperimentalTextApi
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import io.gitlab.voxix.libreusos.R

@OptIn(ExperimentalTextApi::class)
private fun geist(weight: Int) = Font(
    R.font.geist,
    FontWeight(weight),
    variationSettings = FontVariation.Settings(FontVariation.weight(weight)),
)

@OptIn(ExperimentalTextApi::class)
private fun geistMono(weight: Int) = Font(
    R.font.geist_mono,
    FontWeight(weight),
    variationSettings = FontVariation.Settings(FontVariation.weight(weight)),
)

/** Geist, variable on `wght`: 400 body, 500 titles and labels, 600 only for the active nav label. */
val GeistFamily = FontFamily(geist(400), geist(500), geist(600))

/** Geist Mono: class-kind codes, eyebrows, times in grids, URLs. */
val GeistMonoFamily = FontFamily(geistMono(400), geistMono(500))

/** Tabular figures: times, grades, points and percentages (Geist's default figures are proportional). */
fun TextStyle.tabular(): TextStyle = copy(fontFeatureSettings = "tnum")

private fun style(
    size: Int,
    line: Int,
    weight: Int,
    tracking: Double = 0.0,
    family: FontFamily = GeistFamily,
    features: String? = null,
) = TextStyle(
    fontFamily = family,
    fontWeight = FontWeight(weight),
    fontSize = size.sp,
    lineHeight = line.sp,
    letterSpacing = tracking.em,
    fontFeatureSettings = features,
)

/** The M3 type scale of SYSTEM.md section 2 (sizes and lines in sp, tracking in em). */
val LibreTypography = Typography(
    displaySmall = style(36, 44, 500, -0.025, features = "tnum"),
    headlineLarge = style(32, 40, 500, -0.022),
    headlineMedium = style(28, 36, 500, -0.018),
    headlineSmall = style(24, 32, 500, -0.014),
    titleLarge = style(20, 28, 500, -0.01),
    titleMedium = style(16, 22, 500, -0.005),
    titleSmall = style(14, 20, 500),
    bodyLarge = style(16, 24, 400),
    bodyMedium = style(14, 20, 400),
    bodySmall = style(12, 16, 400, 0.005),
    labelLarge = style(14, 20, 500),
    labelMedium = style(12, 16, 500, 0.005),
    labelSmall = style(11, 16, 500, 0.01),
)

/**
 * The mono styles. M3 [Typography] has no slot for them, so they travel in a CompositionLocal and are
 * read through [LibreType]. Uppercase is not a text style in Compose: callers uppercase the text
 * (see [uppercaseForEyebrow]).
 */
@Immutable
data class MonoStyles(
    /** URLs and keys in fields: 16/22, tabular. */
    val large: TextStyle,
    /** Timeline times, hosts: 13/20. */
    val medium: TextStyle,
    /** Section eyebrows: 11/16, uppercase text, +0.04 em. */
    val eyebrow: TextStyle,
    /** Kind codes and status badges: 10.5/16, uppercase text, +0.05 em. */
    val badge: TextStyle,
)

val LibreMonoStyles = MonoStyles(
    large = style(16, 22, 400, family = GeistMonoFamily, features = "tnum"),
    medium = style(13, 20, 400, family = GeistMonoFamily, features = "tnum"),
    eyebrow = style(11, 16, 500, 0.04, family = GeistMonoFamily, features = "case"),
    badge = TextStyle(
        fontFamily = GeistMonoFamily,
        fontWeight = FontWeight.Medium,
        fontSize = 10.5.sp,
        lineHeight = 16.sp,
        letterSpacing = 0.05.em,
        fontFeatureSettings = "case",
    ),
)

val LocalMonoStyles = staticCompositionLocalOf { LibreMonoStyles }

/** Access to the mono styles: `LibreType.monoEyebrow`, as in SYSTEM.md section 2. */
object LibreType {
    val monoLarge: TextStyle @Composable @ReadOnlyComposable get() = LocalMonoStyles.current.large
    val monoMedium: TextStyle @Composable @ReadOnlyComposable get() = LocalMonoStyles.current.medium
    val monoEyebrow: TextStyle @Composable @ReadOnlyComposable get() = LocalMonoStyles.current.eyebrow
    val monoBadge: TextStyle @Composable @ReadOnlyComposable get() = LocalMonoStyles.current.badge
}

/** Eyebrows and badges are uppercase by style, not by the stored text. */
fun uppercaseForEyebrow(text: String): String = text.uppercase(java.util.Locale.getDefault())
