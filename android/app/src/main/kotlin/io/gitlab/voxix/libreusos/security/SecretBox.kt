package io.gitlab.voxix.libreusos.security

import android.security.keystore.KeyPermanentlyInvalidatedException
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * A blob could not be decrypted. [permanent] is true when it never will be (tampered or
 * malformed blob, key permanently invalidated): the caller may then drop the blob. Any other
 * failure (Keystore trouble, provider errors) may be temporary, so the blob must be kept.
 */
class SecretBoxException(val permanent: Boolean, cause: Throwable? = null) :
    Exception("Cannot decrypt the stored blob (permanent=$permanent)", cause)

/** Authenticated encryption of small blobs. `aad` binds a blob to its purpose. */
interface SecretBox {
    fun seal(plain: ByteArray, aad: ByteArray): ByteArray

    @Throws(SecretBoxException::class)
    fun open(blob: ByteArray, aad: ByteArray): ByteArray
}

/**
 * AES-256-GCM. The key comes from [keyProvider] (the Android Keystore in the app, a plain
 * random key in unit tests). The IV is chosen by the cipher (the Keystore requires that) and
 * stored in front of the ciphertext: `[iv length: 1 byte][iv][ciphertext + tag]`.
 */
class AesGcmBox(keyProvider: () -> SecretKey) : SecretBox {
    private val key: SecretKey by lazy(keyProvider)

    override fun seal(plain: ByteArray, aad: ByteArray): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key)
        cipher.updateAAD(aad)
        val iv = cipher.iv
        val ciphertext = cipher.doFinal(plain)
        return byteArrayOf(iv.size.toByte()) + iv + ciphertext
    }

    override fun open(blob: ByteArray, aad: ByteArray): ByteArray {
        val ivLen = if (blob.isEmpty()) 0 else blob[0].toInt() and 0xFF
        if (ivLen == 0 || blob.size < 1 + ivLen + TAG_BITS / 8) throw SecretBoxException(permanent = true)
        try {
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(TAG_BITS, blob, 1, ivLen))
            cipher.updateAAD(aad)
            return cipher.doFinal(blob, 1 + ivLen, blob.size - 1 - ivLen)
        } catch (e: Exception) {
            // ProviderException (a RuntimeException from the Keystore) and IO trouble land here too.
            throw SecretBoxException(permanent = isPermanent(e), cause = e)
        }
    }

    private companion object {
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val TAG_BITS = 128

        // Only these two mean "never again"; everything else may be temporary.
        fun isPermanent(e: Exception): Boolean =
            e is AEADBadTagException || e is KeyPermanentlyInvalidatedException
    }
}
