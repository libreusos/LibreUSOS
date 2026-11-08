package io.gitlab.voxix.libreusos.ui.theme

import android.content.Context
import android.provider.Settings
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.FiniteAnimationSpec
import androidx.compose.animation.core.tween
import androidx.compose.material3.Shapes
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** 4 dp base grid; the steps in use are 2, 4, 6, 8, 12, 16, 20, 24 and 32 (SYSTEM.md section 6). */
object Spacing {
    val s2: Dp = 2.dp
    val s4: Dp = 4.dp
    val s6: Dp = 6.dp
    val s8: Dp = 8.dp
    val s12: Dp = 12.dp
    val s16: Dp = 16.dp
    val s20: Dp = 20.dp
    val s24: Dp = 24.dp
    val s32: Dp = 32.dp

    /** The one content edge: titles, eyebrows, tabs, section labels, cards and banners. */
    val screenEdge: Dp = 16.dp

    /** Row padding is 12 x 16 dp, and a row is at least 56 dp tall. */
    val rowPaddingHorizontal: Dp = 16.dp
    val rowPaddingVertical: Dp = 12.dp
    val rowMinHeight: Dp = 56.dp

    /** Section label: 20 dp above, 8 dp below. */
    val sectionAbove: Dp = 20.dp
    val sectionBelow: Dp = 8.dp

    /** Icon to text gap in rows (8 dp in buttons). */
    val iconGap: Dp = 12.dp

    /** Minimum touch target. */
    val touchTarget: Dp = 48.dp

    /** Header trailing tonal icon buttons. */
    val headerButton: Dp = 40.dp
}

/** Corner radii by use (SYSTEM.md section 6). Nested radii are concentric: inner = outer - padding. */
object Radii {
    val badge: Dp = 4.dp
    val skeleton: Dp = 6.dp
    val chip: Dp = 8.dp
    val block: Dp = 8.dp
    val control: Dp = 10.dp
    val banner: Dp = 12.dp
    val group: Dp = 16.dp
    val nowCardInner: Dp = 16.dp
    val nowCardOuter: Dp = 20.dp
    val sheet: Dp = 28.dp
}

val LibreShapes = Shapes(
    extraSmall = RoundedCornerShape(4.dp),
    small = RoundedCornerShape(8.dp),
    medium = RoundedCornerShape(12.dp),
    large = RoundedCornerShape(16.dp),
    extraLarge = RoundedCornerShape(28.dp),
)

/** Durations (ms) and easing (SYSTEM.md section 7). The curve is a strong ease-out. */
object Motion {
    const val FAST = 150
    const val BASE = 200
    const val PANEL = 220
    const val SLOW = 420

    /** Reduced motion: movement becomes a short opacity crossfade. */
    const val REDUCED = 100

    val easing = CubicBezierEasing(0.23f, 1f, 0.32f, 1f)

    /** Anything leaving: 0.7 x the enter duration with this curve. */
    val exitEasing = CubicBezierEasing(0.4f, 0f, 1f, 1f)

    fun <T> fast(): FiniteAnimationSpec<T> = tween(FAST, easing = easing)
    fun <T> base(): FiniteAnimationSpec<T> = tween(BASE, easing = easing)
    fun <T> panel(): FiniteAnimationSpec<T> = tween(PANEL, easing = easing)
    fun <T> exit(enterMillis: Int = BASE): FiniteAnimationSpec<T> = tween((enterMillis * 0.7f).toInt(), easing = exitEasing)

    /** True when the user removed animations (animator duration scale 0). */
    fun isReduced(context: Context): Boolean =
        Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f
}

/** Whether the system asks for no animation; read once per composition. */
@Composable
fun rememberReducedMotion(): Boolean {
    val context = LocalContext.current
    return remember(context) { Motion.isReduced(context) }
}
