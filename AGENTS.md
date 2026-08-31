# AGENTS.md

This file provides guidance to Claude Code (claude.ai/code) and Codex (Codex.ai/code) when working with code in this repository.

## Build and Development Commands

### Core Commands
- `./gradlew build` - Build the entire project including Rust native libraries and Java/Kotlin code
- `./gradlew test` - Run JUnit tests with native library dependencies
- `./gradlew lint` - Run ktlint for Kotlin and clippy/cargo fmt for Rust code
- `./gradlew autofix` - Auto-format code with ktlint and cargo fmt, apply clippy fixes

### Native Compilation
The project uses Rust for native implementations and requires platform-specific compilation:
- `./gradlew compileNative-<target>-<profile>` - Compile native code for specific platform/profile
- Profiles: `dev` (debug) and `release`
- Enable cross-compilation via gradle.properties flags like `enableCrossCompileToLinuxX86_64=true`

### Sample Applications
- `./gradlew :sample:runComposeSample` - Run the Compose sample on any host; it picks the backend that suits
  the system (Wayland or GTK on Linux, where `GDK_BACKEND` forces GTK). The IDE run configuration
  `Compose Sample` (`.run/`) is this task.
- `./gradlew :sample:runSkikoSampleMac` - Run macOS Skiko sample app
- `./gradlew :sample:runApplicationMenuSampleMac` - Run macOS Application Menu sample
- `./gradlew :sample:runSkikoSampleLinux` - Run Linux Wayland sample app
- `./gradlew :sample:runSkikoSampleWin32` - Run Windows sample app

### Packaging
- `./gradlew :sample:jpackage` - Create macOS .app bundle at `sample/build/dist/SkikoSample.app`
- `./gradlew :sample:runPackagedMac` - Package and launch the macOS .app bundle
- The packaged app includes native libraries, JRE, and proper Info.plist with bundle identifier

## Architecture Overview

### Multi-Platform Structure
This is a Kotlin/Rust hybrid project providing desktop window management APIs for macOS, Linux (Wayland), and Windows:

- **kotlin-desktop-toolkit/** - Main Kotlin API with platform-specific implementations in `org.jetbrains.desktop.{macos,linux,win32}`
- **kotlin-desktop-toolkit-compose/** - Compose Multiplatform facade over the toolkit (`org.jetbrains.desktop.compose`,
  with per-backend implementations in `.{macos,linux,gtk,windows}`). Publishes as `kotlin-desktop-toolkit-compose`;
  built against released Compose artifacts, so it is a library rather than a Compose fork. `application { }` and
  `Window { }` are the whole public entry point, and `DesktopBackend.default` picks the backend
- **kotlin-desktop-toolkit-compose-dispatcher/** - Opt-in artifact that makes `Dispatchers.Main` the toolkit's
  event loop thread, via a `kotlinx.coroutines` `MainDispatcherFactory` registered in `META-INF/services`.
  Separate from the facade because it changes a process-wide setting just by being on the classpath
- **native/** - Rust workspace with platform-specific crates:
  - `desktop-common/` - Shared Rust utilities
  - `desktop-macos/` - macOS native implementation
  - `desktop-linux/` - Linux/Wayland native implementation
  - `desktop-win32/` - Windows native implementation. Has a per-crate doc set at `native/desktop-win32/docs/` — start with `AGENTS.md`; covers architecture, FFI conventions, subsystem reference, and known issues. Treat that doc set as the canonical reference for the Win32 internals; the top-level AGENTS.md is a thin index by comparison.

### Platform Bindings
- Uses JExtract to generate Java bindings from Rust C headers
- Native libraries are compiled per platform and packaged into platform-specific JARs
- Each platform provides distinct APIs reflecting OS differences (non-goal to unify APIs)

### Key Components per Platform
- **Application**: Event loop management, screen listing, application lifecycle
- **Window**: Creation, positioning, sizing, rendering surface setup
- **Events**: Keyboard, mouse, window lifecycle events
- **Rendering**: Metal (macOS), OpenGL/Software (Linux), ANGLE (Windows)
- **System Integration**: Clipboard, drag-and-drop, appearance/theming, notifications (macOS)

### Cross-Compilation Settings
Platform builds are controlled via gradle.properties flags. Only enabled platforms are compiled during build.

## Testing
- Tests require native library compilation for the host platform
- Each test class runs in separate JVM with native access enabled
- Native debug logging can be enabled with `kdt.debug=true` system property
