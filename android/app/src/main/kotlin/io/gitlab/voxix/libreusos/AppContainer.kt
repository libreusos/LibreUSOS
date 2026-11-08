package io.gitlab.voxix.libreusos

import android.content.Context
import io.gitlab.voxix.libreusos.data.AppSettings
import io.gitlab.voxix.libreusos.data.ConsumerKeyStore
import io.gitlab.voxix.libreusos.data.KeystoreTokenStore
import io.gitlab.voxix.libreusos.data.PendingLoginStore
import io.gitlab.voxix.libreusos.data.SecretCodec
import io.gitlab.voxix.libreusos.data.SyncStore
import io.gitlab.voxix.libreusos.data.TermsRepository
import io.gitlab.voxix.libreusos.security.AesGcmBox
import io.gitlab.voxix.libreusos.security.KeystoreKey
import org.libreusos.core.UsosClient
import org.libreusos.core.hasEmbeddedKey as coreHasEmbeddedKey

/** Hand-wired dependencies, created once by [LibreUsosApp]. */
class AppContainer(context: Context) {
    val appContext: Context = context.applicationContext

    val settings = AppSettings(appContext)

    /** When the data was last fetched, and the automatic retry after a failure. */
    val syncStore = SyncStore(appContext)

    private val box = AesGcmBox { KeystoreKey.getOrCreate(KEY_ALIAS) }
    private val secretsDir = java.io.File(appContext.filesDir, "secrets")
    val tokenStore = KeystoreTokenStore(secretsDir, box)
    val pendingLoginStore = PendingLoginStore(java.io.File(secretsDir, "pending-login.bin"), box)
    private val consumerKeys = ConsumerKeyStore(secretsDir, box)

    /** The live terms shared by the screens; cleared on logout. */
    val terms = TermsRepository()

    private val cachePath = java.io.File(appContext.filesDir, "cache/usos-cache.sqlite").absolutePath

    private var client: UsosClient? = null
    private var clientIdentity: ClientIdentity? = null

    /** What a cached client was built from; compared by value, never printed. */
    private data class ClientIdentity(val baseUrl: String, val key: String?, val secret: String?) {
        override fun toString() = "ClientIdentity"
    }

    /** True when this build embeds a consumer key for [baseUrl]. Call from `Dispatchers.IO`. */
    fun hasEmbeddedKey(baseUrl: String): Boolean = coreHasEmbeddedKey(baseUrl.trim())

    /**
     * The client for [baseUrl]. With an embedded key, no credentials are needed; otherwise
     * [credentials] (or, when null, the ones saved earlier) are used, and saved when given.
     * Returns null when a key is needed but none is known. Call from `Dispatchers.IO`.
     */
    @Synchronized
    fun client(baseUrl: String, credentials: SecretCodec.ConsumerCredentials? = null): UsosClient? {
        val url = baseUrl.trim()
        val embedded = hasEmbeddedKey(url)
        val creds = if (embedded) null else credentials ?: consumerKeys.load(url) ?: return null
        if (credentials != null && !embedded) consumerKeys.save(url, credentials)
        val identity = ClientIdentity(url, creds?.key, creds?.secret)
        client?.takeIf { clientIdentity == identity }?.let { return it }
        val created = if (creds == null) {
            UsosClient.withEmbeddedKey(url, cachePath, tokenStore)
        } else {
            UsosClient(url, creds.key, creds.secret, cachePath, tokenStore)
        }
        client = created
        clientIdentity = identity
        return created
    }

    private companion object {
        const val KEY_ALIAS = "libreusos_master_key"
    }
}
