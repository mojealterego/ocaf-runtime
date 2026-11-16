plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "com.ocaf.builder"
    compileSdk = 35
    defaultConfig {
        applicationId = "com.ocaf.builder"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
    }
    buildFeatures { compose = true }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }
    sourceSets.named("main") {
        assets.srcDir(layout.buildDirectory.dir("generated/ocaf-assets"))
    }
}
val copySharedBuilderUi by tasks.registering(Sync::class) {
    from("../../agents-builder-web") {
        include("*.html", "*.css", "*.js", "*.mjs", "*.webmanifest")
    }
    into(layout.buildDirectory.dir("generated/ocaf-assets/www"))
}
tasks.named("preBuild") { dependsOn(copySharedBuilderUi) }

dependencies {
    implementation(platform("androidx.compose:compose-bom:2024.12.01"))
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.webkit:webkit:1.12.1")
}
