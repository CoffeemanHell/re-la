pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
        // rustls-platform-verifier's Kotlin half is only here (its maven-archive branch, pinned to a commit)
        maven("https://raw.githubusercontent.com/rustls/rustls-platform-verifier/1aa691352a5e0c215210dfc9a3c2f2078639dc91/android-release-support/maven/") {
            content { includeGroup("org.rustls") }
        }
    }
}
rootProject.name = "re-la"
include(":app")
