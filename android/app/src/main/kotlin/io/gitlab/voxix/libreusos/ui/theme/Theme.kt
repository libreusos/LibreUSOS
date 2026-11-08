package io.gitlab.voxix.libreusos.ui.theme

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat
import io.gitlab.voxix.libreusos.data.ThemeMode

/** True while the dark scheme is in use, whether chosen by the system or in Settings. */
val LocalIsDark = staticCompositionLocalOf { false }

/** Dynamic colour needs Material You, which exists from Android 12 (API 31). */
fun dynamicColorSupported(): Boolean = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S

fun ThemeMode.isDark(systemDark: Boolean): Boolean = when (this) {
    ThemeMode.System -> systemDark
    ThemeMode.Light -> false
    ThemeMode.Dark -> true
}

/**
 * Colour on API 31+ comes from the wallpaper unless [dynamicColor] is off; the brand scheme
 * (design/tokens) serves API 26-30 and the "off" setting.
 */
@Composable
fun LibreUsosTheme(
    themeMode: ThemeMode = ThemeMode.System,
    dynamicColor: Boolean = true,
    content: @Composable () -> Unit,
) {
    val dark = themeMode.isDark(isSystemInDarkTheme())
    val context = LocalContext.current
    val colors: ColorScheme = when {
        dynamicColor && dynamicColorSupported() ->
            if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        dark -> BrandDark
        else -> BrandLight
    }

    // The system bars follow the chosen mode, not the system one, when the two differ.
    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = context.findActivity()?.window ?: return@SideEffect
            val controller = WindowCompat.getInsetsController(window, view)
            controller.isAppearanceLightStatusBars = !dark
            controller.isAppearanceLightNavigationBars = !dark
        }
    }

    CompositionLocalProvider(LocalIsDark provides dark, LocalMonoStyles provides LibreMonoStyles) {
        MaterialTheme(colorScheme = colors, typography = LibreTypography, shapes = LibreShapes, content = content)
    }
}

private tailrec fun Context.findActivity(): Activity? = when (this) {
    is Activity -> this
    is ContextWrapper -> baseContext.findActivity()
    else -> null
}

/**
 * Fill of grouped lists and the inner panel of the now card (`--group-bg`): the lightest container in
 * light mode, one step up from the page in dark mode.
 */
val ColorScheme.groupContainer: Color
    @Composable get() = if (LocalIsDark.current) surfaceContainerLow else surfaceContainerLowest
