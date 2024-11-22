package org.jetbrains.kwm.macos

import org.jetbrains.kwm.IApplication
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import java.lang.foreign.Arena

object Application: IApplication {
    override fun init() {
        kwm_macos_h.application_init()
    }

    override fun runEventLoop() {
        kwm_macos_h.application_run_event_loop()
    }

    fun createWindow(title: String, x: Float, y: Float) {
        Arena.ofConfined().use { arena ->
            val title = arena.allocateUtf8String(title)
            kwm_macos_h.application_create_window(title, x, y)
        }
    }
}