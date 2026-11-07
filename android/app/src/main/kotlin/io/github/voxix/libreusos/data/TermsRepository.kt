package io.github.voxix.libreusos.data

import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.libreusos.core.DataSource
import org.libreusos.core.TermsResult

/**
 * The live terms, shared by the screens that need them (Grades, Tests), so a tab switch does not
 * refetch them. Single-flight: concurrent callers wait for the one fetch. The last live result is
 * kept in memory for one `today`; [clear] drops it (logout). An offline fallback is never kept.
 */
class TermsRepository {
    private val flight = Mutex()
    private val lock = Any()
    private var last: Pair<String, TermsResult>? = null

    // Bumped by clear(): a fetch that started before a logout must not store its result after it.
    private var generation = 0

    /** Runs [fetch] unless a live result for [today] is held; [force] skips the held one. Call from IO. */
    suspend fun terms(today: String, force: Boolean = false, fetch: suspend () -> TermsResult): TermsResult =
        flight.withLock {
            val startedAt = synchronized(lock) {
                last?.takeIf { !force && it.first == today }?.let { return@withLock it.second }
                generation
            }
            fetch().also { result ->
                synchronized(lock) {
                    if (result.source == DataSource.Live && generation == startedAt) last = today to result
                }
            }
        }

    /** Forgets the held terms (logout, expired session). */
    fun clear() = synchronized(lock) {
        generation++
        last = null
    }
}
