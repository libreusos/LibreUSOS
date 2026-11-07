package io.gitlab.voxix.libreusos.widget

import android.content.Context
import io.gitlab.voxix.libreusos.LibreUsosApp
import io.gitlab.voxix.libreusos.ui.logFailure
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.libreusos.core.DataSource
import org.libreusos.core.UsosException
import org.libreusos.core.installationTimeZone
import java.time.LocalDateTime
import java.time.ZoneId

/** Everything the widget draws, read once per update. */
sealed interface WidgetContent {
    /** No token, or no consumer key: the cache is gone or unusable. */
    data object LoggedOut : WidgetContent

    /** Logged in, but nothing cached yet (or the cache could not be read). */
    data object NoData : WidgetContent

    /** [plan] is null when the cache holds no class today or later in the range. */
    data class Timetable(val plan: DayPlan?, val fetchedAtSec: Long?) : WidgetContent
}

data class WidgetSnapshot(val content: WidgetContent, val zone: ZoneId, val now: LocalDateTime)

/** The cache range of an update: today and the six days after it. */
private const val RANGE_DAYS = 6L

/**
 * One `cachedTimetable` read from the shared client (no network, no second store). Safe on any thread.
 * The token is checked first: a cache left by a previous account must never show. Any failure gives NoData.
 */
suspend fun loadWidgetSnapshot(context: Context): WidgetSnapshot = withContext(Dispatchers.IO) {
    var zone = ZoneId.systemDefault()
    var now = LocalDateTime.now(zone)
    try {
        val container = (context.applicationContext as LibreUsosApp).container
        zone = ZoneId.of(installationTimeZone())
        now = LocalDateTime.now(zone)
        val client = container.client(container.settings.baseUrl)
        if (client == null || !client.hasToken()) return@withContext WidgetSnapshot(WidgetContent.LoggedOut, zone, now)
        val content = try {
            val today = now.toLocalDate()
            val cached = client.cachedTimetable(today.toString(), today.plusDays(RANGE_DAYS).toString())
            val fetchedAt = (cached.source as? DataSource.Cached)?.fetchedAt
            WidgetContent.Timetable(pickDay(timed(cached.activities), now), fetchedAt)
        } catch (_: UsosException.NotCached) {
            WidgetContent.NoData
        }
        WidgetSnapshot(content, zone, now)
    } catch (e: CancellationException) {
        throw e
    } catch (t: Throwable) {
        logFailure(t)
        WidgetSnapshot(WidgetContent.NoData, zone, now)
    }
}
