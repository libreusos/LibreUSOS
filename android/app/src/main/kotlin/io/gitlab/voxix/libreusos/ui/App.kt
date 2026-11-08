package io.gitlab.voxix.libreusos.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.TextButton
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import io.gitlab.voxix.libreusos.AppContainer
import io.gitlab.voxix.libreusos.R
import org.libreusos.core.Activity
import io.gitlab.voxix.libreusos.ui.theme.LibreUsosTheme

private const val ROUTE_LOGIN = "login"
private const val ROUTE_HOME = "main"

@Composable
fun App(authViewModel: AuthViewModel, container: AppContainer, requestedTab: String?, onTabHandled: () -> Unit) {
    val appearance by container.settings.appearance.collectAsStateWithLifecycle()
    LibreUsosTheme(appearance.themeMode, appearance.dynamicColor) {
        Surface(Modifier.fillMaxSize()) {
            val auth by authViewModel.auth.collectAsStateWithLifecycle()
            if (auth == AuthState.Loading) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { CircularProgressIndicator() }
            } else {
                AppNavigation(authViewModel, container, loggedIn = auth == AuthState.LoggedIn, requestedTab, onTabHandled)
            }
        }
    }
}

@Composable
private fun AppNavigation(
    authViewModel: AuthViewModel,
    container: AppContainer,
    loggedIn: Boolean,
    requestedTab: String?,
    onTabHandled: () -> Unit,
) {
    val navController = rememberNavController()
    val target = if (loggedIn) ROUTE_HOME else ROUTE_LOGIN
    NavHost(navController, startDestination = target) {
        composable(ROUTE_LOGIN) {
            val state by authViewModel.login.collectAsStateWithLifecycle()
            LoginScreen(
                state = state,
                callbacks = LoginCallbacks(
                    onBaseUrlChange = authViewModel::onBaseUrlChange,
                    onConsumerKeyChange = authViewModel::onConsumerKeyChange,
                    onConsumerSecretChange = authViewModel::onConsumerSecretChange,
                    onLoginKnown = authViewModel::loginKnown,
                    onLogin = authViewModel::login,
                    onOpenOther = authViewModel::openOtherUniversity,
                    onChooseUniversity = authViewModel::chooseUniversity,
                    onAbandon = authViewModel::abandon,
                    onReopenPage = authViewModel::reopenPage,
                ),
            )
        }
        composable(ROUTE_HOME) { MainShell(authViewModel, container, requestedTab, onTabHandled) }
    }
    // Follow the login state (login finished, logout) without keeping the old screen on the back stack.
    LaunchedEffect(target) {
        if (navController.currentDestination?.route != target) {
            navController.navigate(target) {
                popUpTo(navController.graph.id) { inclusive = true }
                launchSingleTop = true
            }
        }
    }
}

/** Route the widget asks for (see MainActivity). */
const val TAB_TIMETABLE = "timetable"

/**
 * The logged-in app: one NavHost with the four tabs and the drill-down screens. The bottom bar shows on
 * the tabs only.
 */
@Composable
private fun MainShell(
    authViewModel: AuthViewModel,
    container: AppContainer,
    requestedTab: String?,
    onTabHandled: () -> Unit,
) {
    val tabController = rememberNavController()
    fun selectTab(route: String) {
        tabController.navigate(route) {
            popUpTo(tabController.graph.findStartDestination().id) { saveState = true }
            launchSingleTop = true
            restoreState = true
        }
    }
    LaunchedEffect(requestedTab) {
        if (requestedTab == TAB_TIMETABLE) selectTab(TAB_TIMETABLE)
        if (requestedTab != null) onTabHandled()
    }
    // Created here, so they live as long as the logged-in session and survive tab switches.
    val homeViewModel: HomeViewModel = viewModel(
        factory = viewModelFactory { initializer { HomeViewModel(container, authViewModel::onSessionExpired) } },
    )
    val timetableViewModel: TimetableViewModel = viewModel(
        factory = viewModelFactory { initializer { TimetableViewModel(container, authViewModel::onSessionExpired) } },
    )
    val gradesViewModel: GradesViewModel = viewModel(
        factory = viewModelFactory { initializer { GradesViewModel(container, authViewModel::onSessionExpired) } },
    )
    val testsViewModel: TestsViewModel = viewModel(
        factory = viewModelFactory { initializer { TestsViewModel(container, authViewModel::onSessionExpired) } },
    )
    val accountViewModel: AccountViewModel = viewModel(
        factory = viewModelFactory { initializer { AccountViewModel(container) } },
    )
    val settingsViewModel: SettingsViewModel = viewModel(
        factory = viewModelFactory { initializer { SettingsViewModel(container) } },
    )
    val account by accountViewModel.state.collectAsStateWithLifecycle()
    val logoutError by authViewModel.homeError.collectAsStateWithLifecycle()

    var accountSheet by rememberSaveable { mutableStateOf(false) }
    var pendingAction by rememberSaveable { mutableStateOf<AccountAction?>(null) }

    val openClass = { activity: Activity ->
        timetableViewModel.showClass(activity)
        tabController.navigate(ROUTE_CLASS)
    }
    val route = tabController.currentBackStackEntryAsState().value?.destination?.route
    Scaffold(
        bottomBar = { if (MainTab.showsBottomBar(route)) MainNavigationBar(route, ::selectTab) },
    ) { padding ->
        // The padding already covers the system bars; screens must not add them again.
        NavHost(
            tabController,
            startDestination = MainTab.Today.route,
            modifier = Modifier.padding(padding).consumeWindowInsets(padding),
        ) {
            composable(MainTab.Today.route) {
                val sessionExpired by authViewModel.sessionExpired.collectAsStateWithLifecycle()
                HomeScreen(
                    homeViewModel,
                    sessionExpired,
                    onRelogin = authViewModel::relogin,
                    onOpenTimetable = { selectTab(MainTab.Timetable.route) },
                    onOpenClass = openClass,
                )
            }
            composable(MainTab.Timetable.route) {
                val sessionExpired by authViewModel.sessionExpired.collectAsStateWithLifecycle()
                TimetableScreen(
                    timetableViewModel,
                    sessionExpired,
                    onRelogin = authViewModel::relogin,
                    onOpenClass = openClass,
                )
            }
            composable(ROUTE_CLASS) { ClassDetailScreen(timetableViewModel, onBack = { tabController.popBackStack() }) }
            composable(MainTab.Grades.route) {
                val sessionExpired by authViewModel.sessionExpired.collectAsStateWithLifecycle()
                GradesTab(
                    gradesViewModel,
                    testsViewModel,
                    sessionExpired,
                    onRelogin = authViewModel::relogin,
                    onOpenCourse = { termId, course ->
                        gradesViewModel.showCourse(termId, course)
                        tabController.navigate(ROUTE_GRADE)
                    },
                )
            }
            composable(ROUTE_GRADE) { GradeDetailScreen(gradesViewModel, onBack = { tabController.popBackStack() }) }
            composable(MainTab.More.route) {
                MoreScreen(account, onOpenAccount = { accountSheet = true }, onOpenSettings = { tabController.navigate(ROUTE_SETTINGS) })
            }
            composable(ROUTE_SETTINGS) {
                SettingsScreen(
                    settingsViewModel,
                    account,
                    onBack = { tabController.popBackStack() },
                    onOpenLicences = { tabController.navigate(ROUTE_LICENCES) },
                    onAccountAction = { pendingAction = it },
                )
            }
            composable(ROUTE_LICENCES) { LicencesScreen(onBack = { tabController.popBackStack() }) }
        }
    }

    if (accountSheet) {
        AccountSheet(
            account,
            onAction = {
                accountSheet = false
                pendingAction = it
            },
            onDismiss = { accountSheet = false },
        )
    }
    logoutError?.let { error ->
        AlertDialog(
            onDismissRequest = authViewModel::dismissLogoutError,
            title = { Text(stringResource(R.string.logout_failed_title)) },
            text = { Text(stringResource(error.messageRes)) },
            confirmButton = { TextButton(authViewModel::dismissLogoutError) { Text(stringResource(R.string.action_ok)) } },
        )
    }
    pendingAction?.let { action ->
        ConfirmAccountActionDialog(
            action,
            onConfirm = {
                pendingAction = null
                // Both actions end at the login screen, which keeps the current server preselected.
                authViewModel.logout()
            },
            onDismiss = { pendingAction = null },
        )
    }
}

@Composable
private fun MainNavigationBar(route: String?, onSelect: (String) -> Unit) {
    NavigationBar {
        MainTab.entries.forEach { tab ->
            val selected = tab.route == route
            NavigationBarItem(
                selected = selected,
                onClick = { onSelect(tab.route) },
                icon = { Icon(painterResource(if (selected) tab.iconFilled else tab.icon), contentDescription = null) },
                label = {
                    Text(
                        stringResource(tab.label),
                        style = MaterialTheme.typography.labelMedium.copy(fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Medium),
                    )
                },
            )
        }
    }
}
