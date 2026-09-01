@file:Suppress("SameParameterValue")

package org.jetbrains.desktop.gtk.tests

import com.squareup.moshi.JsonClass
import com.squareup.moshi.Moshi
import com.squareup.moshi.kotlin.reflect.KotlinJsonAdapterFactory
import com.sun.jna.NativeLong
import com.sun.jna.Pointer
import com.sun.jna.platform.unix.X11
import com.sun.jna.ptr.IntByReference
import com.sun.jna.ptr.NativeLongByReference
import com.sun.jna.ptr.PointerByReference
import org.jetbrains.desktop.gtk.LogicalPixels
import org.jetbrains.desktop.gtk.LogicalPixelsInt
import org.jetbrains.desktop.gtk.LogicalRect
import org.jetbrains.desktop.gtk.LogicalSize
import org.jetbrains.desktop.gtk.MouseButton
import org.jetbrains.desktop.gtk.PhysicalPixels
import org.jetbrains.desktop.gtk.Scale
import java.nio.file.Path
import kotlin.io.path.absolutePathString
import kotlin.math.roundToInt

internal data class GlobalPosition(
    val x: LogicalPixels,
    val y: LogicalPixels,
)

internal data class GlobalRect(
    val position: GlobalPosition,
    val width: LogicalPixels,
    val height: LogicalPixels,
)

internal data class WmVersion(
    val name: String,
    val major: Int,
    val minor: Int,
    val patch: Int,
)

internal class X11PropertyReader(
    private val x11: X11,
    private val display: X11.Display,
) {
    data class X11PropertyData(
        val pointer: Pointer,
        val format: Int,
        val itemCount: Int,
    )

    private val atomsCache = HashMap<String, X11.Atom>()

    private fun atom(name: String): X11.Atom {
        var atom = atomsCache[name]
        if (atom == null) {
            atom = x11.XInternAtom(display, name, false)
            atomsCache[name] = atom
        }
        return atom
    }

    fun <T> withProperty(w: X11.Window, atom: X11.Atom, regType: X11.Atom, block: (X11PropertyData) -> T): T? {
        val xaRetTypeRef = X11.AtomByReference()
        val retFormatRef = IntByReference()
        val retNItemsRef = NativeLongByReference()
        val retBytesAfterRef = NativeLongByReference()
        val retPropRef = PointerByReference()

        val longOffset = NativeLong(0)
        val longLength = NativeLong((4096 / 4).toLong())

        val status = x11.XGetWindowProperty(
            display, w, atom, longOffset, longLength, false,
            regType, xaRetTypeRef, retFormatRef,
            retNItemsRef, retBytesAfterRef, retPropRef,
        )

        if (status != X11.Success) {
            log("XGetWindowProperty status: $status")
            return null
        }

        val xaRetType = xaRetTypeRef.getValue()
        val retProp = retPropRef.value

        if (xaRetType == null) {
            x11.XFree(retProp)
            return null
        }

        val itemCount = retNItemsRef.value.toInt()
        val propertyData = X11PropertyData(pointer = retPropRef.value, format = retFormatRef.value, itemCount = itemCount)
        return try {
            block(propertyData)
        } finally {
            x11.XFree(propertyData.pointer)
        }
    }

    fun <T> withProperty(w: X11.Window, name: String, regType: X11.Atom, block: (X11PropertyData) -> T): T? {
        return withProperty(w, atom(name), regType, block)
    }

    fun <T> withProperty(w: X11.Window, name: String, regTypeName: String, block: (X11PropertyData) -> T): T? {
        return withProperty(w, atom(name), atom(regTypeName), block)
    }
}

internal class X11WindowOperations(
    private val x11: X11,
    private val display: X11.Display,
    private val scale: Scale,
    private val window: X11.Window,
    private val x11PropReader: X11PropertyReader,
) {
    fun fullScreen() {
        runCommand(listOf("wmctrl", "-i", "-r", window.toString(), "-b", "add,fullscreen"))
    }

    fun unsetFullScreen() {
        runCommand(listOf("wmctrl", "-i", "-r", window.toString(), "-b", "remove,fullscreen"))
    }

    fun close() {
        runCommand(listOf("wmctrl", "-i", "-c", window.toString()))
    }

    fun moveTo(pos: GlobalPosition) {
        val x: Int = pos.x.toRawPhysical(scale).roundToInt()
        val y: Int = pos.y.toRawPhysical(scale).roundToInt()
        val posBefore = position()
        x11.XMoveWindow(display, window, x, y)
        x11.XFlush(display)
        waitUntilEq(true) {
            posBefore != position()
        }
        log("Moved window $window to $x,$y")
    }

    private fun getActiveWindow(): X11.Window? {
        val root = x11.XDefaultRootWindow(display)!!
        return x11PropReader.withProperty(root, "_NET_ACTIVE_WINDOW", X11.XA_WINDOW) {
            X11.Window(it.pointer.getInt(0).toLong())
        }
    }

    fun focus() {
        runCommand(listOf("wmctrl", "-F", "-i", "-a", window.toString()))
        waitUntilEq(window) { getActiveWindow() }
    }

    private fun rawPhysicalToLogical(rawPhysical: Long): LogicalPixels {
        return scale.rawPhysicalToLogical(rawPhysical.toDouble())
    }

    fun clientArea(): GlobalRect? {
        val rootRet = X11.WindowByReference()
        val xRelRet = IntByReference()
        val yRelRet = IntByReference()
        val widthRet = IntByReference()
        val heightRet = IntByReference()
        val borderWidthRet = IntByReference()
        val depthRet = IntByReference()

        if (x11.XGetGeometry(
                display,
                window,
                rootRet,
                xRelRet,
                yRelRet,
                widthRet,
                heightRet,
                borderWidthRet,
                depthRet,
            ) == 0
        ) {
            log("XGetGeometry failed for $window")
            return null
        }
        val geometryWidth = widthRet.value
        val geometryHeight = heightRet.value
        val borderWidth = borderWidthRet.value
        val frameLeft = xRelRet.value
        val frameTop = yRelRet.value
        log(
            "XGetGeometry for $window: relative x=$frameLeft, relative y=$frameTop, width=$geometryWidth, height=$geometryHeight, borderWidth=$borderWidth",
        )

        val x = IntByReference()
        val y = IntByReference()
        if (!x11.XTranslateCoordinates(
                display,
                window,
                rootRet.getValue(),
                -borderWidth,
                -borderWidth,
                x,
                y,
                rootRet,
            )
        ) {
            log("XTranslateCoordinates failed for $window")
            return null
        }

        val geometryX = x.value
        val geometryY = y.value
        log("XTranslateCoordinates for $window: x=$geometryX, y=$geometryY")

//        val xwininfoLines = ProcessBuilder("xwininfo", "-id", window.toString()).start().inputReader().readLines()
//        log(xwininfoLines.joinToString("\n"))
        return GlobalRect(
            GlobalPosition(
                x = rawPhysicalToLogical(geometryX.toLong()),
                y = rawPhysicalToLogical(geometryY.toLong()),
            ),
            width = rawPhysicalToLogical(geometryWidth.toLong()),
            height = rawPhysicalToLogical(geometryHeight.toLong()),
        )
    }

    fun position(): GlobalPosition? {
        return clientArea()?.position
    }

    fun getMinimalSizeHint(): LogicalSize? {
//        val xpropLines = ProcessBuilder("xprop", "-id", window.toString()).start().inputReader().readLines()
//        log(xpropLines.joinToString("\n"))
        return x11PropReader.withProperty(window, X11.XA_WM_NORMAL_HINTS, X11.XA_WM_SIZE_HINTS) {
            val rawPhysicalMinWidth = it.pointer.getLong(Long.SIZE_BYTES * 5L)
            val rawPhysicalMinHeight = it.pointer.getLong(Long.SIZE_BYTES * 6L)
            val minWidth = rawPhysicalToLogical(rawPhysicalMinWidth)
            val minHeight = rawPhysicalToLogical(rawPhysicalMinHeight)
            if (minWidth.rawLogical <= 1 && minHeight.rawLogical <= 1) {
                null
            } else {
                log("minWidth=$minWidth, minHeight=$minHeight")
                LogicalSize(
                    width = LogicalPixelsInt(minWidth.rawLogical.roundToInt()),
                    height = LogicalPixelsInt(minHeight.rawLogical.roundToInt()),
                )
            }
        }
    }

    private fun readUtf8Property(window: X11.Window, propertyName: String): String? {
        return x11PropReader.withProperty(window, propertyName, "UTF8_STRING") {
            when (it.format) {
                8 -> it.pointer.getByteArray(0, it.itemCount).toString(Charsets.UTF_8).trim().takeIf(String::isNotEmpty)
                else -> null
            }
        }
    }

    fun readTitle(): String? {
        return readUtf8Property(window, "_NET_WM_NAME")
    }

    fun screenshot(outPath: Path, rect: LogicalRect? = null, hideCursor: Boolean = true) {
        val cmd = buildList {
            add("maim")
            if (rect != null) {
                val x: Int = rect.x.toRawPhysical(scale).roundToInt()
                val y: Int = rect.y.toRawPhysical(scale).roundToInt()
                val width: Int = rect.width.toRawPhysical(scale).roundToInt()
                val height: Int = rect.height.toRawPhysical(scale).roundToInt()
                add("-g")
                add("${width}x$height+$x+$y")
            }
            if (hideCursor) {
                add("--hidecursor")
            }
            add("--window=${window.toLong()}")
            add(outPath.absolutePathString())
        }
        runCommand(cmd)
    }
}

internal class X11Wm(private val scale: Scale) {
    companion object {
        @Suppress("PropertyName")
        @JsonClass(generateAdapter = true)
        private data class I3VersionModel(
            val human_readable: String,
            val major: Int,
            val minor: Int,
            val patch: Int,
            val loaded_config_file_name: String,
        )

        private fun getI3Version(): I3VersionModel? {
            // log("getI3Version")
            val json = runCommandWithOutput(listOf("i3-msg", "--raw", "-t", "get_version"))?.decodeToString() ?: return null
            // log(json)
            val moshi: Moshi = Moshi.Builder().addLast(KotlinJsonAdapterFactory()).build()
            val jsonAdapter = moshi.adapter(I3VersionModel::class.java)
            return jsonAdapter.fromJson(json)
        }
    }

    private val x11 = X11.INSTANCE
    private val display: X11.Display = X11.INSTANCE.XOpenDisplay(null)!!

    private val x11PropReader = X11PropertyReader(x11, display)

    fun getVersion(): WmVersion {
        return getI3Version()?.let {
            WmVersion(
                name = "i3",
                major = it.major,
                minor = it.minor,
                patch = it.patch,
            )
        } ?: WmVersion(
            name = "",
            major = 0,
            minor = 0,
            patch = 0,
        )
    }

    fun getWindowByTitle(title: String): X11WindowOperations? {
        val children = mutableListOf<X11.Window>()
        fillChildren(x11.XDefaultRootWindow(display), children)
        return children.firstNotNullOfOrNull {
            val operations = X11WindowOperations(x11, display, scale, it, x11PropReader)
            val actualTitle = operations.readTitle()
            if (actualTitle == title) {
                log("Found window with title $title: $it")
                waitUntilEq(true) {
                    val clientArea = operations.clientArea()
                    clientArea != null && clientArea.height.rawLogical > 1.0 && clientArea.width.rawLogical > 1.0
                }
                operations
            } else {
                null
            }
        } ?: run {
            val lines = ProcessBuilder("xwininfo", "-root", "-tree").start().inputReader().readLines()
            log("getWindowByTitle failed: ${lines.joinToString("\n")}")
            null
        }
    }

    private fun fillChildren(window: X11.Window, out: MutableList<X11.Window>) {
        val rootReturn = X11.WindowByReference()
        val parentReturn = X11.WindowByReference()
        val childrenReturn = PointerByReference()
        val childCountReturn = IntByReference()
        if (x11.XQueryTree(display, window, rootReturn, parentReturn, childrenReturn, childCountReturn) == 0) {
            log("XQueryTree failed")
            return
        }
        val childCount = childCountReturn.value

        if (childCount > 0) {
            val arr = childrenReturn.value.getLongArray(0, childCount)
            for (childId in arr) {
                val child = X11.Window(childId)
                out.add(child)
                fillChildren(child, out)
            }
        }
    }

    fun <T> withXtest(doXtest: (X11.XTest, X11.Display) -> Unit, undoXtest: (X11.XTest, X11.Display) -> Unit, block: () -> T): T {
        doXtest(X11.XTest.INSTANCE, display)
        x11.XSync(display, false)
        AutoCloseable {
            undoXtest(X11.XTest.INSTANCE, display)
            x11.XSync(display, false)
        }.use {
            return block()
        }
    }

    fun <T> withKeyPress(key: UInt, block: () -> T): T {
        return withXtest(
            { xtest, display ->
                log("Key down: $key")
                xtest.XTestFakeKeyEvent(display, key.toInt(), true, NativeLong(0))
            },
            { xtest, display ->
                log("Key up: $key")
                xtest.XTestFakeKeyEvent(display, key.toInt(), false, NativeLong(0))
            },
            block,
        )
    }

    fun <T> withMouseButtonDown(button: MouseButton, block: () -> T): T {
        return withXtest(
            { xtest, display ->
                log("Mouse button down: $button")
                xtest.XTestFakeButtonEvent(display, button.value, true, NativeLong(0))
            },
            { xtest, display ->
                log("Mouse button up: $button")
                xtest.XTestFakeButtonEvent(display, button.value, false, NativeLong(0))
            },
            block,
        )
    }

    private fun getMousePosition(): Pair<PhysicalPixels, PhysicalPixels>? {
        val rootWindow = x11.XDefaultRootWindow(display)
        val rootReturn = X11.WindowByReference()
        val childReturn = X11.WindowByReference()
        val rootXReturn = IntByReference()
        val rootYReturn = IntByReference()
        val winXReturn = IntByReference()
        val winYReturn = IntByReference()
        val maskReturn = IntByReference()
        val ret = x11.XQueryPointer(
            display, rootWindow, rootReturn, childReturn, rootXReturn, rootYReturn, winXReturn, winYReturn, maskReturn,
        )
        return if (ret) {
            Pair(PhysicalPixels(rootXReturn.value), PhysicalPixels(rootYReturn.value))
        } else {
            null
        }
    }

    private fun moveMouseToImpl(pos: Pair<PhysicalPixels, PhysicalPixels>) {
        withXtest(
            { xtest, display ->
                val x = pos.first.rawPhysical
                val y = pos.second.rawPhysical
                xtest.XTestFakeMotionEvent(display, 0, x, y, NativeLong(0))
                waitUntilEq(pos) { getMousePosition() }
                log("wm.moveMouseTo $x, $y done")
            },
            { _, _ -> },
            {},
        )
    }

    fun moveMouseTo(pos: TestMousePosition) {
        val x = PhysicalPixels((pos.base.x + pos.offsetX.toLogicalPixels()).toRawPhysical(scale).roundToInt())
        val y = PhysicalPixels((pos.base.y + pos.offsetY.toLogicalPixels()).toRawPhysical(scale).roundToInt())
        moveMouseToImpl(Pair(x, y))
    }

    fun resetMousePosition() {
        moveMouseToImpl(Pair(PhysicalPixels.Zero, PhysicalPixels.Zero))
    }

    fun scrollMouseUp() {
        // https://askubuntu.com/a/1162351
        withXtest(
            { xtest, display ->
                xtest.XTestFakeButtonEvent(display, 4, true, NativeLong(0))
                xtest.XTestFakeButtonEvent(display, 4, false, NativeLong(0))
            },
            { _, _ -> },
            {},
        )
    }

    fun scrollMouseDown() {
        // https://askubuntu.com/a/1162351
        withXtest(
            { xtest, display ->
                xtest.XTestFakeButtonEvent(display, 5, true, NativeLong(0))
                xtest.XTestFakeButtonEvent(display, 5, false, NativeLong(0))
            },
            { _, _ -> },
            {},
        )
    }
}
