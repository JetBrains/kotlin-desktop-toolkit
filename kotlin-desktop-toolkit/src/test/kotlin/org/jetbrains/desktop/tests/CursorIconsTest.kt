import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.Cursor
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.LogLevel
import kotlin.test.Test
import kotlin.test.assertEquals

class CursorIconsTest {
    @Test
    fun interateCursorIconsTest() {
        KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Info)
        GrandCentralDispatch.dispatchOnMainSync {
            Application.init()
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
