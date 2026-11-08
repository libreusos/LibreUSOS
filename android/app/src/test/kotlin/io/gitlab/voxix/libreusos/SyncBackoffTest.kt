package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.sync.SyncRecord
import io.gitlab.voxix.libreusos.sync.afterFailure
import io.gitlab.voxix.libreusos.sync.afterSuccess
import io.gitlab.voxix.libreusos.sync.backoffDelayMs
import io.gitlab.voxix.libreusos.sync.minutesUntil
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class SyncBackoffTest {
    private val min = 60_000L
    private val hour = 60 * min

    @Test fun stepsAreFifteenMinutesOneHourSixHoursThenNone() {
        assertEquals(15 * min, backoffDelayMs(1))
        assertEquals(hour, backoffDelayMs(2))
        assertEquals(6 * hour, backoffDelayMs(3))
        assertNull(backoffDelayMs(4))
    }

    @Test fun failuresEscalateOnlyWhenNoRetryIsWaiting() {
        val first = SyncRecord().afterFailure(1_000)
        assertEquals(1_000 + 15 * min, first.nextAttemptAtMs)
        // A manual retry that fails while the planned one still waits changes nothing but the failure time.
        val manual = first.afterFailure(2_000)
        assertEquals(1, manual.failures)
        assertEquals(first.nextAttemptAtMs, manual.nextAttemptAtMs)
        assertEquals(2_000L, manual.lastFailureMs)
        // The planned retry itself failing moves to the one-hour step.
        val second = manual.afterFailure(first.nextAttemptAtMs!! + 5)
        assertEquals(2, second.failures)
        assertEquals(first.nextAttemptAtMs!! + 5 + hour, second.nextAttemptAtMs)
    }

    @Test fun afterTheLastStepTheNormalScheduleTakesOver() {
        var r = SyncRecord()
        var now = 0L
        repeat(4) { r = r.afterFailure(now); now += 7 * hour }
        assertNull(r.nextAttemptAtMs)
        assertEquals(4, r.failures)
    }

    @Test fun successResetsEverything() {
        val r = SyncRecord().afterFailure(10).afterSuccess(99)
        assertEquals(SyncRecord(lastSuccessMs = 99), r)
    }

    @Test fun minutesRoundUpAndNeverShowZero() {
        assertEquals(15, minutesUntil(15 * min, 0))
        assertEquals(15, minutesUntil(14 * min + 1, 0))
        assertEquals(1, minutesUntil(500, 0))
        assertEquals(1, minutesUntil(0, 5 * min))
    }
}
