package io.gitlab.voxix.libreusos.data

import io.gitlab.voxix.libreusos.security.EncryptedFile
import io.gitlab.voxix.libreusos.security.SecretBox
import java.io.File

/**
 * Consumer key and secret that the user typed in, for installations that this build has no
 * embedded key for. Encrypted like the tokens. Unused when the build embeds a key.
 */
class ConsumerKeyStore(private val dir: File, private val box: SecretBox) {
    private val files = HashMap<String, EncryptedFile>()

    fun load(baseUrl: String): SecretCodec.ConsumerCredentials? =
        fileFor(baseUrl).load(SecretCodec::decodeCredentials)

    fun save(baseUrl: String, credentials: SecretCodec.ConsumerCredentials) {
        fileFor(baseUrl).write(SecretCodec.encodeCredentials(credentials))
    }

    @Synchronized
    private fun fileFor(baseUrl: String) = files.getOrPut(baseUrl) {
        EncryptedFile(File(dir, hashName(baseUrl) + ".consumer"), box, "consumer:$baseUrl")
    }
}
