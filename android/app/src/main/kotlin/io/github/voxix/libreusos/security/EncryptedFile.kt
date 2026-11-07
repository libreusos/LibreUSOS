package io.github.voxix.libreusos.security

import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.nio.file.Files
import java.nio.file.StandardCopyOption

/**
 * One encrypted blob in one app-private file. Create exactly one instance per file and share it:
 * the instance monitor is the lock that serializes read, write and delete.
 *
 * Writes are atomic and durable: temp file, fsync, rename. A `.tmp` left by a crash is removed
 * when the instance is created.
 */
class EncryptedFile(private val file: File, private val box: SecretBox, purpose: String) {
    private val aad = purpose.toByteArray(Charsets.UTF_8)
    private val tmp = File(file.parentFile, file.name + ".tmp")

    init {
        tmp.delete()
    }

    /**
     * Reads and decodes the blob; null when there is none. A blob that can never be read (tampered,
     * malformed, key permanently gone, or [decode] throws IOException) is deleted and reported as
     * none. Any other failure is thrown and the blob is kept. Read and drop happen under one lock,
     * so a concurrent [write] is never deleted by mistake.
     */
    @Throws(SecretBoxException::class, IOException::class)
    @Synchronized
    fun <T> load(decode: (ByteArray) -> T): T? {
        if (!file.isFile) return null
        val plain = try {
            box.open(file.readBytes(), aad)
        } catch (e: SecretBoxException) {
            if (!e.permanent) throw e
            file.delete()
            return null
        }
        return try {
            decode(plain)
        } catch (_: IOException) {
            file.delete()
            null
        }
    }

    /** The plain bytes, or null when the file does not exist. Does not drop anything. */
    @Throws(SecretBoxException::class)
    @Synchronized
    fun read(): ByteArray? {
        if (!file.isFile) return null
        return box.open(file.readBytes(), aad)
    }

    @Throws(IOException::class)
    @Synchronized
    fun write(plain: ByteArray) {
        file.parentFile?.mkdirs()
        FileOutputStream(tmp).use {
            it.write(box.seal(plain, aad))
            it.fd.sync()
        }
        Files.move(tmp.toPath(), file.toPath(), StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
    }

    @Synchronized
    fun delete() {
        tmp.delete()
        file.delete()
    }
}
