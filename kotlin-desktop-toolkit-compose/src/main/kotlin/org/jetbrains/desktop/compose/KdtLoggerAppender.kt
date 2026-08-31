package org.jetbrains.desktop.compose

import org.jetbrains.desktop.gtk.AppenderInterface as GtkAppenderInterface
import org.jetbrains.desktop.linux.AppenderInterface as LinuxAppenderInterface
import org.jetbrains.desktop.macos.AppenderInterface as MacOsAppenderInterface
import org.jetbrains.desktop.win32.AppenderInterface as WindowsAppenderInterface

/**
 * Where the toolkit's logs — the native library's included — should go.
 *
 * Implement this to route them into the host application's logger; without one they go to the console.
 * The toolkit declares the same shape four times, once per backend, so this is the one an application
 * that does not know its backend can implement. Enablement is asked before a message is built, so an
 * appender that answers `false` costs nothing.
 */
interface ComposeLogAppender {
    val isTraceEnabled: Boolean
    val isDebugEnabled: Boolean
    val isInfoEnabled: Boolean
    val isWarnEnabled: Boolean
    val isErrorEnabled: Boolean

    fun trace(message: String)

    fun debug(message: String)

    fun info(message: String)

    fun warn(message: String)

    fun error(message: String)

    fun trace(t: Throwable, message: String)

    fun debug(t: Throwable, message: String)

    fun info(t: Throwable, message: String)

    fun warn(t: Throwable, message: String)

    fun error(t: Throwable, message: String)
}

/** How much the toolkit logs. Mirrors the toolkit's per-backend `LogLevel`. */
enum class ComposeLogLevel { Off, Error, Warn, Info, Debug, Trace }

internal class MacOsLoggerAppender(private val impl: ComposeLogAppender) : MacOsAppenderInterface {
    override val isTraceEnabled: Boolean get() = impl.isTraceEnabled
    override val isDebugEnabled: Boolean get() = impl.isDebugEnabled
    override val isInfoEnabled: Boolean get() = impl.isInfoEnabled
    override val isWarnEnabled: Boolean get() = impl.isWarnEnabled
    override val isErrorEnabled: Boolean get() = impl.isErrorEnabled

    override fun trace(message: String) = impl.trace(message)

    override fun debug(message: String) = impl.debug(message)

    override fun info(message: String) = impl.info(message)

    override fun warn(message: String) = impl.warn(message)

    override fun error(message: String) = impl.error(message)

    override fun trace(t: Throwable, message: String) = impl.trace(t, message)

    override fun debug(t: Throwable, message: String) = impl.debug(t, message)

    override fun info(t: Throwable, message: String) = impl.info(t, message)

    override fun warn(t: Throwable, message: String) = impl.warn(t, message)

    override fun error(t: Throwable, message: String) = impl.error(t, message)
}

internal class LinuxLoggerAppender(private val impl: ComposeLogAppender) : LinuxAppenderInterface {
    override val isTraceEnabled: Boolean get() = impl.isTraceEnabled
    override val isDebugEnabled: Boolean get() = impl.isDebugEnabled
    override val isInfoEnabled: Boolean get() = impl.isInfoEnabled
    override val isWarnEnabled: Boolean get() = impl.isWarnEnabled
    override val isErrorEnabled: Boolean get() = impl.isErrorEnabled

    override fun trace(message: String) = impl.trace(message)

    override fun debug(message: String) = impl.debug(message)

    override fun info(message: String) = impl.info(message)

    override fun warn(message: String) = impl.warn(message)

    override fun error(message: String) = impl.error(message)

    override fun trace(t: Throwable, message: String) = impl.trace(t, message)

    override fun debug(t: Throwable, message: String) = impl.debug(t, message)

    override fun info(t: Throwable, message: String) = impl.info(t, message)

    override fun warn(t: Throwable, message: String) = impl.warn(t, message)

    override fun error(t: Throwable, message: String) = impl.error(t, message)
}

internal class GtkLoggerAppender(private val impl: ComposeLogAppender) : GtkAppenderInterface {
    override val isTraceEnabled: Boolean get() = impl.isTraceEnabled
    override val isDebugEnabled: Boolean get() = impl.isDebugEnabled
    override val isInfoEnabled: Boolean get() = impl.isInfoEnabled
    override val isWarnEnabled: Boolean get() = impl.isWarnEnabled
    override val isErrorEnabled: Boolean get() = impl.isErrorEnabled

    override fun trace(message: String) = impl.trace(message)

    override fun debug(message: String) = impl.debug(message)

    override fun info(message: String) = impl.info(message)

    override fun warn(message: String) = impl.warn(message)

    override fun error(message: String) = impl.error(message)

    override fun trace(t: Throwable, message: String) = impl.trace(t, message)

    override fun debug(t: Throwable, message: String) = impl.debug(t, message)

    override fun info(t: Throwable, message: String) = impl.info(t, message)

    override fun warn(t: Throwable, message: String) = impl.warn(t, message)

    override fun error(t: Throwable, message: String) = impl.error(t, message)
}

internal class WindowsLoggerAppender(private val impl: ComposeLogAppender) : WindowsAppenderInterface {
    override val isTraceEnabled: Boolean get() = impl.isTraceEnabled
    override val isDebugEnabled: Boolean get() = impl.isDebugEnabled
    override val isInfoEnabled: Boolean get() = impl.isInfoEnabled
    override val isWarnEnabled: Boolean get() = impl.isWarnEnabled
    override val isErrorEnabled: Boolean get() = impl.isErrorEnabled

    override fun trace(message: String) = impl.trace(message)

    override fun debug(message: String) = impl.debug(message)

    override fun info(message: String) = impl.info(message)

    override fun warn(message: String) = impl.warn(message)

    override fun error(message: String) = impl.error(message)

    override fun trace(t: Throwable, message: String) = impl.trace(t, message)

    override fun debug(t: Throwable, message: String) = impl.debug(t, message)

    override fun info(t: Throwable, message: String) = impl.info(t, message)

    override fun warn(t: Throwable, message: String) = impl.warn(t, message)

    override fun error(t: Throwable, message: String) = impl.error(t, message)
}
