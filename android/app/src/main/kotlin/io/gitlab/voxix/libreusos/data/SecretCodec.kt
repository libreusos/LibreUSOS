package io.gitlab.voxix.libreusos.data

import org.libreusos.core.PendingLoginState
import org.libreusos.core.TokenRecord
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.DataInputStream
import java.io.DataOutputStream
import java.io.IOException

/**
 * Binary encodings of the secrets that are stored encrypted. Pure JVM, no logging, and the
 * types are never printed.
 */
object SecretCodec {
    private const val VERSION = 1

    /** The state of a login that waits for the browser, plus the installation it belongs to. */
    class PendingLogin(val baseUrl: String, val state: PendingLoginState)

    class ConsumerCredentials(val key: String, val secret: String)

    fun encodeToken(token: TokenRecord): ByteArray = write {
        writeUTF(token.token)
        writeUTF(token.secret)
    }

    fun decodeToken(bytes: ByteArray): TokenRecord = read(bytes) {
        TokenRecord(token = readUTF(), secret = readUTF())
    }

    fun encodePending(pending: PendingLogin): ByteArray = write {
        writeUTF(pending.baseUrl)
        writeUTF(pending.state.requestToken)
        writeUTF(pending.state.requestSecret)
        writeInt(pending.state.scopes.size)
        pending.state.scopes.forEach { writeUTF(it) }
    }

    fun decodePending(bytes: ByteArray): PendingLogin = read(bytes) {
        val baseUrl = readUTF()
        val requestToken = readUTF()
        val requestSecret = readUTF()
        val count = readInt()
        if (count < 0 || count > 1024) throw IOException("Bad scope count")
        val scopes = List(count) { readUTF() }
        PendingLogin(baseUrl, PendingLoginState(requestToken, requestSecret, scopes))
    }

    fun encodeCredentials(credentials: ConsumerCredentials): ByteArray = write {
        writeUTF(credentials.key)
        writeUTF(credentials.secret)
    }

    fun decodeCredentials(bytes: ByteArray): ConsumerCredentials = read(bytes) {
        ConsumerCredentials(key = readUTF(), secret = readUTF())
    }

    private fun write(body: DataOutputStream.() -> Unit): ByteArray {
        val out = ByteArrayOutputStream()
        DataOutputStream(out).use {
            it.writeByte(VERSION)
            it.body()
        }
        return out.toByteArray()
    }

    private fun <T> read(bytes: ByteArray, body: DataInputStream.() -> T): T =
        DataInputStream(ByteArrayInputStream(bytes)).use {
            val version = it.readUnsignedByte()
            if (version != VERSION) throw IOException("Unsupported version $version")
            it.body()
        }
}
