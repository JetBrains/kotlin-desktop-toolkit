package org.jetbrains.desktop.buildscripts

import org.gradle.api.attributes.Attribute

class KotlinDesktopToolkitAttributes {
    companion object {
        val TYPE: Attribute<KotlingDesktopToolkitArtifactType> = Attribute.of("org.jetbrains.kotlin-desktop-toolkit.type", KotlingDesktopToolkitArtifactType::class.java)
        val PROFILE: Attribute<KotlingDesktopToolkitNativeProfile> = Attribute.of("org.jetbrains.kotlin-desktop-toolkit.native-profile", KotlingDesktopToolkitNativeProfile::class.java)
    }
}

enum class KotlingDesktopToolkitArtifactType {
    NATIVE_LIBRARY
}

enum class KotlingDesktopToolkitNativeProfile {
    DEBUG, RELEASE
}