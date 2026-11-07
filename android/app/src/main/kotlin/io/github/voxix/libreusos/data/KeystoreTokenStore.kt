package io.github.voxix.libreusos.data

import io.github.voxix.libreusos.security.EncryptedFile
import io.github.voxix.libreusos.security.SecretBox
import org.libreusos.core.TokenRecord
import org.libreusos.core.TokenStore
import java.io.File
import java.security.MessageDigest

/**
 * The core's [TokenStore]: one encrypted file per installation in [dir]. The AES-256-GCM key
 * lives in the Android Keystore (see `KeystoreKey`); the blob is in app-private storage.
 *
 * A blob that can never be decrypted is dropped (the user is logged out). A temporary failure is
 * thrown to the core, which reports it as a store error, and the blob stays. Every file has one
 * shared [EncryptedFile], and all operations hold this store's monitor, so a `save` cannot
 * interleave with a `delete`/`deleteAll` and bring a deleted token back.
 */
class KeystoreTokenStore(private val dir: File, private val box: SecretBox) : TokenStore {
    private val files = HashMap<String, EncryptedFile>()

    @Synchronized
    override fun load(baseUrl: String): TokenRecord? = fileFor(baseUrl).load(SecretCodec::decodeToken)

    @Synchronized
    override fun save(baseUrl: String, token: TokenRecord) {
        fileFor(baseUrl).write(SecretCodec.encodeToken(token))
    }

    @Synchronized
    override fun delete(baseUrl: String) {
        fileFor(baseUrl).delete()
    }

    /** Deletes the tokens of all installations (logout when no client could be built). */
    @Synchronized
    fun deleteAll() {
        files.values.forEach { it.delete() }
        dir.listFiles { f -> f.name.endsWith(".token") || f.name.endsWith(".token.tmp") }?.forEach { it.delete() }
    }

    private fun fileFor(baseUrl: String) = files.getOrPut(baseUrl) {
        EncryptedFile(File(dir, hashName(baseUrl) + ".token"), box, "token:$baseUrl")
    }
}

internal fun hashName(value: String): String =
    MessageDigest.getInstance("SHA-256").digest(value.toByteArray(Charsets.UTF_8))
        .joinToString("") { "%02x".format(it) }
