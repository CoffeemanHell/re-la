package app.rela.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialExpressiveTheme
import androidx.compose.material3.MotionScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext


/** Material 3 Expressive with the wallpaper's colors. mode: "light", "dark", else system; [amoled]: black when dark. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun RelaTheme(mode: String, amoled: Boolean = false, content: @Composable () -> Unit) {
    val dark = when (mode) {
        "light" -> false
        "dark" -> true
        else -> isSystemInDarkTheme()
    }
    val context = LocalContext.current
    val scheme = if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
    MaterialExpressiveTheme(colorScheme = relaScheme(scheme, dark, amoled), motionScheme = MotionScheme.expressive(), content = content)
}
