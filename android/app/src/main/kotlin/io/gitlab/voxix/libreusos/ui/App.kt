package io.gitlab.voxix.libreusos.ui

import androidx.annotation.StringRes
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Tab as TabItem
import androidx.compose.material3.Text
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.NavDestination.Companion.hierarchy
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import io.gitlab.voxix.libreusos.AppContainer
import io.gitlab.voxix.libreusos.R

private const val ROUTE_LOGIN = "login"
private const val ROUTE_HOME = "main"

@Composable
fun App(authViewModel: AuthViewModel, container: AppContainer, requestedTab: String?, onTabHandled: () -> Unit) {
    LibreUsosTheme {
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
                onBaseUrlChange = authViewModel::onBaseUrlChange,
                onConsumerKeyChange = authViewModel::onConsumerKeyChange,
                onConsumerSecretChange = authViewModel::onConsumerSecretChange,
                onLogin = authViewModel::login,
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

private const val TAB_TODAY = "today"
const val TAB_TIMETABLE = "timetable"
private const val TAB_GRADES = "grades"

private class Tab(val route: String, val icon: ImageVector, @StringRes val label: Int)

private val tabs = listOf(
    Tab(TAB_TODAY, Icons.Filled.Home, R.string.nav_today),
    Tab(TAB_TIMETABLE, Icons.Filled.DateRange, R.string.nav_timetable),
    Tab(TAB_GRADES, Icons.Filled.Star, R.string.nav_grades),
)

/** The logged-in app: a bottom navigation bar over the Today, Timetable and Grades screens. */
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
    // Created here, so both live as long as the logged-in session and survive tab switches.
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
    Scaffold(
        bottomBar = {
            NavigationBar {
                val current = tabController.currentBackStackEntryAsState().value?.destination
                tabs.forEach { tab ->
                    NavigationBarItem(
                        selected = current?.hierarchy?.any { it.route == tab.route } == true,
                        onClick = { selectTab(tab.route) },
                        icon = { Icon(tab.icon, contentDescription = null) },
                        label = { Text(stringResource(tab.label)) },
                    )
                }
            }
        },
    ) { padding ->
        // The padding already covers the system bars; screens must not add them again.
        NavHost(
            tabController,
            startDestination = TAB_TODAY,
            modifier = Modifier.padding(padding).consumeWindowInsets(padding),
        ) {
            composable(TAB_TODAY) {
                val state by homeViewModel.state.collectAsStateWithLifecycle()
                val logoutError by authViewModel.homeError.collectAsStateWithLifecycle()
                HomeScreen(state, logoutError, onRefresh = homeViewModel::refresh, onLogout = authViewModel::logout)
            }
            composable(TAB_TIMETABLE) { TimetableScreen(timetableViewModel) }
            composable(TAB_GRADES) { GradesTab(gradesViewModel, testsViewModel) }
        }
    }
}

/** The Grades tab: a segmented top row switches between the grades and the course tests. */
@Composable
private fun GradesTab(gradesViewModel: GradesViewModel, testsViewModel: TestsViewModel) {
    var selected by rememberSaveable { mutableIntStateOf(0) }
    Column(Modifier.fillMaxSize()) {
        PrimaryTabRow(selectedTabIndex = selected) {
            TabItem(selected == 0, onClick = { selected = 0 }, text = { Text(stringResource(R.string.nav_grades)) })
            TabItem(selected == 1, onClick = { selected = 1 }, text = { Text(stringResource(R.string.grades_tab_tests)) })
        }
        if (selected == 0) GradesScreen(gradesViewModel, Modifier.weight(1f)) else TestsScreen(testsViewModel, Modifier.weight(1f))
    }
}
