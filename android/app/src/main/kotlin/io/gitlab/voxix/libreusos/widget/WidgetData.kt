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

/**
 * Everything the widget draws, read once per update. [day] is null unless [mode] is Content.
 * [syncedAtMs] is the newest sign that the data is current: the last successful sync, or the cache's own stamp.
 */
data class WidgetSnapshot(
    val mode: WidgetMode,
    val day: WidgetDay?,
    val zone: ZoneId,
    val now: LocalDateTime,
    val syncedAtMs: Long?,
    val expired: Boolean,
    /** Follows the app's "wallpaper colours" setting. */
    val dynamicColor: Boolean,
)

/** The cache range of an update: today and the six days after it. */
private const val RANGE_DAYS = 6L

/**
 * One `cachedTimetable` read from the shared client (no network, no second store). Safe on any thread.
 * The token is checked first: a cache left by a previous account must never show. Any failure gives Loading.
 */
suspend fun loadWidgetSnapshot(context: Context): WidgetSnapshot = withContext(Dispatchers.IO) {
    var zone = ZoneId.systemDefault()
    var now = LocalDateTime.now(zone)
    var dynamic = true
    try {
        val container = (context.applicationContext as LibreUsosApp).container
        dynamic = container.settings.appearance.value.dynamicColor
        zone = ZoneId.of(installationTimeZone())
        now = LocalDateTime.now(zone)
        val record = container.syncStore.record.value
        val client = container.client(container.settings.baseUrl)
        val hasToken = client?.hasToken() == true
        var day: WidgetDay? = null
        var cachedAtMs: Long? = null
        if (client != null && hasToken) {
            try {
                val today = now.toLocalDate()
                val cached = client.cachedTimetable(today.toString(), today.plusDays(RANGE_DAYS).toString())
                cachedAtMs = (cached.source as? DataSource.Cached)?.fetchedAt?.let { it * 1000 }
                day = widgetDay(timed(cached.activities), now)
            } catch (_: UsosException.NotCached) {
            }
        }
        val mode = widgetMode(hasToken, day != null, record.expired)
        WidgetSnapshot(
            mode, day.takeIf { mode == WidgetMode.Content }, zone, now,
            listOfNotNull(record.lastSuccessMs, cachedAtMs).maxOrNull(), record.expired, dynamic,
        )
    } catch (e: CancellationException) {
        throw e
    } catch (t: Throwable) {
        logFailure(t)
        WidgetSnapshot(WidgetMode.Loading, null, zone, now, null, false, dynamic)
    }
}
