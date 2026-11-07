package io.gitlab.voxix.libreusos.data

import io.gitlab.voxix.libreusos.security.EncryptedFile
import io.gitlab.voxix.libreusos.security.SecretBox
import org.libreusos.core.PendingLoginState
import java.io.File

/**
 * The state of the login that waits for the browser, encrypted, so it survives process death.
 * Create one instance per file.
 */
class PendingLoginStore(file: File, box: SecretBox) {
    private val store = EncryptedFile(file, box, "pending-login")

    fun save(baseUrl: String, state: PendingLoginState) {
        store.write(SecretCodec.encodePending(SecretCodec.PendingLogin(baseUrl, state)))
    }

    /** The saved login, or null when there is none (or it can never be read, then it is dropped). */
    fun load(): SecretCodec.PendingLogin? = store.load(SecretCodec::decodePending)

    fun clear() = store.delete()
}
