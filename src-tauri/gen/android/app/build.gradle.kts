import groovy.json.JsonSlurper
import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

// release 签名（官方口径）：CI 由 Secrets 注入 keystore.properties
// （keyAlias / password / storePassword / storeFile），文件本身不入库。
val keystoreProperties = Properties().apply {
    val propFile = rootProject.file("keystore.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

// rustls-platform-verifier 的 Kotlin 组件（ADR-008）：gix/reqwest 在 Android 的
// rustls 证书验证经 JNI 调 org.rustls.platformverifier.CertificateVerifier。
// AAR 随 rustls-platform-verifier-android crate（Cargo.lock 锁 0.1.1）分发，
// 用 cargo metadata 定位其 maven 目录（crate 官方 README 集成方式）。
repositories {
    maven {
        url = uri(rustlsPlatformVerifierMaven())
    }
}

fun rustlsPlatformVerifierMaven(): File {
    val metadataText = providers.exec {
        workingDir = File(project.rootDir, "../../")
        commandLine(
            "cargo", "metadata", "--format-version", "1",
            "--filter-platform", "aarch64-linux-android"
        )
    }.standardOutput.asText.get()
    val metadata = JsonSlurper().parseText(metadataText) as Map<*, *>
    val manifestPath = (metadata["packages"] as List<*>)
        .first { (it as Map<*, *>)["name"] == "rustls-platform-verifier-android" }
        .let { (it as Map<*, *>)["manifest_path"] as String }
    return File(manifestPath).parentFile.resolve("maven")
}

android {
    compileSdk = 36
    namespace = "com.microstep.app"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "com.microstep.app"
        minSdk = 24
        targetSdk = 36
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
    }
    signingConfigs {
        create("release") {
            storeFile = (keystoreProperties["storeFile"] as String?)?.let { file(it) }
            storePassword = keystoreProperties["storePassword"] as String?
                ?: keystoreProperties["password"] as String?
            keyAlias = keystoreProperties["keyAlias"] as String?
            keyPassword = keystoreProperties["password"] as String?
                ?: keystoreProperties["storePassword"] as String?
        }
    }
    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }
        getByName("release") {
            signingConfig = signingConfigs.getByName("release")
            isMinifyEnabled = true
            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList().toTypedArray()
            )
        }
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    buildFeatures {
        buildConfig = true
    }
}

rust {
    rootDirRel = "../../../"
}

dependencies {
    implementation("rustls:rustls-platform-verifier:0.1.1")
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = "tauri.build.gradle.kts")
