package io.gitlab.voxix.libreusos.ui

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

// Fixed palette for Android 11 and older, which have no dynamic color.
private val LightColors = lightColorScheme(
    primary = Color(0xFF3F5AA6),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFDCE1FF),
    onPrimaryContainer = Color(0xFF001551),
    secondary = Color(0xFF595E72),
    background = Color(0xFFFEFBFF),
    surface = Color(0xFFFEFBFF),
    error = Color(0xFFBA1A1A),
)

private val DarkColors = darkColorScheme(
    primary = Color(0xFFB6C4FF),
    onPrimary = Color(0xFF0B2977),
    primaryContainer = Color(0xFF264190),
    onPrimaryContainer = Color(0xFFDCE1FF),
    secondary = Color(0xFFC1C5DD),
    background = Color(0xFF1B1B1F),
    surface = Color(0xFF1B1B1F),
    error = Color(0xFFFFB4AB),
)

@Composable
fun LibreUsosTheme(content: @Composable () -> Unit) {
    val dark = isSystemInDarkTheme()
    val colors = when {
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        dark -> DarkColors
        else -> LightColors
    }
    MaterialTheme(colorScheme = colors, content = content)
}
