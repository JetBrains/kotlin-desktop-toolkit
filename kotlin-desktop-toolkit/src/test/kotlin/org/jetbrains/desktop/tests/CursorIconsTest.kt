import org.jetbrains.desktop.macos.Cursor
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.LogLevel
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.Window
import kotlin.test.Test
import kotlin.test.assertEquals

class CursorIconsTest {
    @Test
    fun interateCursorIconsTest() {
        KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Info)
//        GrandCentralDispatch.dispatchOnMainSync {
//            Application.init()
//        }
        val window1 = GrandCentralDispatch.dispatchOnMainSync {
            Window.create(origin = LogicalPoint(100.0, 200.0), title = "Hello1")
        }
        Cursor.Icon.entries.forEach { icon ->
            GrandCentralDispatch.dispatchOnMainSync {
                Cursor.icon = icon
            }
            val actualIcon = GrandCentralDispatch.dispatchOnMainSync {
                Cursor.icon
            }
            assertEquals(actualIcon, icon)
        }
        GrandCentralDispatch.dispatchOnMainSync {
            window1.close()
        }
    }

    @Test
    fun hideAndShowTest() {
        KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Info)
//        GrandCentralDispatch.dispatchOnMainSync {
//            Application.init()
//        }
        GrandCentralDispatch.dispatchOnMainSync {
            Cursor.hidden = true
        }
        val actualHidden = GrandCentralDispatch.dispatchOnMainSync {
            Cursor.hidden
        }
        assertEquals(actualHidden, true)
        GrandCentralDispatch.dispatchOnMainSync {
            Cursor.hidden = false
        }
        val actualVisible = GrandCentralDispatch.dispatchOnMainSync {
            !Cursor.hidden
        }
        assertEquals(actualVisible, true)
    }
}
