plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.afstudy20.ytb"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.afstudy20.ytb"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
    }

    // Tauri builds per-ABI APKs plus a "universal" variant; without these
    // flavors `tauri android build` fails with "Task 'assembleUniversalRelease'
    // not found".
    flavorDimensions += "abi"
    productFlavors {
        create("armv7") {
            dimension = "abi"
            ndk { abiFilters += "armeabi-v7a" }
        }
        create("arm64") {
            dimension = "abi"
            ndk { abiFilters += "arm64-v8a" }
        }
        create("x86_64") {
            dimension = "abi"
            ndk { abiFilters += "x86_64" }
        }
        create("universal") {
            dimension = "abi"
        }
    }

    buildTypes {
        getByName("debug") {
            isMinifyEnabled = false
        }
        getByName("release") {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.media:media:1.7.0")
    implementation("androidx.media3:media3-exoplayer:1.4.1")
    implementation("androidx.media3:media3-session:1.4.1")
    implementation("androidx.media3:media3-ui:1.4.1")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
}
