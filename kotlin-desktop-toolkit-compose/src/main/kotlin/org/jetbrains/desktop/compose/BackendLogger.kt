package org.jetbrains.desktop.compose

import java.nio.file.Files
import java.nio.file.Path
import org.jetbrains.desktop.gtk.KotlinDesktopToolkit as GtkToolkit
import org.jetbrains.desktop.gtk.LogLevel as GtkLogLevel
import org.jetbrains.desktop.gtk.Logger as GtkLogger
import org.jetbrains.desktop.linux.KotlinDesktopToolkit as LinuxToolkit
import org.jetbrains.desktop.linux.LogLevel as LinuxLogLevel
import org.jetbrains.desktop.linux.Logger as LinuxLogger
import org.jetbrains.desktop.macos.KotlinDesktopToolkit as MacOsToolkit
import org.jetbrains.desktop.macos.LogLevel as MacOsLogLevel
import org.jetbrains.desktop.macos.Logger as MacOsLogger
import org.jetbrains.desktop.win32.KotlinDesktopToolkit as WindowsToolkit
import org.jetbrains.desktop.win32.LogLevel as WindowsLogLevel
import org.jetbrains.desktop.win32.Logger as WindowsLogger

/**
 * Logging that does not require the caller to know which backend it is talking to.
 *
 * The toolkit deliberately does not unify its per-platform APIs, so even `Logger` is per-backend and there
 * are four unrelated objects with the same shape. Callers above this layer have a [DesktopBackend] rather
 * than a platform package, so they need this to be able to log at all.
 *
 * Only the branch that is actually taken ever gets loaded, which is what keeps a reference to all four
 * platforms' classes harmless: each `Logger` pulls in its own native library on first touch.
 */
internal fun logInfo(backend: DesktopBackend, message: () -> String) {
    when (backend) {
        DesktopBackend.MacOs -> MacOsLogger.info(message)
        DesktopBackend.Wayland -> LinuxLogger.info(message)
        DesktopBackend.Gtk -> GtkLogger.info(message)
        DesktopBackend.Windows -> WindowsLogger.info(message)
    }
}

/**
 * The backend [initToolkit] booted, for shared code that has to log without being handed one.
 *
 * Null until the toolkit is up, which is also the window in which there is no configured appender to log
 * through, so a message raised that early has nowhere useful to go anyway.
 */
@Volatile
private var bootedBackend: DesktopBackend? = null

/** Logs through whichever backend is up. A no-op before [initToolkit], deliberately — see [bootedBackend]. */
internal fun logError(throwable: Throwable, message: () -> String) {
    when (bootedBackend) {
        DesktopBackend.MacOs -> MacOsLogger.error(throwable, message)
        DesktopBackend.Wayland -> LinuxLogger.error(throwable, message)
        DesktopBackend.Gtk -> GtkLogger.error(throwable, message)
        DesktopBackend.Windows -> WindowsLogger.error(throwable, message)
        null -> Unit
    }
}

internal fun logWarn(backend: DesktopBackend, message: () -> String) {
    when (backend) {
        DesktopBackend.MacOs -> MacOsLogger.warn(message)
        DesktopBackend.Wayland -> LinuxLogger.warn(message)
        DesktopBackend.Gtk -> GtkLogger.warn(message)
        DesktopBackend.Windows -> WindowsLogger.warn(message)
    }
}

/**
 * Loads the toolkit's native library for [backend] and points its logger where [params] says.
 *
 * One place rather than four: the Wayland and GTK backends carried an identical private copy of this,
 * and macOS and Windows had neither the log-directory creation nor any way to configure the toolkit at
 * all. `KotlinDesktopToolkit.init` only takes effect once per process, so the appender is assigned
 * afterwards rather than passed in — that way it still applies when the toolkit is already up.
 */
internal fun initToolkit(backend: DesktopBackend, params: ComposeApplicationParams) {
    val libraryFolderPath = params.libraryFolderPath ?: Path.of(requiredProperty("kdt.library.folder.path"))
    val logFilePath = params.logFilePath ?: Path.of(requiredProperty("kdt.native.log.path"))
    // The native logger fails to initialize if the directory is not there, and a fresh checkout has no
    // build directory yet.
    logFilePath.parent?.let(Files::createDirectories)
    val console = params.consoleLogLevel
    val file = params.fileLogLevel
    val appender = params.logAppender
    when (backend) {
        DesktopBackend.MacOs -> {
            MacOsToolkit.init(libraryFolderPath, logFilePath, consoleLogLevel = console.toMacOs(), fileLogLevel = file.toMacOs())
            appender?.let { MacOsLogger.appender = MacOsLoggerAppender(it) }
        }

        DesktopBackend.Wayland -> {
            LinuxToolkit.init(libraryFolderPath, logFilePath, consoleLogLevel = console.toLinux(), fileLogLevel = file.toLinux())
            appender?.let { LinuxLogger.appender = LinuxLoggerAppender(it) }
        }

        DesktopBackend.Gtk -> {
            GtkToolkit.init(libraryFolderPath, logFilePath, consoleLogLevel = console.toGtk(), fileLogLevel = file.toGtk())
            appender?.let { GtkLogger.appender = GtkLoggerAppender(it) }
        }

        DesktopBackend.Windows -> {
            WindowsToolkit.init(libraryFolderPath, logFilePath, consoleLogLevel = console.toWindows(), fileLogLevel = file.toWindows())
            appender?.let { WindowsLogger.appender = WindowsLoggerAppender(it) }
        }
    }
    bootedBackend = backend
    // Only now, with the level and the appender in place: logged any earlier this goes to the default
    // console appender, so a caller who asked for `Off` or supplied its own appender neither suppresses
    // it nor ever sees it.
    logInfo(backend) { startupInfo(backend) }
}

/** Matches the toolkit's own message, since these are the properties it would have read itself. */
private fun requiredProperty(propertyName: String): String = System.getProperty(propertyName)
    ?: error("Please specify $propertyName, or pass it to ComposeApplicationParams")

private fun ComposeLogLevel.toMacOs(): MacOsLogLevel = MacOsLogLevel.valueOf(name)

private fun ComposeLogLevel.toLinux(): LinuxLogLevel = LinuxLogLevel.valueOf(name)

private fun ComposeLogLevel.toGtk(): GtkLogLevel = GtkLogLevel.valueOf(name)

private fun ComposeLogLevel.toWindows(): WindowsLogLevel = WindowsLogLevel.valueOf(name)
