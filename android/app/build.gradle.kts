plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "app.rela"
    compileSdk = 37
    compileSdkMinor = 2
    defaultConfig {
        applicationId = "app.rela"
        minSdk = 35
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        // the ABIs build-rust.sh makes librela.so for (a 32-bit phone can't install rather than crash)
        ndk { abiFilters += listOf("arm64-v8a") } // 64-bit ARM only; x86_64 emulator images run it through their ARM translation
    }
    // the release key comes from outside the repo: RELA_KEYSTORE (a PKCS12 file, key "rela") and RELA_KEYSTORE_PASSWORD
    val keystore = System.getenv("RELA_KEYSTORE")
    if (!keystore.isNullOrEmpty()) signingConfigs.create("release") {
        storeFile = file(keystore)
        storePassword = System.getenv("RELA_KEYSTORE_PASSWORD")
        keyAlias = "rela"
        keyPassword = storePassword
    }
    buildTypes {
        debug {
            applicationIdSuffix = ".debug"
        }
        // the one to use day to day: not debuggable, so Compose runs compiled (debug scrolls and starts several times slower)
        release {
            // without the release key: the local debug key, fine for your own phone only
            signingConfig = signingConfigs.findByName("release") ?: signingConfigs.getByName("debug")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }
    buildFeatures { compose = true }
    lint {
        warningsAsErrors = true
        // not ours: ARM only and no core-ktx by choice; the rest fail whenever a newer version of something comes out
        disable += setOf("ChromeOsAbiSupport", "UseKtx", "GradleDependency", "NewerVersionAvailable", "AndroidGradlePluginVersion", "OldTargetApi")
    }
}

dependencies {
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.profileinstaller:profileinstaller:1.4.1") // installs the libraries' baseline profiles: compiled hot paths from the first run
    implementation("androidx.compose.ui:ui:1.13.0")
    implementation("androidx.compose.foundation:foundation:1.13.0")
    implementation("androidx.compose.material3:material3:1.5.0")
    implementation("androidx.compose.animation:animation-graphics:1.13.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.11.1")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.11.1")
    implementation("androidx.lifecycle:lifecycle-viewmodel-navigation3:2.11.1") // a screen keeps its data while on the back stack
    implementation("androidx.navigation3:navigation3-runtime:1.2.2")
    implementation("androidx.navigation3:navigation3-ui:1.2.2")
    implementation("io.coil-kt.coil3:coil-compose:3.6.3")
    implementation("com.materialkolor:material-kolor:5.0.1") // MIT: a color scheme from a cover's color
    implementation("androidx.media3:media3-exoplayer:1.11.1")
    implementation("androidx.media3:media3-session:1.11.1")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.11.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-guava:1.11.0")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.13.0")
    // UniFFI's Kotlin bindings call into librela.so through JNA
    implementation("net.java.dev.jna:jna:5.19.1@aar")

    testImplementation("junit:junit:4.13.2")
    testImplementation("net.java.dev.jna:jna:5.19.1")
}
