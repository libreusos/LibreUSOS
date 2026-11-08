package io.gitlab.voxix.libreusos

import io.gitlab.voxix.libreusos.ui.ErrorKind
import io.gitlab.voxix.libreusos.ui.FinishStep
import io.gitlab.voxix.libreusos.ui.LOGIN_TIMEOUT_MS
import io.gitlab.voxix.libreusos.ui.LoginEvent
import io.gitlab.voxix.libreusos.ui.LoginFlowState
import io.gitlab.voxix.libreusos.ui.LoginStage
import io.gitlab.voxix.libreusos.ui.UrlProblem
import io.gitlab.voxix.libreusos.ui.knownUniversityAt
import io.gitlab.voxix.libreusos.ui.reduceLogin
import io.gitlab.voxix.libreusos.ui.validateBaseUrl
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LoginFlowTest {
    private val polsl = "https://usosapi.polsl.pl/"
    private val other = "https://usosapi.uni.example.pl/"

    private fun run(start: LoginFlowState, vararg events: LoginEvent) = events.fold(start) { s, e -> reduceLogin(s, e) }

    private fun loaded(baseUrl: String = polsl, knownHasKey: Boolean = true, needsKey: Boolean = false) =
        run(LoginFlowState(baseUrl = baseUrl), LoginEvent.Loaded(baseUrl, knownHasKey, needsKey))

    private fun waiting(now: Long = 1_000) =
        run(loaded(), LoginEvent.Submitted, LoginEvent.BrowserOpened("usosapi.polsl.pl", now))

    // --- first screen

    @Test
    fun startsOnTheListForTheKnownUniversityWithItsKey() {
        assertEquals(LoginStage.Universities, loaded().stage)
    }

    @Test
    fun aRedirectThatStartedTheProcessIsNotResetByTheLateFirstLook() {
        val state = run(LoginFlowState(), LoginEvent.RedirectReceived, LoginEvent.Loaded(polsl, true, false))
        assertEquals(LoginStage.Finishing, state.stage)
    }

    @Test
    fun startsOnTheFormWhenTheBuildHasNoKeyForTheKnownUniversity() {
        val state = loaded(knownHasKey = false, needsKey = true)
        assertEquals(LoginStage.OtherUniversity, state.stage)
        assertEquals(polsl, state.baseUrl)
    }

    @Test
    fun startsOnTheFormForAnUnknownSavedAddress() {
        val state = loaded(baseUrl = other, needsKey = true)
        assertEquals(LoginStage.OtherUniversity, state.stage)
        assertEquals(other, state.baseUrl)
    }

    @Test
    fun knownUniversityIsFoundByHostWhateverTheSpelling() {
        assertNotNull(knownUniversityAt("https://USOSAPI.polsl.pl"))
        assertNull(knownUniversityAt(other))
    }

    // --- the form

    @Test
    fun openingTheFormKeepsOrReplacesTheAddress() {
        val kept = run(loaded(), LoginEvent.OpenOther(null))
        assertEquals(LoginStage.OtherUniversity, kept.stage)
        assertEquals(polsl, kept.baseUrl)
        assertEquals(other, run(kept, LoginEvent.OpenOther(other)).baseUrl)
    }

    @Test
    fun backFromTheFormReturnsToTheList() {
        val state = run(loaded(), LoginEvent.OpenOther(null), LoginEvent.ChooseUniversity)
        assertEquals(LoginStage.Universities, state.stage)
    }

    @Test
    fun submitNeedsAValidAddressAndBothKeyParts() {
        var s = run(loaded(baseUrl = other, needsKey = true), LoginEvent.BaseUrlChanged(other))
        assertFalse(s.canSubmitOther)
        s = run(s, LoginEvent.KeyChanged("key"))
        assertFalse(s.canSubmitOther)
        s = run(s, LoginEvent.SecretChanged("secret"))
        assertTrue(s.canSubmitOther)
        s = run(s, LoginEvent.BaseUrlChanged("not an address"))
        assertFalse(s.canSubmitOther)
    }

    @Test
    fun submitWithoutKeyFieldsWhenTheBuildHasTheKey() {
        val s = run(loaded(), LoginEvent.OpenOther(null))
        assertTrue(run(s, LoginEvent.NeedsKeyResolved(polsl, false)).canSubmitOther)
    }

    @Test
    fun keyAvailabilityOfAStaleAddressIsIgnored() {
        val s = run(loaded(baseUrl = other, needsKey = true), LoginEvent.NeedsKeyResolved(polsl, false))
        assertTrue(s.needsKey)
    }

    @Test
    fun addressValidation() {
        assertNull(validateBaseUrl(""))
        assertNull(validateBaseUrl("https://usosapi.uni.example.pl"))
        assertEquals(UrlProblem.NotHttps, validateBaseUrl("http://usosapi.uni.example.pl"))
        assertEquals(UrlProblem.NotAnAddress, validateBaseUrl("usosapi.uni.example.pl"))
        assertEquals(UrlProblem.NotAnAddress, validateBaseUrl("hello world"))
        assertEquals(UrlProblem.NotAnAddress, validateBaseUrl("ftp://usosapi.uni.example.pl"))
    }

    @Test
    fun serverSaysNoUsosApiShowsTheAddressErrorUntilTheAddressChanges() {
        val form = run(loaded(baseUrl = other, needsKey = true), LoginEvent.Submitted)
        val failed = run(form, LoginEvent.BeginFailed(ErrorKind.API))
        assertEquals(LoginStage.OtherUniversity, failed.stage)
        assertFalse(failed.busy)
        assertEquals(UrlProblem.NoUsosApi, failed.urlProblem)
        assertNull(failed.error)
        assertNull(run(failed, LoginEvent.BaseUrlChanged(other + "x")).urlProblem)
    }

    @Test
    fun rejectedKeyIsFlaggedOnTheKeyUntilItChanges() {
        val form = run(loaded(baseUrl = other, needsKey = true), LoginEvent.Submitted)
        val failed = run(form, LoginEvent.BeginFailed(ErrorKind.UNAUTHORIZED))
        assertTrue(failed.keyRejected)
        assertFalse(run(failed, LoginEvent.KeyChanged("new")).keyRejected)
    }

    @Test
    fun networkFailureOnTheFormIsABannerNotAFieldError() {
        val failed = run(loaded(baseUrl = other, needsKey = true), LoginEvent.Submitted, LoginEvent.BeginFailed(ErrorKind.NETWORK))
        assertEquals(ErrorKind.NETWORK, failed.error)
        assertNull(failed.urlProblem)
        assertFalse(failed.keyRejected)
    }

    @Test
    fun failureOnTheListIsABanner() {
        val failed = run(loaded(), LoginEvent.Submitted, LoginEvent.BeginFailed(ErrorKind.NETWORK))
        assertEquals(LoginStage.Universities, failed.stage)
        assertEquals(ErrorKind.NETWORK, failed.error)
        assertFalse(failed.busy)
    }

    // --- the browser step

    @Test
    fun submitShowsBusyThenWaitingWithNothingTicked() {
        val busy = run(loaded(), LoginEvent.Submitted)
        assertTrue(busy.busy)
        assertEquals(LoginStage.Universities, busy.stage)
        val waiting = run(busy, LoginEvent.BrowserOpened("usosapi.polsl.pl", 5))
        assertEquals(LoginStage.Waiting, waiting.stage)
        assertFalse(waiting.busy)
        assertEquals("usosapi.polsl.pl", waiting.authHost)
        assertEquals(5, waiting.waitingSinceMs)
    }

    @Test
    fun backFromTheBrowserStepReturnsToWhereTheLoginBegan() {
        assertEquals(LoginStage.Universities, run(waiting(), LoginEvent.Abandoned).stage)
        val fromForm = run(loaded(knownHasKey = false, needsKey = true), LoginEvent.Submitted, LoginEvent.BrowserOpened("h", 1), LoginEvent.Abandoned)
        assertEquals(LoginStage.OtherUniversity, fromForm.stage)
    }

    @Test
    fun comingBackFromTheTabWithoutARedirectIsACancel() {
        val s = run(waiting(), LoginEvent.AppStopped, LoginEvent.AppResumed(2_000))
        assertEquals(LoginStage.Cancelled, s.stage)
    }

    @Test
    fun resumeWithoutLeavingIsNotACancel() {
        // For example a rotation or a permission dialog: the activity paused but never stopped.
        assertEquals(LoginStage.Waiting, run(waiting(), LoginEvent.AppResumed(2_000)).stage)
    }

    @Test
    fun theRedirectArrivesBeforeTheResume() {
        val s = run(waiting(), LoginEvent.AppStopped, LoginEvent.RedirectReceived, LoginEvent.AppResumed(2_000))
        assertEquals(LoginStage.Finishing, s.stage)
    }

    @Test
    fun theTimerEndsTheWaitAfterTenMinutes() {
        assertEquals(LoginStage.Waiting, run(waiting(1_000), LoginEvent.Tick(1_000 + LOGIN_TIMEOUT_MS - 1)).stage)
        assertEquals(LoginStage.TimedOut, run(waiting(1_000), LoginEvent.Tick(1_000 + LOGIN_TIMEOUT_MS)).stage)
    }

    @Test
    fun aLateResumeIsATimeoutNotACancel() {
        val s = run(waiting(1_000), LoginEvent.AppStopped, LoginEvent.AppResumed(1_000 + LOGIN_TIMEOUT_MS + 5))
        assertEquals(LoginStage.TimedOut, s.stage)
    }

    @Test
    fun timerOutsideTheWaitChangesNothing() {
        val done = run(waiting(), LoginEvent.RedirectReceived)
        assertEquals(done, run(done, LoginEvent.Tick(10 * LOGIN_TIMEOUT_MS)))
    }

    @Test
    fun noBrowserEndsTheWaitWithAnError() {
        val s = run(waiting(), LoginEvent.BrowserMissing)
        assertEquals(LoginStage.Failed, s.stage)
        assertEquals(ErrorKind.NO_BROWSER, s.error)
    }

    // --- the redirect

    @Test
    fun redirectMovesToStepThreeAndThenTheFirstSync() {
        val s = run(waiting(), LoginEvent.RedirectReceived)
        assertEquals(LoginStage.Finishing, s.stage)
        assertEquals(FinishStep.Exchanging, s.finishStep)
        val synced = run(s, LoginEvent.Exchanged(listOf("w")))
        assertEquals(LoginStage.Finishing, synced.stage)
        assertEquals(FinishStep.Syncing, synced.finishStep)
        assertEquals(listOf("w"), synced.warnings)
    }

    @Test
    fun redirectIsAcceptedAfterACancelAndAfterARestart() {
        val cancelled = run(waiting(), LoginEvent.AppStopped, LoginEvent.AppResumed(2))
        assertEquals(LoginStage.Finishing, run(cancelled, LoginEvent.RedirectReceived).stage)
        assertEquals(LoginStage.Finishing, run(loaded(), LoginEvent.RedirectReceived).stage)
    }

    @Test
    fun deniedRedirectShowsTheDeniedScreen() {
        val s = run(waiting(), LoginEvent.RedirectReceived, LoginEvent.ExchangeFailed(ErrorKind.UNAUTHORIZED))
        assertEquals(LoginStage.Denied, s.stage)
    }

    @Test
    fun otherExchangeFailuresShowTheFailedScreenWithTheirKind() {
        val s = run(waiting(), LoginEvent.RedirectReceived, LoginEvent.ExchangeFailed(ErrorKind.NETWORK))
        assertEquals(LoginStage.Failed, s.stage)
        assertEquals(ErrorKind.NETWORK, s.error)
    }

    // --- retries and the session

    @Test
    fun signInAgainFromAnOutcomeKeepsTheEntryScreen() {
        val cancelled = run(loaded(knownHasKey = false, needsKey = true), LoginEvent.Submitted, LoginEvent.BrowserOpened("h", 1), LoginEvent.AppStopped, LoginEvent.AppResumed(2))
        val retry = run(cancelled, LoginEvent.Submitted)
        assertTrue(retry.busy)
        assertEquals(LoginStage.Cancelled, retry.stage)
        assertEquals(LoginStage.OtherUniversity, retry.entry)
        assertEquals(LoginStage.Waiting, run(retry, LoginEvent.BrowserOpened("h", 3)).stage)
    }

    @Test
    fun aFailedRetryLandsOnTheFailedScreen() {
        val cancelled = run(waiting(), LoginEvent.AppStopped, LoginEvent.AppResumed(2), LoginEvent.Submitted)
        val failed = run(cancelled, LoginEvent.BeginFailed(ErrorKind.NETWORK))
        assertEquals(LoginStage.Failed, failed.stage)
        assertFalse(failed.busy)
    }

    @Test
    fun chooseAnotherUniversityClearsTheOutcome() {
        val s = run(waiting(), LoginEvent.RedirectReceived, LoginEvent.ExchangeFailed(ErrorKind.NETWORK), LoginEvent.ChooseUniversity)
        assertEquals(LoginStage.Universities, s.stage)
        assertNull(s.error)
    }

    @Test
    fun expiredSessionShowsABannerOnTheEntryScreen() {
        val s = run(loaded(), LoginEvent.SessionExpired)
        assertEquals(LoginStage.Universities, s.stage)
        assertTrue(s.sessionExpired)
        assertFalse(run(s, LoginEvent.Submitted).sessionExpired)
    }

    @Test
    fun logoutAfterALoginReturnsToTheEntryScreen() {
        val s = run(waiting(), LoginEvent.RedirectReceived, LoginEvent.Exchanged(emptyList()), LoginEvent.LoggedOut(listOf("w")))
        assertEquals(LoginStage.Universities, s.stage)
        assertEquals(listOf("w"), s.warnings)
        assertFalse(s.busy)
    }
}
