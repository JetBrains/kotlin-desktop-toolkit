package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.DraggingItem
import org.jetbrains.desktop.macos.Image
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalRect
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.desktop.macos.Pasteboard
import org.jetbrains.desktop.macos.Window
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import kotlin.test.Test

@EnabledOnOs(OS.MAC)
class DragSessionTest : KDTApplicationTestBase() {
    @Test
    fun startDragSessionSmokeTest() {
        val window = ui {
            Window.create(
                origin = LogicalPoint(100.0, 200.0),
                size = LogicalSize(400.0, 300.0),
                title = "Drag Test Window",
            )
        }

        ui {
            val pasteboardItem = Pasteboard.Item.ofString(
                type = Pasteboard.STRING_TYPE,
                content = "Dragged text",
            )

            val draggingItem = DraggingItem(
                pasteboardItem = pasteboardItem,
                rect = LogicalRect(
                    origin = LogicalPoint(10.0, 10.0),
                    size = LogicalSize(50.0, 50.0),
                ),
                image = Image.fromBytes(jbIconBytes()),
            )

            window.startDragSession(LogicalPoint(10.0, 10.0), listOf(draggingItem))
        }

        ui {
            window.close()
        }
    }
}
