package org.jetbrains.desktop.macos

import java.nio.file.Path
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.io.path.absolutePathString

public object KotlinDesktopToolkit {
    private val isInitialized: AtomicBoolean = AtomicBoolean(false)

    public fun init(
        libraryPath: Path = property("kdt.library.path"),
        logFilePath: Path = property("kdt.native.log.path"),
        consoleLogLevel: LogLevel = LogLevel.Info,
        fileLogLevel: LogLevel = LogLevel.Info,
        appenderInterface: AppenderInterface = DefaultConsoleAppender.fromLevel(consoleLogLevel),
    ) {
        if (isInitialized.compareAndSet(false, true)) {
            // todo check that native library version is consistent with Kotlin code
            load(libraryPath)
            initLogger(
                logFile = logFilePath,
                consoleLogLevel = consoleLogLevel,
                fileLogLevel = fileLogLevel,
                appender = appenderInterface,
            )
            Logger.info { "KotlinDesktopToolkit is initialized" }
        } else {
            Logger.error { "Init was called for already initialized library" }
        }
    }

    private fun property(propertyName: String): Path {
        val path = System.getProperty(propertyName)
        if (path == null) {
            throw Error("Please specify $propertyName or pass args explicitly to library init")
        }
        return Path.of(path)
    }

    private fun load(libraryPath: Path) {
        System.load(libraryPath.absolutePathString())
    }
}
