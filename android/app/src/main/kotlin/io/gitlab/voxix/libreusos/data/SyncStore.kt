package io.gitlab.voxix.libreusos.data

import android.content.Context
import androidx.core.content.edit
import io.gitlab.voxix.libreusos.sync.SyncRecord
import io.gitlab.voxix.libreusos.sync.SyncScheduler
import io.gitlab.voxix.libreusos.sync.afterFailure
import io.gitlab.voxix.libreusos.sync.afterSuccess
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** The [SyncRecord] kept in preferences, shared by the sync worker and the Today screen. */
class SyncStore(private val context: Context) {
    private val prefs = context.getSharedPreferences("sync", Context.MODE_PRIVATE)

    private val _record = MutableStateFlow(
        SyncRecord(
            lastSuccessMs = prefs.getLong(KEY_SUCCESS, NONE).takeIf { it != NONE },
            failures = prefs.getInt(KEY_FAILURES, 0),
            lastFailureMs = prefs.getLong(KEY_LAST_FAILURE, NONE).takeIf { it != NONE },
            nextAttemptAtMs = prefs.getLong(KEY_NEXT, NONE).takeIf { it != NONE },
            expired = prefs.getBoolean(KEY_EXPIRED, false),
        ),
    )
    val record: StateFlow<SyncRecord> = _record.asStateFlow()

    /** A live fetch worked: the failure count starts again. A retry still waiting finds this and does nothing. */
    @Synchronized
    fun succeeded(nowMs: Long = System.currentTimeMillis()) = save(_record.value.afterSuccess(nowMs))

    /** A fetch failed (not a 401): notes it and plans the next automatic attempt. */
    @Synchronized
    fun failed(nowMs: Long = System.currentTimeMillis()) {
        val before = _record.value
        val after = before.afterFailure(nowMs)
        save(after)
        val next = after.nextAttemptAtMs
        if (next != null && next != before.nextAttemptAtMs) SyncScheduler.scheduleRetry(context, next - nowMs)
    }

    /** USOS rejected the token (true), or a new login replaced it (false). A retry is pointless while expired. */
    @Synchronized
    fun setExpired(expired: Boolean) = save(_record.value.copy(expired = expired, nextAttemptAtMs = null))

    /** Logout: forget everything, the retry included. */
    @Synchronized
    fun clear() = save(SyncRecord())

    private fun save(record: SyncRecord) {
        prefs.edit {
            putLongOrRemove(KEY_SUCCESS, record.lastSuccessMs)
            putInt(KEY_FAILURES, record.failures)
            putLongOrRemove(KEY_LAST_FAILURE, record.lastFailureMs)
            putLongOrRemove(KEY_NEXT, record.nextAttemptAtMs)
            putBoolean(KEY_EXPIRED, record.expired)
        }
        _record.value = record
    }

    private fun android.content.SharedPreferences.Editor.putLongOrRemove(key: String, value: Long?) {
        if (value == null) remove(key) else putLong(key, value)
    }

    private companion object {
        const val NONE = Long.MIN_VALUE
        const val KEY_SUCCESS = "last_success_ms"
        const val KEY_FAILURES = "failures"
        const val KEY_LAST_FAILURE = "last_failure_ms"
        const val KEY_NEXT = "next_attempt_at_ms"
        const val KEY_EXPIRED = "expired"
    }
}
