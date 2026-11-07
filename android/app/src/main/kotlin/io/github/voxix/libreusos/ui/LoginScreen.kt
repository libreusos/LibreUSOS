package io.github.voxix.libreusos.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import io.github.voxix.libreusos.R

@Composable
fun LoginScreen(
    state: LoginUiState,
    onBaseUrlChange: (String) -> Unit,
    onConsumerKeyChange: (String) -> Unit,
    onConsumerSecretChange: (String) -> Unit,
    onLogin: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .safeDrawingPadding()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(stringResource(R.string.login_title), style = MaterialTheme.typography.headlineMedium)
        OutlinedTextField(
            value = state.baseUrl,
            onValueChange = onBaseUrlChange,
            label = { Text(stringResource(R.string.login_base_url)) },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
            modifier = Modifier.fillMaxWidth(),
        )
        // Key and secret only when this build embeds no consumer key for the server.
        if (state.needsKey) {
            Text(
                stringResource(R.string.login_key_needed),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.secondary,
            )
            OutlinedTextField(
                value = state.consumerKey,
                onValueChange = onConsumerKeyChange,
                label = { Text(stringResource(R.string.login_consumer_key)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = state.consumerSecret,
                onValueChange = onConsumerSecretChange,
                label = { Text(stringResource(R.string.login_consumer_secret)) },
                singleLine = true,
                visualTransformation = PasswordVisualTransformation(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                modifier = Modifier.fillMaxWidth(),
            )
        }
        Button(
            onClick = onLogin,
            enabled = !state.busy && state.baseUrl.isNotBlank(),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(stringResource(R.string.login_button))
        }
        if (state.busy) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp)
                Spacer(Modifier.size(12.dp))
                Text(stringResource(R.string.login_in_progress))
            }
        }
        state.error?.let { kind ->
            Text(
                stringResource(R.string.error_prefix, stringResource(kind.messageRes)),
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodyMedium,
            )
        }
        if (state.warnings.isNotEmpty()) {
            Text(stringResource(R.string.login_warnings), style = MaterialTheme.typography.titleSmall)
            state.warnings.forEach { Text(it, style = MaterialTheme.typography.bodyMedium) }
        }
    }
}
