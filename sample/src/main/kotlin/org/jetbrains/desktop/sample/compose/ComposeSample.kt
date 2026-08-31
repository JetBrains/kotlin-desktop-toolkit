package org.jetbrains.desktop.sample.compose

/**
 * The sample's entry point — one `main` for every platform.
 *
 * Everything that used to make this per-backend is behind [application]: it picks the backend the system
 * calls for, boots the toolkit, and tears it down again. What is left is the part that is actually the
 * sample's, [SampleWindows].
 *
 * Run it with `./gradlew :sample:runComposeSample` — on every platform, since there is nothing left for a
 * per-backend run configuration to select.
 */
fun main() = application {
    SampleWindows()
}
