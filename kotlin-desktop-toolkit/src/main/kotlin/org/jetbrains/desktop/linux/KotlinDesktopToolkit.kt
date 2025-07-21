package org.jetbrains.desktop.linux

import java.nio.file.Path
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.io.path.absolutePathString

public object KotlinDesktopToolkit {
    private val isInitialized: AtomicBoolean = AtomicBoolean(false)

    public fun init(
        libraryFolderPath: Path = Path.of(requiredProperty("kdt.library.folder.path")),
        logFilePath: Path = Path.of(requiredProperty("kdt.native.log.path")),
        useDebugBuild: Boolean = System.getProperty("kdt.debug")?.toBooleanStrictOrNull() ?: false,
        consoleLogLevel: LogLevel = LogLevel.Info,
        fileLogLevel: LogLevel = LogLevel.Info,
        appenderInterface: AppenderInterface = DefaultConsoleAppender.fromLevel(consoleLogLevel),
    ) {
        if (isInitialized.compareAndSet(false, true)) {
            // todo check that native library version is consistent with Kotlin code
            val libraryPath = libraryFolderPath.resolve(libraryName(useDebugBuild = useDebugBuild))
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

    private fun requiredProperty(propertyName: String): String {
        return System.getProperty(propertyName) ?: throw Error("Please specify $propertyName or pass args explicitly to library init")
    }

    private fun isAarch64(): Boolean {
        val arch = System.getProperty("os.arch")
        return "aarch64" == arch || "arm64" == arch
    }

    /**
     * See `CompileRustTask.kt` if you would like to change this logic.
     */
    private fun libraryName(useDebugBuild: Boolean): String {
        val targetSuffix = when {
            isAarch64() -> "arm64"
            else -> "x64"
        }

        val debugSuffix = if (useDebugBuild) "+debug" else ""

        val libName = "desktop_linux_${targetSuffix}$debugSuffix"
        return "lib$libName.so"
    }

    private fun load(libraryPath: Path) {
        System.load(libraryPath.absolutePathString())
    }
}
