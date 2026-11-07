package io.github.voxix.libreusos.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import io.github.voxix.libreusos.R
import io.github.voxix.libreusos.data.fetchedLabel
import org.libreusos.core.DataSource
import java.time.ZoneId

@Composable
fun HomeScreen(state: HomeUiState, logoutError: ErrorKind?, onRefresh: () -> Unit, onLogout: () -> Unit) {
    val lang = LocalConfiguration.current.locales[0].language
    Column(
        modifier = Modifier
            .fillMaxSize()
            .safeDrawingPadding()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(stringResource(R.string.home_title), style = MaterialTheme.typography.headlineMedium)
        state.identity?.user?.let { user ->
            Text(
                stringResource(R.string.home_logged_in_as, "${user.firstName} ${user.lastName}"),
                style = MaterialTheme.typography.titleMedium,
            )
        }
        if (state.loading) LinearProgressIndicator(Modifier.fillMaxWidth())
        (state.errors + listOfNotNull(logoutError)).forEach {
            Text(
                stringResource(R.string.error_prefix, stringResource(it.messageRes)),
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodyMedium,
            )
        }
        state.timetable?.let { timetable ->
            Text(stringResource(R.string.home_timetable), style = MaterialTheme.typography.titleMedium)
            Text(sourceLabel(timetable.source, state.zone), style = MaterialTheme.typography.bodySmall)
            if (timetable.activities.isEmpty()) {
                Text(stringResource(R.string.home_no_classes))
            }
        }
        LazyColumn(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items(state.timetable?.activities.orEmpty()) { ActivityCard(it, lang) }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(onClick = onRefresh, enabled = !state.loading) {
                Text(stringResource(R.string.home_refresh))
            }
            OutlinedButton(onClick = onLogout) { Text(stringResource(R.string.home_logout)) }
        }
    }
}

@Composable
private fun sourceLabel(source: DataSource, zone: ZoneId?): String = when (source) {
    DataSource.Live -> stringResource(R.string.home_source_live)
    is DataSource.Cached -> stringResource(
        R.string.home_source_cached,
        fetchedLabel(source.fetchedAt, zone ?: ZoneId.systemDefault(), LocalConfiguration.current.locales[0]),
    )
}
