package io.gitlab.voxix.libreusos.data

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities

/** True when the device has a network that claims to reach the internet. Tells "offline" from "the server is down". */
fun isOnline(context: Context): Boolean {
    val manager = context.getSystemService(ConnectivityManager::class.java) ?: return true
    val caps = manager.getNetworkCapabilities(manager.activeNetwork ?: return false) ?: return false
    return caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
}
