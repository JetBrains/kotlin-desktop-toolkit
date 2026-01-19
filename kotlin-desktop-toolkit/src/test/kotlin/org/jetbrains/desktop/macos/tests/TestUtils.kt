package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.LogLevel
import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.Window
import org.jetbrains.desktop.macos.tests.KeyboardTest.Companion.window
import org.junit.jupiter.api.AfterAll
import org.junit.jupiter.api.BeforeAll
import org.junit.jupiter.api.Timeout
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread

/**
 * We expect that every test class will be executed with a separate JVM instance, without parallel forks.
 * Tests from one test class also should be executed sequentially.
 * This is a requirement because tests interact with NSApplication, which can be initialized only once per process,
 * moreover often the test might change OS state shared across different processes, so it's better to not run it
 * in parallel event in separate processes.
 */
open class KDTTestBase {
    companion object {
        @BeforeAll
        @JvmStatic
        fun loadLibrary() {
            KotlinDesktopToolkit.init(
                consoleLogLevel = LogLevel.Info,
                useDebugBuild = true,
            )
        }
    }
}

open class KDTApplicationTestBase : KDTTestBase() {
    companion object {
        fun <T> withEventHandler(handler: (Event) -> EventHandlerResult, body: () -> T): T {
            eventHandler = handler
            val result = try {
                body()
            } finally {
                eventHandler = null
            }
            return result
        }

        fun <T> ui(body: () -> T): T = GrandCentralDispatch.dispatchOnMainSync(highPriority = false, body)

        fun createWindowAndEnsureItsFocused(name: String): Window {
            window = ui {
                val window = Window.create(origin = LogicalPoint(100.0, 200.0), title = name)
                Logger.info { "$name create with ID: ${window.windowId()}" }
                window
            }
            ui {
                window.makeKeyAndOrderFront()
            }
            awaitEventOfType<Event.WindowChangedOcclusionState> { it.windowId == window.windowId() && it.isVisible }

            if (!window.isKey) {
                ui {
                    window.makeKeyAndOrderFront()
                }
                Logger.info { "$name before Window focused" }
                awaitEventOfType<Event.WindowFocusChange> { it.isKeyWindow }
                Logger.info { "$name Window focused" }
            }
            return window
        }

        val eventQueue = LinkedBlockingQueue<Event>()

        fun awaitEvent(predicate: (Event) -> Boolean): Event {
            while (true) {
                val event = eventQueue.take()
                if (predicate(event)) return event
            }
        }

        inline fun <reified T : Event> awaitEventOfType(crossinline predicate: (T) -> Boolean): T {
            return awaitEvent { it is T && predicate(it) } as T
        }

        @Volatile
        var eventHandler: ((Event) -> EventHandlerResult)? = null

        @Volatile
        lateinit var handle: Thread

        @Timeout(value = 20, unit = TimeUnit.SECONDS)
        @BeforeAll
        @JvmStatic
        fun startApplication() {
            handle = thread {
                GrandCentralDispatch.startOnMainThread {
                    Application.init()
                    Application.runEventLoop { event ->
                        Logger.info { "Event: $event" }
                        assert(eventQueue.offer(event), { "Event queue overflow" })
                        eventHandler?.invoke(event) ?: EventHandlerResult.Continue
                    }
                    GrandCentralDispatch.close()
                }
            }
            awaitEvent { it is Event.ApplicationDidFinishLaunching }
        }

        @Timeout(value = 20, unit = TimeUnit.SECONDS)
        @AfterAll
        @JvmStatic
        fun stopApplication() {
            GrandCentralDispatch.dispatchOnMain {
                Application.stopEventLoop()
            }
            handle.join()
        }
    }
}

fun jbIconBytes(): ByteArray {
    return object {}.javaClass.getResource("/jb-logo.png")!!.readBytes()
}
