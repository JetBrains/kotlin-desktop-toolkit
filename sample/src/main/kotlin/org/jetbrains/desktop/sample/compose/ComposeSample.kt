package org.jetbrains.desktop.sample.compose

import org.jetbrains.desktop.compose.ComposeApplicationParams
import org.jetbrains.desktop.compose.application

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
fun main() = application(
    // The desktop matches windows against a `.desktop` file by this name, so it is the application's to
    // give rather than the library's; on Wayland it is the app id.
    ComposeApplicationParams(identifier = "org.jetbrains.desktop.sample.compose"),
) {
    SampleWindows()
}
