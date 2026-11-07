package io.github.voxix.libreusos

import io.github.voxix.libreusos.data.KeystoreTokenStore
import io.github.voxix.libreusos.data.PendingLoginStore
import io.github.voxix.libreusos.data.SecretCodec
import io.github.voxix.libreusos.security.AesGcmBox
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.libreusos.core.PendingLoginState
import org.libreusos.core.TokenRecord
import javax.crypto.KeyGenerator

class StoreTest {
    @get:Rule val folder = TemporaryFolder()

    private val key = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()
    private val box = AesGcmBox { key }

    @Test fun tokenCodecRoundTrip() {
        val back = SecretCodec.decodeToken(SecretCodec.encodeToken(TokenRecord("tok", "sec")))
        assertEquals("tok", back.token)
        assertEquals("sec", back.secret)
    }

    @Test fun pendingCodecRoundTrip() {
        val pending = SecretCodec.PendingLogin("https://x/", PendingLoginState("rt", "rs", listOf("a", "b")))
        val back = SecretCodec.decodePending(SecretCodec.encodePending(pending))
        assertEquals("https://x/", back.baseUrl)
        assertEquals("rt", back.state.requestToken)
        assertEquals("rs", back.state.requestSecret)
        assertEquals(listOf("a", "b"), back.state.scopes)
    }

    @Test fun tokenStoreSavesLoadsAndDeletesPerInstallation() {
        val store = KeystoreTokenStore(folder.root, box)
        assertNull(store.load("https://a/"))
        store.save("https://a/", TokenRecord("t1", "s1"))
        store.save("https://b/", TokenRecord("t2", "s2"))
        assertEquals("t1", store.load("https://a/")?.token)
        assertEquals("s2", store.load("https://b/")?.secret)
        store.delete("https://a/")
        assertNull(store.load("https://a/"))
        assertNotNull(store.load("https://b/"))
    }

    @Test fun tokenFileDoesNotContainThePlainToken() {
        KeystoreTokenStore(folder.root, box).save("https://a/", TokenRecord("visible-token", "visible-secret"))
        folder.root.listFiles().orEmpty().forEach {
            assertEquals(false, String(it.readBytes(), Charsets.ISO_8859_1).contains("visible"))
        }
    }

    @Test fun tokenOfAnotherKeyIsDroppedNotFatal() {
        KeystoreTokenStore(folder.root, box).save("https://a/", TokenRecord("t", "s"))
        val otherKey = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()
        val other = KeystoreTokenStore(folder.root, AesGcmBox { otherKey })
        assertNull(other.load("https://a/"))
    }

    @Test fun pendingLoginStoreSurvivesANewInstance() {
        val file = folder.root.resolve("pending.bin")
        PendingLoginStore(file, box).save("https://a/", PendingLoginState("rt", "rs", listOf("s")))
        val loaded = PendingLoginStore(file, box).load()
        assertEquals("rt", loaded?.state?.requestToken)
        PendingLoginStore(file, box).clear()
        assertNull(PendingLoginStore(file, box).load())
    }
}
