package org.jetbrains.desktop.compose

/**
 * The operating system this process runs on.
 *
 * kotlin-desktop-toolkit deliberately does not unify its per-platform APIs, so every backend needs its
 * own [ComposeApplication] / [ComposeWindow] implementation. Which OS this is does not on its own settle
 * which backend to use — see [DesktopBackend].
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

/**
 * Which of the toolkit's windowing backends to talk to.
 *
 * Separate from [DesktopPlatform] because on Linux the two questions have different answers: the toolkit
 * ships both a Wayland backend and a GTK one there, and they are two independent APIs rather than two
 * configurations of one. Nothing about the running system decides between them, so the choice belongs to
 * the entry point — this enum is what [initApplication] switches on.
 */
enum class DesktopBackend {
    MacOs,

    /** The toolkit's Wayland backend. Requires a Wayland session. */
    Wayland,

    /** The toolkit's GTK backend, which also covers X11. */
    Gtk,

    Windows,
    ;

    companion object {
        /**
         * The backend to use on this system, and what [application] runs on unless its caller names one.
         *
         * Only Linux has a decision to make, and only half of it is forced: X11 leaves [Gtk] as the sole
         * option, while a Wayland session could run either, so preferring [Wayland] there is this layer's
         * policy rather than something the system dictates.
         *
         * `GDK_BACKEND` wins over both. It is a GTK variable that nothing outside GTK reads, so setting it
         * at all — to `x11` to force XWayland, or to `wayland` — says the caller is steering GTK, and
         * `GDK_BACKEND=x11` inside a Wayland session would otherwise be read straight past: such a session
         * still exports `WAYLAND_DISPLAY` to every client it launches.
         */
        val default: DesktopBackend by lazy {
            when (DesktopPlatform.current) {
                DesktopPlatform.MacOs -> MacOs
                DesktopPlatform.Windows -> Windows
                DesktopPlatform.Linux -> when {
                    System.getenv("GDK_BACKEND") != null -> Gtk
                    currentLinuxWindowSystem() == LinuxWindowSystem.Wayland -> Wayland
                    // X11, and anything we could not identify: GTK is the only backend that can serve X11,
                    // and on a session with no display at all its `gtk_init` fails more legibly than the
                    // Wayland backend's connect does.
                    else -> Gtk
                }
            }
        }
    }
}

/**
 * The window system a Linux session is running.
 *
 * Deliberately not the same question as which backend to use — see [DesktopBackend]. GTK runs on both of
 * these, so the answer only forces a backend when it is [X11].
 */
enum class LinuxWindowSystem {
    Wayland,
    X11,

    /** No display variable was set at all: a bare shell, a container, an ssh session without forwarding. */
    Unknown,
}

/**
 * Reads the session's window system out of the environment.
 *
 * `WAYLAND_DISPLAY` and `WAYLAND_SOCKET` are the strongest signals, because they are the two ways
 * `wl_display_connect` can reach a compositor: a socket name to open, or a file descriptor already opened
 * for us. A client launched by a nested compositor or a session manager often gets only the second.
 * `XDG_SESSION_TYPE` is what the login session declares, and `DISPLAY` is the last resort. Note that a
 * Wayland session exports `WAYLAND_DISPLAY` to X11 clients under XWayland too, which is why this cannot be
 * the whole story for choosing a backend.
 */
fun currentLinuxWindowSystem(): LinuxWindowSystem {
    val waylandDisplay = System.getenv("WAYLAND_DISPLAY")
    val waylandSocket = System.getenv("WAYLAND_SOCKET")
    val xdgSessionType = System.getenv("XDG_SESSION_TYPE")?.lowercase()
    val display = System.getenv("DISPLAY")
    return when {
        !waylandDisplay.isNullOrBlank() || !waylandSocket.isNullOrBlank() -> LinuxWindowSystem.Wayland
        xdgSessionType == "wayland" -> LinuxWindowSystem.Wayland
        xdgSessionType == "x11" -> LinuxWindowSystem.X11
        !display.isNullOrBlank() -> LinuxWindowSystem.X11
        else -> LinuxWindowSystem.Unknown
    }
}
