package org.jetbrains.kwm.macos

import org.jetbrains.kwm.macos.generated.ExceptionsArray
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import org.jetbrains.kwm.macos.generated.LoggerConfiguration as NativeLoggerConfiguration
import java.lang.foreign.Arena
import java.nio.file.Path

enum class LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace;

    fun isNoMoreVerbose(other: LogLevel): Boolean {
        return this.ordinal >= other.ordinal
    }

    internal fun toNative(): Int {
        return when (this) {
            Error -> kwm_macos_h.Error()
            Warn -> kwm_macos_h.Warn()
            Info -> kwm_macos_h.Info()
            Debug -> kwm_macos_h.Debug()
            Trace -> kwm_macos_h.Trace()
        }
    }
}

interface LoggerInterface {
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

class DefaultConsoleLogger(override val isTraceEnabled: Boolean = false,
                           override val isDebugEnabled: Boolean = false,
                           override val isInfoEnabled: Boolean = true,
                           override val isWarnEnabled: Boolean = true,
                           override val isErrorEnabled: Boolean = true): LoggerInterface {

    companion object {
        fun fromLevel(level: LogLevel = LogLevel.Info): DefaultConsoleLogger {
            return DefaultConsoleLogger(
                isTraceEnabled = LogLevel.Trace.isNoMoreVerbose(level),
                isDebugEnabled = LogLevel.Debug.isNoMoreVerbose(level),
                isInfoEnabled = LogLevel.Info.isNoMoreVerbose(level),
                isWarnEnabled = LogLevel.Warn.isNoMoreVerbose(level),
                isErrorEnabled = LogLevel.Error.isNoMoreVerbose(level)
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
        println("[SKIKO] info: $message")
        println(t)
    }

    override fun warn(message: String) {
        System.err.println("[DEBUG] $message")
    }

    override fun warn(t: Throwable, message: String) {
        System.err.println("[DEBUG] $message")
        println(t)
    }

    override fun error(message: String) {
        System.err.println("[DEBUG] $message")
    }

    override fun error(t: Throwable, message: String) {
        System.err.println("[DEBUG] $message")
        println(t)
    }
}

internal object Logger {
    var loggerFactory: () -> SkikoLoggerInterface = { DefaultConsoleLogger() }

    val loggerImpl by lazy {
        loggerFactory()
    }

    inline fun trace(msg: () -> String) {
        if (loggerImpl.isTraceEnabled) {
            loggerImpl.trace(msg())
        }
    }

    inline fun debug(msg: () -> String) {
        if (loggerImpl.isDebugEnabled) {
            loggerImpl.debug(msg())
        }
    }

    inline fun info(msg: () -> String) {
        if (loggerImpl.isInfoEnabled) {
            loggerImpl.info(msg())
        }
    }

    inline fun warn(msg: () -> String) {
        if (loggerImpl.isWarnEnabled) {
            loggerImpl.warn(msg())
        }
    }

    inline fun error(msg: () -> String) {
        if (loggerImpl.isErrorEnabled) {
            loggerImpl.error(msg())
        }
    }

    inline fun trace(t: Throwable, msg: () ->  String) {
        if (loggerImpl.isTraceEnabled) {
            loggerImpl.trace(t, msg())
        }
    }

    inline fun debug(t: Throwable, msg: () ->  String) {
        if (loggerImpl.isDebugEnabled) {
            loggerImpl.debug(t, msg())
        }
    }

    inline fun info(t: Throwable, msg: () -> String) {
        if (loggerImpl.isInfoEnabled) {
            loggerImpl.info(t, msg())
        }
    }
    inline fun warn(t: Throwable, msg: () -> String) {
        if (loggerImpl.isWarnEnabled) {
            loggerImpl.warn(t, msg())
        }
    }
    inline fun error(t: Throwable, msg:() ->  String) {
        if (loggerImpl.isErrorEnabled) {
            loggerImpl.error(t, msg())
        }
    }
}

fun initNativeLogger(logFile: Path,
                     consoleLogLevel: LogLevel = LogLevel.Warn,
                     fileLogLevel: LogLevel = LogLevel.Info) {
    Arena.ofConfined().use { arena ->
        val configuration = NativeLoggerConfiguration.allocate(arena)
        val logFileStr = logFile.toAbsolutePath().toString()
        NativeLoggerConfiguration.file_path(configuration, arena.allocateUtf8String(logFileStr))
        NativeLoggerConfiguration.console_level(configuration, consoleLogLevel.toNative())
        NativeLoggerConfiguration.file_level(configuration, fileLogLevel.toNative())
        kwm_macos_h.logger_init(configuration)
    }
}

class NativeError(messages: List<String>): Error(messages.joinToString(separator = "\n"))

private fun checkExceptions(): List<String> {
    return Arena.ofConfined().use { arena ->
        val exceptionsArray = kwm_macos_h.logger_check_exceptions(arena)
        val count = ExceptionsArray.count(exceptionsArray)
        val items = ExceptionsArray.items(exceptionsArray)

        if (count != 0L) {
            (0 until count).map { i ->
                val cStrPtr = items.getAtIndex(ExceptionsArray.`items$layout`(), i)
                cStrPtr.getUtf8String(0)
            }.toList()
        } else {
            emptyList()
        }
    }
}

fun <T> withThrowNativeExceptions(body: () -> T): T {
    val result = body()
    val exceptions = checkExceptions()
    if (exceptions.isNotEmpty()) {
        kwm_macos_h.logger_clear_exceptions()
        throw NativeError(exceptions)
    }
    return result
}