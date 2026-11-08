package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.ui.components.DrillDownScreen
import io.gitlab.voxix.libreusos.ui.components.SectionHeader
import io.gitlab.voxix.libreusos.ui.theme.LibreType
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** A bundled third-party asset: what it is and the licence text file shipped in `assets/licenses/`. */
private class LicenceEntry(val title: String, val asset: String)

private val entries = listOf(
    LicenceEntry("Geist, Geist Mono (SIL OFL 1.1)", "licenses/Geist-OFL-1.1.txt"),
    LicenceEntry("Material Symbols (Apache 2.0)", "licenses/MaterialSymbols-Apache-2.0.txt"),
)

/** Open-source licences of the bundled fonts and icons. */
@Composable
fun LicencesScreen(onBack: () -> Unit) {
    val context = LocalContext.current
    var texts by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
    LaunchedEffect(Unit) {
        texts = withContext(Dispatchers.IO) {
            entries.associate { entry ->
                entry.asset to runCatching { context.assets.open(entry.asset).bufferedReader().use { it.readText() } }
                    .getOrDefault("")
            }
        }
    }
    DrillDownScreen(stringResource(R.string.settings_licences), onBack) { padding ->
        Column(
            Modifier
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = Spacing.screenEdge),
        ) {
            entries.forEach { entry ->
                SectionHeader(entry.title)
                Text(
                    texts[entry.asset].orEmpty().trim(),
                    style = LibreType.monoMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Spacer(Modifier.size(Spacing.s16))
        }
    }
}
