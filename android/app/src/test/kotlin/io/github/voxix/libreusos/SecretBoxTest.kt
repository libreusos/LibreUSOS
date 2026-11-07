package io.github.voxix.libreusos

import io.github.voxix.libreusos.security.AesGcmBox
import io.github.voxix.libreusos.security.EncryptedFile
import io.github.voxix.libreusos.security.SecretBoxException
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.assertThrows
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import javax.crypto.KeyGenerator

class SecretBoxTest {
    @get:Rule val folder = TemporaryFolder()

    private fun newBox() = AesGcmBox {
        KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()
    }

    private val aad = "purpose".toByteArray()
    private val plain = "super secret".toByteArray()

    @Test fun roundTrip() {
        val box = newBox()
        assertArrayEquals(plain, box.open(box.seal(plain, aad), aad))
    }

    @Test fun sealIsRandomizedAndHidesPlaintext() {
        val box = newBox()
        val a = box.seal(plain, aad)
        val b = box.seal(plain, aad)
        assertNotEquals(a.toList(), b.toList())
        assertFalse(String(a, Charsets.ISO_8859_1).contains("super secret"))
    }

    @Test fun tamperedBlobIsRejected() {
        val box = newBox()
        val blob = box.seal(plain, aad)
        blob[blob.size - 1] = (blob[blob.size - 1].toInt() xor 1).toByte()
        assertThrows(SecretBoxException::class.java) { box.open(blob, aad) }
    }

    @Test fun wrongPurposeIsRejected() {
        val box = newBox()
        val blob = box.seal(plain, aad)
        assertThrows(SecretBoxException::class.java) { box.open(blob, "other".toByteArray()) }
    }

    @Test fun wrongKeyIsRejected() {
        val blob = newBox().seal(plain, aad)
        assertThrows(SecretBoxException::class.java) { newBox().open(blob, aad) }
    }

    @Test fun truncatedBlobIsRejected() {
        val box = newBox()
        assertThrows(SecretBoxException::class.java) { box.open(ByteArray(0), aad) }
        assertThrows(SecretBoxException::class.java) { box.open(byteArrayOf(12, 1, 2), aad) }
    }

    @Test fun encryptedFileRoundTrip() {
        val file = EncryptedFile(folder.root.resolve("a/b.bin"), newBox(), "p")
        assertNull(file.read())
        file.write(plain)
        assertArrayEquals(plain, file.read())
        file.write("again".toByteArray())
        assertArrayEquals("again".toByteArray(), file.read())
        file.delete()
        assertNull(file.read())
    }

    // --- delete policy: only permanent failures may drop a blob ---

    @Test fun malformedAndTamperedBlobsArePermanent() {
        val box = newBox()
        val e1 = assertThrows(SecretBoxException::class.java) { box.open(ByteArray(0), aad) }
        assertTrue(e1.permanent)
        val blob = box.seal(plain, aad)
        blob[blob.size - 1] = (blob[blob.size - 1].toInt() xor 1).toByte()
        val e2 = assertThrows(SecretBoxException::class.java) { box.open(blob, aad) }
        assertTrue(e2.permanent)
    }

    @Test fun providerFailureIsNotPermanent() {
        val blob = newBox().seal(plain, aad)
        val broken = AesGcmBox { throw java.security.ProviderException("keystore busy") }
        val e = assertThrows(SecretBoxException::class.java) { broken.open(blob, aad) }
        assertFalse(e.permanent)
    }

    @Test fun permanentFailureDeletesTheBlob() {
        val path = folder.root.resolve("p.bin")
        EncryptedFile(path, newBox(), "p").write(plain)
        // A different key can never read it: the blob is dropped and reported as absent.
        assertNull(EncryptedFile(path, newBox(), "p").load { it })
        assertFalse(path.exists())
    }

    @Test fun temporaryFailureKeepsTheBlob() {
        val path = folder.root.resolve("t.bin")
        EncryptedFile(path, newBox(), "p").write(plain)
        val broken = EncryptedFile(path, AesGcmBox { throw java.security.ProviderException("keystore busy") }, "p")
        assertThrows(SecretBoxException::class.java) { broken.load { it } }
        assertTrue(path.exists())
    }

    @Test fun undecodablePlaintextDeletesTheBlob() {
        val path = folder.root.resolve("d.bin")
        val file = EncryptedFile(path, newBox(), "p")
        file.write(plain)
        assertNull(file.load<Unit> { throw java.io.IOException("bad format") })
        assertFalse(path.exists())
    }

    // --- atomic write ---

    @Test fun writeLeavesNoTempFileAndReplacesAtomically() {
        val path = folder.root.resolve("w.bin")
        val file = EncryptedFile(path, newBox(), "p")
        file.write("one".toByteArray())
        file.write("two".toByteArray())
        assertArrayEquals("two".toByteArray(), file.read())
        assertFalse(folder.root.resolve("w.bin.tmp").exists())
    }

    @Test fun staleTempFileIsRemovedOnCreateAndRealFileSurvives() {
        val path = folder.root.resolve("s.bin")
        val box = newBox()
        EncryptedFile(path, box, "p").write(plain)
        val tmp = folder.root.resolve("s.bin.tmp")
        tmp.writeText("half-written garbage")
        val reopened = EncryptedFile(path, box, "p")
        assertFalse(tmp.exists())
        assertArrayEquals(plain, reopened.read())
    }
}
