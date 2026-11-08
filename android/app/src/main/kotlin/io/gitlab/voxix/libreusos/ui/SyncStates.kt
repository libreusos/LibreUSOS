package io.gitlab.voxix.libreusos.ui

import android.content.ClipData
import android.os.Build
import androidx.annotation.StringRes
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import io.gitlab.voxix.libreusos.R
import io.gitlab.voxix.libreusos.ui.components.Banner
import io.gitlab.voxix.libreusos.ui.components.BannerStyle
import io.gitlab.voxix.libreusos.ui.components.StatusLine
import io.gitlab.voxix.libreusos.ui.components.SyncState
import io.gitlab.voxix.libreusos.ui.components.WeekMotif
import io.gitlab.voxix.libreusos.ui.theme.LibreType
import io.gitlab.voxix.libreusos.ui.theme.Radii
import io.gitlab.voxix.libreusos.ui.theme.Spacing
import kotlinx.coroutines.launch
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter

/** The strings of the sync states that name what is fetched: the plan by default. */
internal data class SyncCopy(
    @StringRes val fetchingFirst: Int = R.string.today_fetching_first,
    @StringRes val bannerSyncError: Int = R.string.today_banner_sync_error,
    @StringRes val bannerSyncErrorGeneric: Int = R.string.today_banner_sync_error_generic,
    @StringRes val bannerExpired: Int = R.string.today_banner_expired,
    @StringRes val offlineBody: Int = R.string.today_offline_body,
    @StringRes val failedTitle: Int = R.string.today_failed_title,
    @StringRes val expiredBody: Int = R.string.today_expired_body,
)

/** The copy of the Oceny tabs: grades and tests. */
internal val GradesCopy = SyncCopy(
    fetchingFirst = R.string.grades_fetching_first,
    bannerSyncError = R.string.grades_banner_sync_error,
    bannerSyncErrorGeneric = R.string.grades_banner_sync_error_generic,
    bannerExpired = R.string.grades_banner_expired,
    offlineBody = R.string.grades_offline_body,
    failedTitle = R.string.grades_failed_title,
    expiredBody = R.string.grades_expired_body,
)

/** The sync banner of Today and Plan: offline, sync error with retry and details, or expired session. */
@Composable
internal fun SyncBanner(
    banner: HomeBanner,
    failure: HomeFailure?,
    dataStamp: String,
    retryIn: Int?,
    onRetry: () -> Unit,
    onDetails: () -> Unit,
    onRelogin: () -> Unit,
    modifier: Modifier = Modifier,
    copy: SyncCopy = SyncCopy(),
) {
    when (banner) {
        HomeBanner.None -> Unit
        HomeBanner.Offline -> Banner(stringResource(R.string.today_banner_offline, dataStamp), modifier)
        HomeBanner.SyncError -> {
            val base = stringResource(
                if (failure?.kind == ErrorKind.NETWORK || failure?.kind == ErrorKind.API) {
                    copy.bannerSyncError
                } else {
                    copy.bannerSyncErrorGeneric
                },
            )
            val text = retryIn?.let { base + " " + stringResource(R.string.today_banner_retry_in, spanText(it)) } ?: base
            Banner(
                text,
                modifier,
                style = BannerStyle.Error,
                actionLabel = stringResource(R.string.today_action_retry),
                onAction = onRetry,
                secondaryLabel = stringResource(R.string.today_action_details),
                onSecondary = onDetails,
            )
        }
        HomeBanner.Expired -> Banner(
            stringResource(copy.bannerExpired),
            modifier,
            style = BannerStyle.Error,
            icon = R.drawable.ic_lock_clock,
            actionLabel = stringResource(R.string.today_action_relogin),
            onAction = onRelogin,
        )
    }
}

@Composable
internal fun statusText(
    status: HomeStatus,
    mode: HomeMode,
    state: HomeUiState,
    zone: ZoneId,
    today: LocalDate,
    locale: java.util.Locale,
    copy: SyncCopy = SyncCopy(),
): String? {
    val data = state.dataAtSec
    return when (status) {
        HomeStatus.Hidden -> null
        HomeStatus.Fetching -> stringResource(if (mode == HomeMode.Skeleton) copy.fetchingFirst else R.string.today_fetching)
        HomeStatus.Synced -> stringResource(R.string.today_status_synced, data?.let { stampText(it, zone, today, locale) }.orEmpty())
        HomeStatus.DataFromFailed -> {
            val from = data?.let { if (stampDay(it, zone, today) == StampDay.Today) clockOf(it, zone) else stampText(it, zone, today, locale) }.orEmpty()
            val attempt = state.failure?.let { clockOf(it.atMs / 1000, zone) }.orEmpty()
            stringResource(R.string.today_status_data_from_failed, from, attempt)
        }
        HomeStatus.LastSync -> stringResource(R.string.today_status_last_sync, data?.let { stampText(it, zone, today, locale) }.orEmpty())
        HomeStatus.OfflineNoData -> stringResource(R.string.today_status_offline_no_data)
        HomeStatus.FirstSyncFailed -> stringResource(R.string.today_status_first_failed)
    }
}

/** The status line under the title; the dot follows [status]. */
@Composable
internal fun SyncStatusLine(status: HomeStatus, text: String) {
    StatusLine(
        when {
            status.isFailure -> SyncState.Failed
            status == HomeStatus.OfflineNoData -> SyncState.Offline
            else -> SyncState.Fresh
        },
        text,
    )
}

/** The full-screen state of a [mode] with nothing cached (offline, failed, expired). */
@Composable
internal fun NoCacheContent(
    mode: HomeMode,
    failure: HomeFailure?,
    retryIn: Int?,
    onRetry: () -> Unit,
    onDetails: () -> Unit,
    onRelogin: () -> Unit,
    copy: SyncCopy = SyncCopy(),
) {
    when (mode) {
        HomeMode.NoCacheOffline -> NoCacheState(
            title = stringResource(R.string.today_offline_title),
            body = stringResource(copy.offlineBody),
            actionLabel = stringResource(R.string.today_action_retry),
            onAction = onRetry,
        )
        HomeMode.NoCacheError -> {
            val server = failure?.kind == ErrorKind.NETWORK || failure?.kind == ErrorKind.API
            NoCacheState(
                title = stringResource(if (server) R.string.today_server_title else copy.failedTitle),
                body = if (server) {
                    retryIn?.let { stringResource(R.string.today_server_body, spanText(it)) } ?: stringResource(R.string.today_server_body_now)
                } else {
                    stringResource(R.string.today_failed_body)
                },
                actionLabel = stringResource(R.string.today_action_retry),
                onAction = onRetry,
                secondaryLabel = stringResource(R.string.today_action_details),
                onSecondary = onDetails,
            )
        }
        HomeMode.NoCacheExpired -> NoCacheState(
            title = stringResource(R.string.today_expired_title),
            body = stringResource(copy.expiredBody),
            actionLabel = stringResource(R.string.today_action_relogin),
            onAction = onRelogin,
        )
        HomeMode.Skeleton, HomeMode.Content -> Unit
    }
}

/** Full-screen state when nothing is cached: the week motif, a title, one sentence and the way out. */
@Composable
internal fun NoCacheState(
    title: String,
    body: String,
    actionLabel: String,
    onAction: () -> Unit,
    secondaryLabel: String? = null,
    onSecondary: (() -> Unit)? = null,
) {
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = Spacing.s16, vertical = Spacing.s32),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        WeekMotif(legend = false)
        Spacer(Modifier.height(Spacing.s24))
        Text(title, style = MaterialTheme.typography.headlineSmall, textAlign = TextAlign.Center)
        Spacer(Modifier.height(Spacing.s8))
        Text(body, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
        Spacer(Modifier.height(Spacing.s24))
        Button(onAction) { Text(actionLabel) }
        if (secondaryLabel != null && onSecondary != null) {
            TextButton(onSecondary) { Text(secondaryLabel) }
        }
    }
}

/** The four lines of the technical details: what failed, where, when, and which build. */
@Composable
internal fun detailsText(failure: HomeFailure, baseUrl: String, zone: ZoneId): String {
    val first = failure.status?.let { "HTTP $it ${failure.reason.orEmpty()}".trim() }
        ?: failure.errorClass.ifBlank { failure.kind.name }
    val host = runCatching { java.net.URI(baseUrl).host }.getOrNull() ?: baseUrl
    val at = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss").format(Instant.ofEpochMilli(failure.atMs).atZone(zone))
    val context = LocalContext.current
    val version = remember(context) {
        runCatching { context.packageManager.getPackageInfo(context.packageName, 0).versionName }.getOrNull().orEmpty()
    }
    return listOf(first, host, at, stringResource(R.string.today_details_app, version, Build.VERSION.RELEASE)).joinToString("\n")
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun DetailsSheet(text: String, onDismiss: () -> Unit) {
    val clipboard = LocalClipboard.current
    val scope = rememberCoroutineScope()
    ModalBottomSheet(onDismiss, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        Column(
            Modifier.fillMaxWidth().padding(horizontal = Spacing.s24).padding(bottom = Spacing.s24),
            verticalArrangement = Arrangement.spacedBy(Spacing.s12),
        ) {
            Text(stringResource(R.string.today_details_title), style = MaterialTheme.typography.titleLarge)
            Text(
                stringResource(R.string.today_details_hint),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Surface(
                Modifier.fillMaxWidth(),
                shape = androidx.compose.foundation.shape.RoundedCornerShape(Radii.block),
                color = MaterialTheme.colorScheme.surfaceContainerHigh,
            ) {
                SelectionContainer {
                    Text(text, Modifier.padding(Spacing.s12), style = LibreType.monoMedium)
                }
            }
            Button(
                onClick = { scope.launch { clipboard.setClipEntry(ClipEntry(ClipData.newPlainText("error details", text))) } },
                modifier = Modifier.align(Alignment.End),
            ) {
                Icon(painterResource(R.drawable.ic_content_copy), null, Modifier.size(18.dp))
                Spacer(Modifier.padding(start = Spacing.s8))
                Text(stringResource(R.string.today_details_copy))
            }
        }
    }
}
