package app.rela

import android.annotation.SuppressLint
import android.app.Application
import androidx.annotation.PluralsRes
import androidx.annotation.StringRes
import app.rela.ui.CoverFetcher
import app.rela.ui.CoverKeyer
import coil3.ImageLoader
import coil3.PlatformContext
import coil3.SingletonImageLoader

class RelaApp : Application(), SingletonImageLoader.Factory {
    override fun onCreate() {
        super.onCreate()
        instance = this
    }

    override fun newImageLoader(context: PlatformContext) = ImageLoader.Builder(context)
        .components {
            add(CoverFetcher.Factory())
            add(CoverKeyer())
        }
        .build()

    companion object {
        /** For [str] and [plural]. The Application: it lives as long as the process. */
        @SuppressLint("StaticFieldLeak")
        lateinit var instance: RelaApp
            private set
    }
}

/**
 * A string resource in the app's language (values/ English, values-tr/ Turkish), from any code, Compose or not: a
 * language change recreates the activity, so screens read it again.
 */
fun str(@StringRes id: Int, vararg args: Any): String =
    if (args.isEmpty()) RelaApp.instance.getString(id) else RelaApp.instance.getString(id, *args)

/** "%d songs" for n. */
fun plural(@PluralsRes id: Int, n: Int): String = RelaApp.instance.resources.getQuantityString(id, n, n)
