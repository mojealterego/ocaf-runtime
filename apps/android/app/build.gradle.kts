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
        java.srcDir(layout.buildDirectory.dir("generated/uniffi-kotlin"))
        jniLibs.srcDir(layout.buildDirectory.dir("generated/uniffi-jni"))
    }
}
val copySharedBuilderUi by tasks.registering(Sync::class) {
    from("../../agents-builder-web") {
        include("*.html", "*.css", "*.js", "*.mjs", "*.webmanifest")
    }
    into(layout.buildDirectory.dir("generated/ocaf-assets/www"))
}
tasks.named("preBuild") { dependsOn(copySharedBuilderUi) }

// The native binding is generated from the SAME Rust source used by the runtime.
// Compilation fails closed if Cargo, NDK, cargo-ndk or binding generation fails.
val ffiManifest = file("../../../crates/ocaf-builder-mobile-ffi/Cargo.toml")
val ffiTarget = layout.buildDirectory.dir("ocaf-ffi-target").get().asFile
val kotlinBindings = layout.buildDirectory.dir("generated/uniffi-kotlin").get().asFile
val nativeLibraries = layout.buildDirectory.dir("generated/uniffi-jni").get().asFile

val buildHostFfi by tasks.registering(Exec::class) {
    environment("CARGO_TARGET_DIR", ffiTarget.absolutePath)
    commandLine("cargo", "build", "--release", "--manifest-path", ffiManifest.absolutePath)
}
val generateKotlinFfi by tasks.registering(Exec::class) {
    dependsOn(buildHostFfi)
    environment("CARGO_TARGET_DIR", ffiTarget.absolutePath)
    commandLine(
        "cargo", "run", "--release", "--features", "bindgen", "--bin", "uniffi-bindgen",
        "--manifest-path", ffiManifest.absolutePath, "--", "generate", "--library",
        ffiTarget.resolve("release").resolve(System.mapLibraryName("ocaf_builder_mobile_ffi")).absolutePath,
        "--language", "kotlin", "--out-dir", kotlinBindings.absolutePath
    )
}
val buildAndroidFfi by tasks.registering(Exec::class) {
    environment("CARGO_TARGET_DIR", ffiTarget.absolutePath)
    commandLine(
        "cargo", "ndk", "-t", "arm64-v8a", "-o", nativeLibraries.absolutePath,
        "build", "--release", "--manifest-path", ffiManifest.absolutePath
    )
}
tasks.named("preBuild") { dependsOn(generateKotlinFfi, buildAndroidFfi) }

dependencies {
    implementation(platform("androidx.compose:compose-bom:2024.12.01"))
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.webkit:webkit:1.12.1")
    implementation("net.java.dev.jna:jna:5.14.0@aar")
}
