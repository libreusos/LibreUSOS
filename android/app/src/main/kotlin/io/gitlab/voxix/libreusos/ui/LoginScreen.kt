package io.gitlab.voxix.libreusos.ui

import androidx.activity.compose.BackHandler
import androidx.annotation.DrawableRes
import androidx.annotation.StringRes
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.requiredSize
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.ui.components.Banner
import io.gitlab.voxix.libreusos.ui.components.BannerStyle
import io.gitlab.voxix.libreusos.ui.components.GroupedListCard
import io.gitlab.voxix.libreusos.ui.components.SectionHeader
import io.gitlab.voxix.libreusos.ui.theme.GeistMonoFamily
import io.gitlab.voxix.libreusos.ui.theme.LibreType
import io.gitlab.voxix.libreusos.ui.theme.Radii
import io.gitlab.voxix.libreusos.ui.theme.Spacing

/** Everything the sign-in screens can ask for; the view model implements each. */
class LoginCallbacks(
    val onBaseUrlChange: (String) -> Unit,
    val onConsumerKeyChange: (String) -> Unit,
    val onConsumerSecretChange: (String) -> Unit,
    /** Sign in at the university with the built-in key (m-01). */
    val onLoginKnown: () -> Unit,
    /** Sign in with the address and key typed on the form (m-02), or again from a result screen. */
    val onLogin: () -> Unit,
    val onOpenOther: () -> Unit,
    val onChooseUniversity: () -> Unit,
    val onAbandon: () -> Unit,
    val onReopenPage: () -> Unit,
)

/** The sign-in flow (SYSTEM.md sections 11 and 12): university list, other university, browser step, outcomes. */
@Composable
fun LoginScreen(state: LoginFlowState, callbacks: LoginCallbacks) {
    when (state.stage) {
        LoginStage.Universities -> UniversitiesScreen(state, callbacks)
        LoginStage.OtherUniversity -> OtherUniversityScreen(state, callbacks)
        LoginStage.Waiting, LoginStage.Finishing -> BrowserStepScreen(state, callbacks)
        LoginStage.Cancelled, LoginStage.TimedOut, LoginStage.Denied, LoginStage.Failed -> OutcomeScreen(state, callbacks)
    }
}

/** The page frame: safe-area padding, a scrolling body, and the actions pinned to the bottom. */
@Composable
private fun LoginFrame(
    actions: (@Composable ColumnScope.() -> Unit)?,
    body: @Composable ColumnScope.() -> Unit,
) {
    Column(Modifier.fillMaxSize().safeDrawingPadding().padding(horizontal = Spacing.screenEdge)) {
        Column(Modifier.weight(1f).verticalScroll(rememberScrollState()), content = body)
        if (actions != null) {
            Column(
                Modifier.padding(top = Spacing.s12, bottom = Spacing.s16),
                verticalArrangement = Arrangement.spacedBy(Spacing.s8),
                content = actions,
            )
        }
    }
}

/** The 56 dp filled button; while [busy] the label is a 16 dp spinner and the width does not change. */
@Composable
private fun LoginButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true, busy: Boolean = false) {
    Button(
        onClick = { if (!busy) onClick() },
        modifier = modifier.fillMaxWidth().defaultMinSize(minHeight = 56.dp),
        enabled = enabled,
        shape = RoundedCornerShape(Radii.control),
    ) {
        if (busy) {
            CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp, color = LocalContentColor.current)
        } else {
            Text(text, style = MaterialTheme.typography.labelLarge)
        }
    }
}

@Composable
private fun BackRow(onBack: (() -> Unit)?, title: String? = null) {
    Row(Modifier.fillMaxWidth().height(Spacing.touchTarget + Spacing.s8), verticalAlignment = Alignment.CenterVertically) {
        if (onBack != null) {
            IconButton(onBack, Modifier.size(Spacing.touchTarget)) {
                Icon(painterResource(R.drawable.ic_arrow_back), stringResource(R.string.action_back))
            }
        }
        if (title != null) {
            Text(
                title,
                Modifier.padding(start = Spacing.s8).semantics { heading() },
                style = MaterialTheme.typography.headlineSmall,
            )
        }
    }
}

// ---------------------------------------------------------------------------------------------
// m-01: universities
// ---------------------------------------------------------------------------------------------

@Composable
private fun UniversitiesScreen(state: LoginFlowState, callbacks: LoginCallbacks) {
    val known = SIGN_IN_UNIVERSITIES.first()
    LoginFrame(
        actions = {
            LoginButton(
                text = stringResource(R.string.onboarding_sign_in_with, known.name),
                onClick = callbacks.onLoginKnown,
                busy = state.busy,
            )
        },
    ) {
        Box(Modifier.padding(top = Spacing.s24).size(width = 40.dp, height = 48.dp), contentAlignment = Alignment.Center) {
            Icon(
                painterResource(R.drawable.ic_launcher_monochrome),
                stringResource(R.string.onboarding_logo_description),
                Modifier.requiredSize(76.dp),
                tint = MaterialTheme.colorScheme.primary,
            )
        }
        Text(
            stringResource(R.string.onboarding_universities_title),
            Modifier.padding(top = Spacing.s32).semantics { heading() },
            style = MaterialTheme.typography.headlineMedium,
        )
        Text(
            stringResource(R.string.onboarding_universities_body),
            Modifier.padding(top = Spacing.s8),
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Column(Modifier.padding(top = Spacing.s20), verticalArrangement = Arrangement.spacedBy(Spacing.s8)) {
            if (state.sessionExpired) {
                Banner(stringResource(R.string.error_session_expired), style = BannerStyle.Error)
            }
            state.error?.let { Banner(stringResource(it.messageRes), style = BannerStyle.Error) }
            if (state.warnings.isNotEmpty()) {
                Banner(
                    stringResource(R.string.onboarding_logout_warnings) + "\n" + state.warnings.joinToString("\n"),
                    icon = R.drawable.ic_warning,
                )
            }
        }
        GroupedListCard(Modifier.padding(top = Spacing.s20)) {
            row {
                UniversityRow(
                    name = known.name,
                    host = hostOf(known.baseUrl),
                    keyLabel = if (state.knownHasKey) R.string.onboarding_key_embedded else R.string.onboarding_key_required,
                )
            }
        }
        Row(
            Modifier
                .padding(top = Spacing.s12)
                .fillMaxWidth()
                .defaultMinSize(minHeight = Spacing.touchTarget)
                .clip(RoundedCornerShape(Radii.chip))
                .clickable(role = Role.Button, onClick = callbacks.onOpenOther)
                .padding(horizontal = Spacing.s12),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Spacing.s12),
        ) {
            Icon(painterResource(R.drawable.ic_link), null, Modifier.size(24.dp), tint = MaterialTheme.colorScheme.primary)
            Text(
                stringResource(R.string.onboarding_other_link),
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.primary,
            )
        }
    }
}

/** One selectable university: radio, name, and "host · key label" with the host in mono. */
@Composable
private fun UniversityRow(name: String, host: String, keyLabel: Int) {
    val label = stringResource(keyLabel)
    val mono = SpanStyle(fontFamily = GeistMonoFamily)
    Row(
        Modifier
            .fillMaxWidth()
            .defaultMinSize(minHeight = Spacing.rowMinHeight)
            .selectable(selected = true, onClick = {}, role = Role.RadioButton)
            .padding(horizontal = Spacing.rowPaddingHorizontal, vertical = Spacing.rowPaddingVertical),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(Spacing.iconGap),
    ) {
        RadioButton(selected = true, onClick = null)
        Column(Modifier.weight(1f)) {
            Text(name, style = MaterialTheme.typography.bodyLarge)
            Text(
                buildAnnotatedString {
                    withStyle(mono) { append(host) }
                    append(" · ")
                    append(label)
                },
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// ---------------------------------------------------------------------------------------------
// m-02: other university
// ---------------------------------------------------------------------------------------------

@Composable
private fun OtherUniversityScreen(state: LoginFlowState, callbacks: LoginCallbacks) {
    BackHandler(onBack = callbacks.onChooseUniversity)
    val uriHandler = LocalUriHandler.current
    var secretShown by rememberSaveable { mutableStateOf(false) }
    val urlProblem = state.urlProblem
    val fieldShape = RoundedCornerShape(Radii.control)
    val mono = LibreType.monoLarge
    LoginFrame(
        actions = {
            LoginButton(
                text = stringResource(R.string.onboarding_submit_other),
                onClick = callbacks.onLogin,
                enabled = state.canSubmitOther || state.busy,
                busy = state.busy,
            )
            val hint = when {
                urlProblem != null -> R.string.onboarding_hint_fix_url
                !state.canSubmitOther && !state.busy -> R.string.onboarding_hint_fill_in
                else -> null
            }
            // Keeps its height so the button does not jump when the hint goes.
            Text(
                hint?.let { stringResource(it) }.orEmpty(),
                Modifier.fillMaxWidth().defaultMinSize(minHeight = 20.dp),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = androidx.compose.ui.text.style.TextAlign.Center,
            )
        },
    ) {
        BackRow(callbacks.onChooseUniversity, stringResource(R.string.onboarding_other_title))
        Text(
            stringResource(R.string.onboarding_other_body),
            Modifier.padding(top = Spacing.s8),
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        state.error?.let { Banner(stringResource(it.messageRes), Modifier.padding(top = Spacing.s16), style = BannerStyle.Error) }
        if (state.sessionExpired) {
            Banner(stringResource(R.string.error_session_expired), Modifier.padding(top = Spacing.s16), style = BannerStyle.Error)
        }
        OutlinedTextField(
            value = state.baseUrl,
            onValueChange = callbacks.onBaseUrlChange,
            modifier = Modifier.fillMaxWidth().padding(top = Spacing.s24),
            label = { Text(stringResource(R.string.onboarding_url_label)) },
            textStyle = mono,
            singleLine = true,
            isError = urlProblem != null,
            supportingText = urlProblem?.let { problem -> { ErrorHelp(stringResource(problem.messageRes())) } },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
            shape = fieldShape,
        )
        if (state.needsKey) {
            SectionHeader(stringResource(R.string.onboarding_key_section))
            OutlinedTextField(
                value = state.consumerKey,
                onValueChange = callbacks.onConsumerKeyChange,
                modifier = Modifier.fillMaxWidth(),
                label = { Text(stringResource(R.string.onboarding_consumer_key)) },
                textStyle = mono,
                singleLine = true,
                isError = state.keyRejected,
                shape = fieldShape,
            )
            OutlinedTextField(
                value = state.consumerSecret,
                onValueChange = callbacks.onConsumerSecretChange,
                modifier = Modifier.fillMaxWidth().padding(top = Spacing.s12),
                label = { Text(stringResource(R.string.onboarding_consumer_secret)) },
                textStyle = mono,
                singleLine = true,
                isError = state.keyRejected,
                visualTransformation = if (secretShown) VisualTransformation.None else PasswordVisualTransformation(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                trailingIcon = {
                    IconButton({ secretShown = !secretShown }) {
                        Icon(
                            painterResource(if (secretShown) R.drawable.ic_visibility_off else R.drawable.ic_visibility),
                            stringResource(if (secretShown) R.string.onboarding_hide_secret else R.string.onboarding_show_secret),
                        )
                    }
                },
                supportingText = {
                    if (state.keyRejected) {
                        ErrorHelp(stringResource(R.string.onboarding_key_rejected))
                    } else {
                        Text(stringResource(R.string.onboarding_secret_note))
                    }
                },
                shape = fieldShape,
            )
            if (urlProblem == null && state.baseUrl.isNotBlank()) {
                val host = hostOf(state.baseUrl)
                val developersUrl = state.baseUrl.trim().trimEnd('/') + "/developers/"
                Surface(
                    onClick = { uriHandler.openUri(developersUrl) },
                    modifier = Modifier.padding(top = Spacing.s16).fillMaxWidth(),
                    shape = RoundedCornerShape(Radii.group),
                    color = Color.Transparent,
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                ) {
                    Row(
                        Modifier.padding(Spacing.s16),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Spacing.s16),
                    ) {
                        Icon(painterResource(R.drawable.ic_open_in_new), null, Modifier.size(24.dp), tint = MaterialTheme.colorScheme.primary)
                        Column {
                            Text(stringResource(R.string.onboarding_generate_key), style = MaterialTheme.typography.bodyLarge)
                            Text(
                                "$host/developers/",
                                style = LibreType.monoMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }
        }
        Spacer(Modifier.height(Spacing.s16))
    }
}

@StringRes
private fun UrlProblem.messageRes(): Int = when (this) {
    UrlProblem.NoUsosApi -> R.string.onboarding_url_error_no_api
    UrlProblem.NotAnAddress -> R.string.onboarding_url_error_invalid
    UrlProblem.NotHttps -> R.string.onboarding_url_error_https
}

/** Field help in the error colour with an icon, so the state is not colour alone. */
@Composable
private fun ErrorHelp(text: String) {
    Row(horizontalArrangement = Arrangement.spacedBy(Spacing.s8)) {
        Icon(painterResource(R.drawable.ic_error), null, Modifier.padding(top = 1.dp).size(16.dp))
        Text(text)
    }
}

// ---------------------------------------------------------------------------------------------
// m-03 / m-sheet-oauth: the browser step
// ---------------------------------------------------------------------------------------------

private enum class StepState { Pending, Current, Done }

@Composable
private fun BrowserStepScreen(state: LoginFlowState, callbacks: LoginCallbacks) {
    val finishing = state.stage == LoginStage.Finishing
    // While finishing, going back would orphan a login that is already half done: the arrow is hidden.
    BackHandler(enabled = true) { if (!finishing) callbacks.onAbandon() }
    val steps = if (finishing) {
        listOf(StepState.Done, StepState.Done, StepState.Current)
    } else {
        listOf(StepState.Current, StepState.Pending, StepState.Pending)
    }
    LoginFrame(
        actions = if (finishing) {
            null
        } else {
            {
                OutlinedButton(
                    onClick = callbacks.onReopenPage,
                    modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = 56.dp),
                    shape = RoundedCornerShape(Radii.control),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                ) {
                    Icon(painterResource(R.drawable.ic_open_in_new), null, Modifier.size(20.dp))
                    Spacer(Modifier.width(Spacing.s8))
                    Text(stringResource(R.string.onboarding_reopen_page), style = MaterialTheme.typography.labelLarge)
                }
            }
        },
    ) {
        BackRow(if (finishing) null else callbacks.onAbandon)
        StepStrip(filled = steps.count { it == StepState.Done } + 1, Modifier.padding(top = Spacing.s4))
        Text(
            stringResource(R.string.onboarding_waiting_title),
            Modifier.padding(top = Spacing.s24).semantics { heading() },
            style = MaterialTheme.typography.headlineMedium,
        )
        Text(
            stringResource(R.string.onboarding_waiting_body, state.authHost.ifBlank { hostOf(state.baseUrl) }),
            Modifier.padding(top = Spacing.s12),
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Column(Modifier.padding(top = Spacing.s32), verticalArrangement = Arrangement.spacedBy(Spacing.s20)) {
            StepRow(1, steps[0], stringResource(R.string.onboarding_step_1_title), state.authHost.ifBlank { hostOf(state.baseUrl) })
            StepRow(2, steps[1], stringResource(R.string.onboarding_step_2_title), stringResource(R.string.onboarding_step_2_body))
            StepRow(3, steps[2], stringResource(R.string.onboarding_step_3_title), stringResource(R.string.onboarding_step_3_body))
        }
        val progress = when {
            !finishing -> R.string.onboarding_progress_waiting
            state.finishStep == FinishStep.Exchanging -> R.string.onboarding_progress_exchanging
            else -> R.string.onboarding_progress_syncing
        }
        Row(
            Modifier.padding(top = Spacing.s24).semantics { liveRegion = LiveRegionMode.Polite },
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Spacing.s8),
        ) {
            if (finishing) CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
            Text(
                stringResource(progress),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

/** Three pills like the lit slot strip of the week motif; [filled] of them are in the primary colour. */
@Composable
private fun StepStrip(filled: Int, modifier: Modifier = Modifier) {
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(Spacing.s4)) {
        repeat(3) { index ->
            val on = index < filled
            Box(
                Modifier
                    .size(width = 28.dp, height = 8.dp)
                    .clip(RoundedCornerShape(Radii.badge))
                    .background(if (on) MaterialTheme.colorScheme.primary else Color.Transparent)
                    .border(1.dp, if (on) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(Radii.badge)),
            )
        }
    }
}

@Composable
private fun StepRow(number: Int, state: StepState, title: String, subtitle: String) {
    val colors = MaterialTheme.colorScheme
    val statusText = when (state) {
        StepState.Done -> stringResource(R.string.onboarding_step_done)
        StepState.Current -> stringResource(R.string.onboarding_step_current)
        StepState.Pending -> ""
    }
    Row(
        Modifier.fillMaxWidth().semantics(mergeDescendants = true) {
            contentDescription = listOf("$number", title, subtitle, statusText).filter { it.isNotEmpty() }.joinToString(", ")
        },
        horizontalArrangement = Arrangement.spacedBy(Spacing.s16),
    ) {
        Box(
            Modifier
                .size(28.dp)
                .clip(CircleShape)
                .background(if (state == StepState.Done) colors.primary else Color.Transparent)
                .border(
                    if (state == StepState.Current) 2.dp else 1.dp,
                    when (state) {
                        StepState.Done, StepState.Current -> colors.primary
                        StepState.Pending -> colors.outlineVariant
                    },
                    CircleShape,
                ),
            contentAlignment = Alignment.Center,
        ) {
            if (state == StepState.Done) {
                Icon(painterResource(R.drawable.ic_check), null, Modifier.size(16.dp), tint = colors.onPrimary)
            } else {
                Text(
                    "$number",
                    style = MaterialTheme.typography.labelMedium,
                    color = if (state == StepState.Current) colors.primary else colors.onSurfaceVariant,
                )
            }
        }
        Column(Modifier.weight(1f)) {
            Text(
                title,
                style = MaterialTheme.typography.bodyLarge,
                color = if (state == StepState.Pending) colors.onSurfaceVariant else colors.onSurface,
            )
            Text(subtitle, style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceVariant)
        }
    }
}

// ---------------------------------------------------------------------------------------------
// m-sheet-oauth: cancelled, timeout, denied, failed
// ---------------------------------------------------------------------------------------------

@Composable
private fun OutcomeScreen(state: LoginFlowState, callbacks: LoginCallbacks) {
    BackHandler(onBack = callbacks.onAbandon)
    @DrawableRes val icon: Int
    @StringRes val title: Int
    @StringRes val body: Int
    when (state.stage) {
        LoginStage.Cancelled -> {
            icon = R.drawable.ic_info; title = R.string.onboarding_cancelled_title; body = R.string.onboarding_cancelled_body
        }
        LoginStage.TimedOut -> {
            icon = R.drawable.ic_hourglass_top; title = R.string.onboarding_timeout_title; body = R.string.onboarding_timeout_body
        }
        LoginStage.Denied -> {
            icon = R.drawable.ic_block; title = R.string.onboarding_denied_title; body = R.string.onboarding_denied_body
        }
        else -> {
            icon = R.drawable.ic_error; title = R.string.onboarding_failed_title; body = R.string.onboarding_failed_body
        }
    }
    var detailsShown by rememberSaveable { mutableStateOf(false) }
    LoginFrame(
        actions = {
            LoginButton(stringResource(R.string.onboarding_retry), callbacks.onLogin, busy = state.busy)
            TextButton(
                onClick = callbacks.onChooseUniversity,
                modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = Spacing.touchTarget),
            ) {
                Text(stringResource(R.string.onboarding_choose_other), style = MaterialTheme.typography.labelLarge)
            }
        },
    ) {
        BackRow(callbacks.onAbandon)
        Box(
            Modifier
                .padding(top = Spacing.s16)
                .size(48.dp)
                .clip(RoundedCornerShape(Radii.banner))
                .background(MaterialTheme.colorScheme.surfaceContainerHigh),
            contentAlignment = Alignment.Center,
        ) {
            Icon(painterResource(icon), null, Modifier.size(24.dp))
        }
        Text(
            stringResource(title),
            Modifier.padding(top = Spacing.s20).semantics { heading() },
            style = MaterialTheme.typography.headlineMedium,
        )
        Text(
            stringResource(body),
            Modifier.padding(top = Spacing.s12),
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        val kind = state.error
        if (state.stage == LoginStage.Failed && kind != null) {
            TextButton({ detailsShown = !detailsShown }, Modifier.padding(top = Spacing.s8)) {
                Text(stringResource(R.string.onboarding_details), style = MaterialTheme.typography.labelLarge)
                Spacer(Modifier.width(Spacing.s4))
                Icon(
                    painterResource(if (detailsShown) R.drawable.ic_expand_less else R.drawable.ic_expand_more),
                    null,
                    Modifier.size(20.dp),
                )
            }
            if (detailsShown) {
                Text(
                    stringResource(kind.messageRes),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
