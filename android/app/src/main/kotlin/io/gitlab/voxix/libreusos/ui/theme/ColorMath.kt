package io.gitlab.voxix.libreusos.ui.theme

import kotlin.math.cbrt
import kotlin.math.pow
import kotlin.math.sqrt

/** Below this CIE76 distance two kind colours are too alike to tell apart by tone (SYSTEM.md section 3). */
const val KIND_DELTA_E_MIN = 15.0

/** CIE76 colour distance between two opaque sRGB colours given as ARGB ints (alpha is ignored). */
fun deltaE76(argbA: Int, argbB: Int): Double {
    val a = toLab(argbA)
    val b = toLab(argbB)
    val dl = a[0] - b[0]
    val da = a[1] - b[1]
    val db = a[2] - b[2]
    return sqrt(dl * dl + da * da + db * db)
}

private fun toLab(argb: Int): DoubleArray {
    val r = linear((argb shr 16 and 0xFF) / 255.0)
    val g = linear((argb shr 8 and 0xFF) / 255.0)
    val b = linear((argb and 0xFF) / 255.0)
    // sRGB to XYZ, D65.
    val x = 0.4124564 * r + 0.3575761 * g + 0.1804375 * b
    val y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b
    val z = 0.0193339 * r + 0.1191920 * g + 0.9503041 * b
    val fx = f(x / 0.95047)
    val fy = f(y)
    val fz = f(z / 1.08883)
    return doubleArrayOf(116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz))
}

private fun linear(c: Double) = if (c <= 0.04045) c / 12.92 else ((c + 0.055) / 1.055).pow(2.4)

private fun f(t: Double) = if (t > 216.0 / 24389.0) cbrt(t) else (24389.0 / 27.0 * t + 16) / 116
