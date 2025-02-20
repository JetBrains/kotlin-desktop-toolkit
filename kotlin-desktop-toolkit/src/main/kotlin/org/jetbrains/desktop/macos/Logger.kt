package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeExceptionsArray
import org.jetbrains.desktop.macos.generated.NativeLoggerConfiguration
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.nio.file.Path

public enum class LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    ;

    internal fun isNoMoreVerbose(other: LogLevel): Boolean {
        return this.ordinal <= other.ordinal
    }

    internal fun toNative(): Int {
        return when (this) {
            Off -> desktop_macos_h.NativeLogLevel_Off()
            Error -> desktop_macos_h.NativeLogLevel_Error()
            Warn -> desktop_macos_h.NativeLogLevel_Warn()
            Info -> desktop_macos_h.NativeLogLevel_Info()
            Debug -> desktop_macos_h.NativeLogLevel_Debug()
            Trace -> desktop_macos_h.NativeLogLevel_Trace()
        }
    }
}

public interface AppenderInterface {
    public val isTraceEnabled: Boolean
    public val isDebugEnabled: Boolean
    public val isInfoEnabled: Boolean
    public val isWarnEnabled: Boolean
    public val isErrorEnabled: Boolean

    public fun trace(message: String)
    public fun debug(message: String)
    public fun info(message: String)
    public fun warn(message: String)
    public fun error(message: String)

    public fun trace(t: Throwable, message: String)
    public fun debug(t: Throwable, message: String)
    public fun info(t: Throwable, message: String)
    public fun warn(t: Throwable, message: String)
    public fun error(t: Throwable, message: String)
}

internal class DefaultConsoleAppender(
    override val isTraceEnabled: Boolean,
    override val isDebugEnabled: Boolean,
    override val isInfoEnabled: Boolean,
    override val isWarnEnabled: Boolean,
    override val isErrorEnabled: Boolean,
) : AppenderInterface {

    companion object {
        fun fromLevel(level: LogLevel = LogLevel.Info): DefaultConsoleAppender {
            return DefaultConsoleAppender(
                isTraceEnabled = LogLevel.Trace.isNoMoreVerbose(level),
                isDebugEnabled = LogLevel.Debug.isNoMoreVerbose(level),
                isInfoEnabled = LogLevel.Info.isNoMoreVerbose(level),
                isWarnEnabled = LogLevel.Warn.isNoMoreVerbose(level),
                isErrorEnabled = LogLevel.Error.isNoMoreVerbose(level),
            )
        }
    }

    override fun trace(message: String) {
        System.err.println("[TRACE] $message")
    }

    override fun trace(t: Throwable, message: String) {
        System.err.println("[TRACE] $message")
        System.err.println(t.stackTraceToString())
    }

    override fun debug(message: String) {
        System.err.println("[DEBUG] $message")
    }

    override fun debug(t: Throwable, message: String) {
        System.err.println("[DEBUG] $message")
        System.err.println(t.stackTraceToString())
    }

    override fun info(message: String) {
        System.err.println("[INFO] $message")
    }

    override fun info(t: Throwable, message: String) {
        System.err.println("[INFO] $message")
        System.err.println(t.stackTraceToString())
    }

    override fun warn(message: String) {
        System.err.println("[WARN] $message")
    }

    override fun warn(t: Throwable, message: String) {
        System.err.println("[WARN] $message")
        System.err.println(t.stackTraceToString())
    }

    override fun error(message: String) {
        System.err.println("[ERROR] $message")
    }

    override fun error(t: Throwable, message: String) {
        System.err.println("[ERROR] $message")
        System.err.println(t.stackTraceToString())
    }
}

public object Logger {
    public var appender: AppenderInterface = DefaultConsoleAppender.fromLevel(LogLevel.Info)

    public inline fun trace(msg: () -> String) {
        if (appender.isTraceEnabled) {
            appender.trace(msg())
        }
    }

    public inline fun debug(msg: () -> String) {
        if (appender.isDebugEnabled) {
            appender.debug(msg())
        }
    }

    public inline fun info(msg: () -> String) {
        if (appender.isInfoEnabled) {
            appender.info(msg())
        }
    }

    public inline fun warn(msg: () -> String) {
        if (appender.isWarnEnabled) {
            appender.warn(msg())
        }
    }

    public inline fun error(msg: () -> String) {
        if (appender.isErrorEnabled) {
            appender.error(msg())
        }
    }

    public inline fun trace(t: Throwable, msg: () -> String = { "" }) {
        if (appender.isTraceEnabled) {
            appender.trace(t, msg())
        }
    }

    public inline fun debug(t: Throwable, msg: () -> String = { "" }) {
        if (appender.isDebugEnabled) {
            appender.debug(t, msg())
        }
    }

    public inline fun info(t: Throwable, msg: () -> String = { "" }) {
        if (appender.isInfoEnabled) {
            appender.info(t, msg())
        }
    }

    public inline fun warn(t: Throwable, msg: () -> String = { "" }) {
        if (appender.isWarnEnabled) {
            appender.warn(t, msg())
        }
    }

    public inline fun error(t: Throwable, msg: () -> String = { "" }) {
        if (appender.isErrorEnabled) {
            appender.error(t, msg())
        }
    }
}

internal fun initLogger(logFile: Path, consoleLogLevel: LogLevel, fileLogLevel: LogLevel, appender: AppenderInterface) {
    ffiDownCall {
        Logger.appender = appender

        Arena.ofConfined().use { arena ->
            val configuration = NativeLoggerConfiguration.allocate(arena)
            val logFileStr = logFile.toAbsolutePath().toString()
            NativeLoggerConfiguration.file_path(configuration, arena.allocateUtf8String(logFileStr))
            NativeLoggerConfiguration.console_level(configuration, consoleLogLevel.toNative())
            NativeLoggerConfiguration.file_level(configuration, fileLogLevel.toNative())
            desktop_macos_h.logger_init(configuration)
        }
    }
}

internal class NativeError(messages: List<String>) : Error(messages.joinToString(prefix = "[\n", separator = ",\n", postfix = "]"))

private fun checkExceptions(): List<String> {
    return Arena.ofConfined().use { arena ->
        val exceptionsArray = desktop_macos_h.logger_check_exceptions(arena)
        val count = NativeExceptionsArray.count(exceptionsArray)
        val items = NativeExceptionsArray.items(exceptionsArray)

        if (count != 0L) {
            (0 until count).map { i ->
                val cStrPtr = items.getAtIndex(NativeExceptionsArray.`items$layout`(), i)
                cStrPtr.getUtf8String(0)
            }.toList()
        } else {
            emptyList()
        }
    }
}

internal fun <T> ffiDownCall(body: () -> T): T {
    val result = body()
    val exceptions = checkExceptions()
    if (exceptions.isNotEmpty()) {
        desktop_macos_h.logger_clear_exceptions()
        throw NativeError(exceptions)
    }
    return result
}

internal inline fun ffiUpCall(crossinline body: () -> Unit) {
    return try {
        body()
    } catch (e: Throwable) {
        Logger.error(e) { "Exception caught" }
    }
}

internal inline fun <T> ffiUpCall(default: T, crossinline body: () -> T): T {
    return try {
        body()
    } catch (e: Throwable) {
        Logger.error(e) { "Exception caught" }
        default
    }
}
