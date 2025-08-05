package org.jetbrains.desktop.buildscripts

import org.gradle.api.attributes.Attribute

class KotlinDesktopToolkitAttributes {
    companion object {
        val TYPE: Attribute<KotlinDesktopToolkitArtifactType> = Attribute.of(
            "org.jetbrains.kotlin-desktop-toolkit.type",
            KotlinDesktopToolkitArtifactType::class.java,
        )
        val PROFILE: Attribute<KotlinDesktopToolkitNativeProfile> = Attribute.of(
            "org.jetbrains.kotlin-desktop-toolkit.native-profile",
            KotlinDesktopToolkitNativeProfile::class.java,
        )
    }
}

enum class KotlinDesktopToolkitArtifactType {
    NATIVE_LIBRARY,
}

enum class KotlinDesktopToolkitNativeProfile {
    DEBUG,
    RELEASE,
}
