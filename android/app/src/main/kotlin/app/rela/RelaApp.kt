package app.rela

import android.app.Application
import app.rela.ui.CoverFetcher
import app.rela.ui.CoverKeyer
import coil3.ImageLoader
import coil3.PlatformContext
import coil3.SingletonImageLoader

class RelaApp : Application(), SingletonImageLoader.Factory {
    override fun newImageLoader(context: PlatformContext) = ImageLoader.Builder(context)
        .components {
            add(CoverFetcher.Factory())
            add(CoverKeyer())
        }
        .build()
}
