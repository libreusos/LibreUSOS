package io.gitlab.voxix.libreusos.ui

import androidx.annotation.DrawableRes
import androidx.annotation.StringRes
import io.gitlab.voxix.libreusos.R

/** The four destinations of the bottom bar, in order. */
enum class MainTab(
    val route: String,
    @StringRes val label: Int,
    @DrawableRes val icon: Int,
    @DrawableRes val iconFilled: Int,
) {
    Today("today", R.string.nav_today, R.drawable.ic_today, R.drawable.ic_today_filled),
    Timetable("timetable", R.string.nav_timetable, R.drawable.ic_calendar_view_week, R.drawable.ic_calendar_view_week_filled),
    Grades("grades", R.string.nav_grades, R.drawable.ic_school, R.drawable.ic_school_filled),
    More("more", R.string.nav_more, R.drawable.ic_more_horiz, R.drawable.ic_more_horiz_filled),
    ;

    companion object {
        fun fromRoute(route: String?): MainTab? = entries.firstOrNull { it.route == route }

        /** The bottom bar shows on tab routes only; drill-down screens (settings, licences) cover it. */
        fun showsBottomBar(route: String?): Boolean = fromRoute(route) != null
    }
}

const val ROUTE_SETTINGS = "settings"
const val ROUTE_LICENCES = "licences"
const val ROUTE_CLASS = "class"
const val ROUTE_GRADE = "grade"
