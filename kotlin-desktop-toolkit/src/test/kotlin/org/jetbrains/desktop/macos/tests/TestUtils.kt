package org.jetbrains.desktop.tests

import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import kotlin.concurrent.thread

fun runTestWithEventLoop(
    eventHandler: (Event) -> EventHandlerResult,
    body: () -> Unit
) {
    KotlinDesktopToolkit.init()
    val applicationStartedLatch = java.util.concurrent.CountDownLatch(1)
    val applicationStoppedLatch = java.util.concurrent.CountDownLatch(1)
    thread {
        GrandCentralDispatch.startOnMainThread {
            Application.init()
            Application.runEventLoop { event ->
                if (event is Event.ApplicationDidFinishLaunching) {
                    applicationStartedLatch.countDown()
                }
                eventHandler(event)
            }
        }
        applicationStoppedLatch.countDown()
    }
    try {
        applicationStartedLatch.await()
        body()
    } finally {
        GrandCentralDispatch.dispatchOnMainSync {
            Application.stopEventLoop()
        }
        applicationStoppedLatch.await()
    }
}