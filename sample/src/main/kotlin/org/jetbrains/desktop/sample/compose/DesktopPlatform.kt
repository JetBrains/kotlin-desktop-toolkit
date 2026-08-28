package org.jetbrains.desktop.sample.compose

/**
 * The windowing backend this process runs against.
 *
 * kotlin-desktop-toolkit deliberately does not unify its per-platform APIs, so every platform needs its
 * own [ComposeApplication] / [ComposeWindow] implementation. This enum is the single place where the
 * choice is made — see [initApplication].
 */
enum class DesktopPlatform {
    MacOs,
    Linux,
    Windows,
    ;

    companion object {
        val current: DesktopPlatform by lazy {
            val name = System.getProperty("os.name")
            when {
                name.startsWith("Mac") || name.startsWith("Darwin") -> MacOs
                name.startsWith("Win") -> Windows
                name == "Linux" -> Linux
                else -> error("Unsupported desktop platform: $name")
            }
        }
    }
}
