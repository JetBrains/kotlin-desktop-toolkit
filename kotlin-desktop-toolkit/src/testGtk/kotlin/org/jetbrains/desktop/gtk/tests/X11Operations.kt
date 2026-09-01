package org.jetbrains.desktop.gtk.tests

import com.github.moaxcp.x11.protocol.Utilities
import com.github.moaxcp.x11.protocol.X11BigEndianOutputStream
import com.github.moaxcp.x11.protocol.X11LittleEndianOutputStream
import com.github.moaxcp.x11.protocol.xproto.Atom
import com.github.moaxcp.x11.protocol.xproto.ClientMessageData32
import com.github.moaxcp.x11.protocol.xproto.ClientMessageEvent
import com.github.moaxcp.x11.protocol.xproto.ConfigureWindow
import com.github.moaxcp.x11.protocol.xproto.EventMask
import com.github.moaxcp.x11.protocol.xproto.GetGeometry
import com.github.moaxcp.x11.protocol.xproto.GetImage
import com.github.moaxcp.x11.protocol.xproto.GetProperty
import com.github.moaxcp.x11.protocol.xproto.GetSelectionOwner
import com.github.moaxcp.x11.protocol.xproto.ImageOrder
import com.github.moaxcp.x11.protocol.xproto.MotionNotifyEvent
import com.github.moaxcp.x11.protocol.xproto.QueryPointer
import com.github.moaxcp.x11.protocol.xproto.SendEvent
import com.github.moaxcp.x11.protocol.xproto.TranslateCoordinates
import com.github.moaxcp.x11.protocol.xtest.FakeInput
import com.github.moaxcp.x11.x11client.X11Client
import com.squareup.moshi.JsonClass
import com.squareup.moshi.Moshi
import com.squareup.moshi.kotlin.reflect.KotlinJsonAdapterFactory
import org.jetbrains.desktop.gtk.LogicalPixels
import org.jetbrains.desktop.gtk.LogicalPixelsInt
import org.jetbrains.desktop.gtk.LogicalRect
import org.jetbrains.desktop.gtk.LogicalSize
import org.jetbrains.desktop.gtk.MouseButton
import org.jetbrains.desktop.gtk.PhysicalPixels
import org.jetbrains.desktop.gtk.PhysicalSize
import org.jetbrains.desktop.gtk.Scale
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.IntBuffer
import kotlin.math.roundToInt
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlin.test.fail

internal data class GlobalPosition<T>(
    val x: T,
    val y: T,
)

internal data class WmVersion(
    val name: String,
    val major: Int,
    val minor: Int,
    val patch: Int,
)

internal data class TransformData(
    val x: PhysicalPixels,
    val y: PhysicalPixels,
)

internal data class PhysicalExtents(
    val top: PhysicalPixels,
    val left: PhysicalPixels,
    val right: PhysicalPixels,
    val bottom: PhysicalPixels,
)

private data class X11PropertyData(
    private val format: Byte,
    private val byteBuffer: ByteBuffer,
) {
    fun asIntBuffer(): IntBuffer {
        assertEquals(32, format)
        return byteBuffer.asIntBuffer()
    }
}

private object X11Helper {
    fun getProperty(x11: X11Client, window: Int, property: Int, type: Atom): X11PropertyData? {
        val reply = x11.send(
            GetProperty
                .builder()
                .window(window)
                .property(property)
                .type(type.value)
                .longOffset(0)
                .longLength(1024)
                .delete(false)
                .build(),
        )

        if (reply.type == 0) {
            return null
        }

        assertEquals(type.value, reply.type)
//        log("len = ${reply.valueLen}")
        return X11PropertyData(
            reply.format,
            ByteBuffer
                .wrap(reply.value.toArray())
                .asReadOnlyBuffer()
                .order(if (x11.bigEndian) ByteOrder.BIG_ENDIAN else ByteOrder.LITTLE_ENDIAN),
        )
    }

    fun getRootWindowProperty(x11: X11Client, propertyName: String, type: Atom): X11PropertyData? {
        return getProperty(x11, x11.defaultRoot, x11.getAtom(propertyName).id, type)
    }

    fun readNetWmName(x11: X11Client, window: Int): String? {
        // https://specifications.freedesktop.org/wm/1.5/ar01s05.html#id-1.6.2
        val reply = x11.send(
            GetProperty
                .builder()
                .window(window)
                .property(x11.getAtom("_NET_WM_NAME").id)
                .type(x11.getAtom("UTF8_STRING").id)
                .longOffset(0)
                .longLength(1024)
                .delete(false)
                .build(),
        )
        return when (reply.format) {
            8.toByte() -> Utilities.toString(reply.value)
            else -> null
        }
    }

    fun getMousePosition(x11: X11Client): Pair<Short, Short> {
        val reply = x11.send(QueryPointer.builder().window(x11.defaultRoot).build())
        return Pair(reply.rootX, reply.rootY)
    }

    fun moveMouseTo(x11: X11Client, x: Short, y: Short) {
        log("wm.moveMouseTo $x, $y start")
        x11.send(
            FakeInput
                .builder()
                .root(x11.defaultRoot)
                .type(MotionNotifyEvent.NUMBER)
                .rootX(x)
                .rootY(y)
                .time(0)
                .build(),
        )
        x11.sync()
    }

    fun getGtkFrameExtents(x11: X11Client, window: Int): PhysicalExtents? {
        // https://github.com/GNOME/gtk/blob/4.22.4/gdk/x11/gdksurface-x11.c#L240-L246
        val intBuffer = getProperty(
            x11,
            window = window,
            property = x11.getAtom("_GTK_FRAME_EXTENTS").id,
            type = Atom.CARDINAL,
        )?.asIntBuffer() ?: return null
        assertTrue(isComposerActive(x11, screen = 0)) // sanity check
        val left = intBuffer.get()
        val right = intBuffer.get()
        val top = intBuffer.get()
        val bottom = intBuffer.get()
        return PhysicalExtents(
            top = PhysicalPixels(top),
            left = PhysicalPixels(left),
            right = PhysicalPixels(right),
            bottom = PhysicalPixels(bottom),
        ).also {
            log("_GTK_FRAME_EXTENTS=$it")
        }
    }

    fun getNetWmExtents(x11: X11Client, window: Int): PhysicalExtents? {
        // https://specifications.freedesktop.org/wm/1.5/ar01s05.html#id-1.6.18
        val intBuffer = getProperty(
            x11,
            window = window,
            property = x11.getAtom("_NET_FRAME_EXTENTS").id,
            type = Atom.CARDINAL,
        )?.asIntBuffer() ?: return null
        val left = intBuffer.get()
        val right = intBuffer.get()
        val top = intBuffer.get()
        val bottom = intBuffer.get()
        return PhysicalExtents(
            top = PhysicalPixels(top),
            left = PhysicalPixels(left),
            right = PhysicalPixels(right),
            bottom = PhysicalPixels(bottom),
        ).also {
            log("_NET_FRAME_EXTENTS=$it")
        }
    }

    private fun getExtents(x11: X11Client, window: Int): PhysicalExtents {
        var left = PhysicalPixels.Zero
        var top = PhysicalPixels.Zero
        var right = PhysicalPixels.Zero
        var bottom = PhysicalPixels.Zero

        val gtkFrameExtents = getGtkFrameExtents(x11, window)

        if (gtkFrameExtents != null) {
            left += gtkFrameExtents.left
            top += gtkFrameExtents.top
            right += gtkFrameExtents.right
            bottom += gtkFrameExtents.bottom
        }

        val netWmExtents = getNetWmExtents(x11, window)
        log("netWmExtents=$netWmExtents")
        if (netWmExtents != null) {
            left -= netWmExtents.left
            top -= netWmExtents.top
            right += netWmExtents.right
            bottom += netWmExtents.bottom
        }

        return PhysicalExtents(
            top = top,
            left = left,
            right = right,
            bottom = bottom,
        )
    }

    fun windowSize(x11: X11Client, window: Int): PhysicalSize {
        val geometry = x11.send(GetGeometry.builder().drawable(window).build())
        val geometryWidth = geometry.width
        val geometryHeight = geometry.height
        log("XGetGeometry get size for $window: width=$geometryWidth, height=$geometryHeight")
        return PhysicalSize(
            width = PhysicalPixels(geometryWidth),
            height = PhysicalPixels(geometryHeight),
        )
    }

    fun clientAreaPosition(x11: X11Client, window: Int, transform: TransformData): GlobalPosition<PhysicalPixels> {
        val translated = x11.send(
            TranslateCoordinates
                .builder()
                .srcWindow(window)
                .dstWindow(x11.defaultRoot)
                .srcX(0)
                .srcY(0)
                .build(),
        )

        val geometryX = translated.dstX
        val geometryY = translated.dstY
        log("XTranslateCoordinates for $window: x=$geometryX, y=$geometryY")

        ProcessBuilder("xwininfo", "-id", window.toString()).start().inputReader().readLines().let {
            log(it.joinToString("\n"))
        }

        ProcessBuilder("xprop", "-id", window.toString()).start().inputReader().readLines().let {
            log(it.joinToString("\n"))
        }
        return GlobalPosition(
            x = PhysicalPixels(geometryX) + transform.x,
            y = PhysicalPixels(geometryY) + transform.y,
        )
    }

    fun screenshot(x11: X11Client, window: Int, x: Short, y: Short, width: Short, height: Short): org.jetbrains.skia.Image {
        val reply = x11.send(
            GetImage
                .builder()
                .drawable(window)
                .x(x)
                .y(y)
                .width(width)
                .height(height)
                .planeMask(0.inv())
                .format(2) // ZPixmap
                .build(),
        )

        val data = reply.data
        val rgbaBytes = when (reply.depth.toInt()) {
            32 -> {
                val rgbaBytes = ByteArray(data.size())

                if (x11.setup.imageByteOrder.toInt() == ImageOrder.M_S_B_FIRST.value) {
                    // Interpreting as ARGB
                    for (i in 0..<data.size() step 4) {
                        val a = data[i]
                        val r = data[i + 1]
                        val g = data[i + 2]
                        val b = data[i + 3]

                        rgbaBytes[i] = r
                        rgbaBytes[i + 1] = g
                        rgbaBytes[i + 2] = b
                        rgbaBytes[i + 3] = a
                    }
                } else {
                    // Interpreting as BGRA
                    for (i in 0..<data.size() step 4) {
                        val b = data[i]
                        val g = data[i + 1]
                        val r = data[i + 2]
                        val a = data[i + 3]

                        rgbaBytes[i] = r
                        rgbaBytes[i + 1] = g
                        rgbaBytes[i + 2] = b
                        rgbaBytes[i + 3] = a
                    }
                }
                rgbaBytes
            }
            24 -> {
                val rgbaBytes = ByteArray(data.size())

                if (x11.setup.imageByteOrder.toInt() == ImageOrder.M_S_B_FIRST.value) {
                    // Interpreting as ARGB
                    for (i in 0..<data.size() step 4) {
                        val r = data[i]
                        val g = data[i + 1]
                        val b = data[i + 2]

                        rgbaBytes[i] = r
                        rgbaBytes[i + 1] = g
                        rgbaBytes[i + 2] = b
                        rgbaBytes[i + 3] = Byte.MAX_VALUE
                    }
                } else {
                    // Interpreting as BGRA
                    for (i in 0..<data.size() step 4) {
                        val b = data[i]
                        val g = data[i + 1]
                        val r = data[i + 2]

                        rgbaBytes[i] = r
                        rgbaBytes[i + 1] = g
                        rgbaBytes[i + 2] = b
                        rgbaBytes[i + 3] = Byte.MAX_VALUE
                    }
                }
                rgbaBytes
            }
            else -> {
                fail("X11 GetImage: unexpected depth=${reply.depth}, visual=${reply.visual}, data size=${reply.data.size()}")
            }
        }

        return org.jetbrains.skia.Image.makeRaster(
            imageInfo = org.jetbrains.skia.ImageInfo(
                colorInfo = org.jetbrains.skia.ColorInfo(
                    colorType = org.jetbrains.skia.ColorType.RGBA_8888,
                    alphaType = org.jetbrains.skia.ColorAlphaType.OPAQUE,
                    colorSpace = null,
                ),
                width = width.toInt(),
                height = height.toInt(),
            ),
            rgbaBytes,
            rowBytes = width * 4,
        )
    }

    fun rootSize(x11: X11Client): Pair<Short, Short> {
        val root = x11.defaultRoot
        val geometry = x11.send(GetGeometry.builder().drawable(root).build())
        return Pair(geometry.width, geometry.height)
    }

    fun netClientList(x11: X11Client): List<Int>? {
        // https://specifications.freedesktop.org/wm/1.5/ar01s03.html#id-1.4.4
        val intBuffer = getRootWindowProperty(
            x11,
            propertyName = "_NET_CLIENT_LIST",
            type = Atom.WINDOW,
        )?.asIntBuffer() ?: return null
        val res = IntArray(intBuffer.remaining())
        intBuffer.get(res)
        log("netClientList: value=${res.contentToString()}")
        return res.asList()
    }

    fun isComposerActive(x11: X11Client, screen: Int): Boolean {
        // https://specifications.freedesktop.org/wm/1.5/ar01s08.html
        val atomName = "_NET_WM_CM_S$screen"
        val reply = x11.send(GetSelectionOwner.builder().selection(x11.getAtom(atomName).id).build())
        val owner = reply.owner
        return owner != 0
    }
}

internal class X11WindowOperations(
    private val x11: X11Client,
    private val window: Int,
    private val scale: Scale,
    private val getTransform: () -> Pair<LogicalPixels, LogicalPixels>,
) {
    private companion object {
        // https://github.com/tcltk/tk/blob/d2311aed9c6a4952a78e049cb7fe2fa414b9b705/unix/tkUnixWm.c#L5059-L5060
        const val NET_WM_STATE_REMOVE = 0
        const val NET_WM_STATE_ADD = 1

        private fun sendClientMessageToRootWindow(x11: X11Client, window: Int, type: String, vararg data32: Int) {
            val event = ClientMessageEvent
                .builder()
                .sentEvent(true)
                .window(window)
                .format(32)
                .type(x11.getAtom(type).id)
                .data(ClientMessageData32(*data32))
                .build()

            val saved = ByteArrayOutputStream()
            val out = if (x11.bigEndian) X11BigEndianOutputStream(saved) else X11LittleEndianOutputStream(saved)
            event.write(out)

            x11.send(
                SendEvent
                    .builder()
                    .destination(x11.defaultRoot)
                    .propagate(false)
                    .eventMaskEnable(EventMask.SUBSTRUCTURE_REDIRECT, EventMask.SUBSTRUCTURE_NOTIFY)
                    .event(Utilities.toList(saved.toByteArray()))
                    .build(),
            )
            x11.sync()
        }
    }

    private fun setFullScreen(action: Int) {
        // https://specifications.freedesktop.org/wm/1.5/ar01s05.html#id-1.6.8
        sendClientMessageToRootWindow(x11, window, "_NET_WM_STATE", action, x11.getAtom("_NET_WM_STATE_FULLSCREEN").id)
    }

    fun fullScreen() {
        setFullScreen(NET_WM_STATE_ADD)
    }

    fun unsetFullScreen() {
        setFullScreen(NET_WM_STATE_REMOVE)
    }

    fun close() {
        // https://specifications.freedesktop.org/wm/1.5/ar01s04.html#id-1.5.2
        sendClientMessageToRootWindow(x11, window, "_NET_CLOSE_WINDOW")
    }

    fun moveTo(pos: GlobalPosition<LogicalPixels>) {
        val x = logicalToPhysical(pos.x)
        val y = logicalToPhysical(pos.y)
        x11.send(ConfigureWindow.builder().window(window).x(x.rawPhysical).y(y.rawPhysical).build())
        x11.sync()
        waitUntilTrue("Window $window moved to $pos") {
            val actualPosition = framePosition()
            (actualPosition == pos).also {
                if (!it) {
                    log("moveTo: actualPosition ($actualPosition) != $pos")
                }
            }
        }
        log("Moved window $window to $x,$y")
    }

    private fun getActiveWindow(): Int? {
        // https://specifications.freedesktop.org/wm/1.5/ar01s03.html#id-1.4.10
        val intBuffer = X11Helper.getRootWindowProperty(
            x11,
            propertyName = "_NET_ACTIVE_WINDOW",
            type = Atom.WINDOW,
        )?.asIntBuffer() ?: return null
        val res = intBuffer.get()
        log("getActiveWindow: value=$res")
        return res
    }

    fun focus() {
        // https://specifications.freedesktop.org/wm/1.5/ar01s03.html#id-1.4.10
        sendClientMessageToRootWindow(x11, window, "_NET_ACTIVE_WINDOW")
        waitUntil("Window $window active", getter = { getActiveWindow() }) { it == window }
    }

    private fun rawPhysicalToLogical(rawPhysical: Int): LogicalPixels {
        return scale.rawPhysicalToLogical(rawPhysical.toDouble())
    }

    private fun rawPhysicalToLogicalInt(rawPhysical: Int): LogicalPixelsInt {
        return LogicalPixelsInt(scale.rawPhysicalToLogical(rawPhysical.toDouble()).rawLogical.roundToInt())
    }

    private fun physicalToLogicalInt(physical: PhysicalPixels): LogicalPixelsInt {
        return LogicalPixelsInt(scale.rawPhysicalToLogical(physical.rawPhysical.toDouble()).rawLogical.roundToInt())
    }

    private fun logicalToPhysical(logical: LogicalPixels): PhysicalPixels {
        return PhysicalPixels(logical.toRawPhysical(scale).roundToInt())
    }

    private fun logicalToPhysical(logical: LogicalPixelsInt): PhysicalPixels {
        return PhysicalPixels(logical.toRawPhysical(scale).roundToInt())
    }

    private fun getPhysicalTransform(): TransformData {
        val gtkFrameExtents = X11Helper.getGtkFrameExtents(x11, window)
        if (gtkFrameExtents != null) {
            return TransformData(x = gtkFrameExtents.left, y = gtkFrameExtents.top)
        }
        val (transformX, transformY) = getTransform()
        log("Window $window transform: x=$transformX, y=$transformY")

        ProcessBuilder("xwininfo", "-id", window.toString()).start().inputReader().readLines().let {
            log(it.joinToString("\n"))
        }

        ProcessBuilder("xprop", "-id", window.toString()).start().inputReader().readLines().let {
            log(it.joinToString("\n"))
        }
        return TransformData(x = logicalToPhysical(transformX), y = logicalToPhysical(transformY))
    }

    fun clientAreaPosition(): GlobalPosition<LogicalPixels> {
        val pos = X11Helper.clientAreaPosition(x11, window, getPhysicalTransform())
        return GlobalPosition(
            x = rawPhysicalToLogical(pos.x.rawPhysical),
            y = rawPhysicalToLogical(pos.y.rawPhysical),
        )
    }

    fun framePosition(): GlobalPosition<LogicalPixels> {
        val netWmExtents = X11Helper.getNetWmExtents(x11, window)
        val transform = if (netWmExtents == null) {
            TransformData(PhysicalPixels.Zero, PhysicalPixels.Zero)
        } else {
            TransformData(
                x = PhysicalPixels.Zero - netWmExtents.left,
                y = PhysicalPixels.Zero - netWmExtents.top,
            )
        }
        val pos = X11Helper.clientAreaPosition(x11, window, transform)
        return GlobalPosition(
            x = rawPhysicalToLogical(pos.x.rawPhysical),
            y = rawPhysicalToLogical(pos.y.rawPhysical),
        )
    }

    fun getMinimalSizeHint(): LogicalSize? {
        val intBuffer = X11Helper.getProperty(x11, window, Atom.WM_NORMAL_HINTS.value, Atom.WM_SIZE_HINTS)?.asIntBuffer() ?: return null
        val res = IntArray(intBuffer.remaining())
        intBuffer.get(res)
        log("WM_NORMAL_HINTS: ${res.contentToString()}")

        val minWidthProperty = PhysicalPixels(res[5])
        val minHeightProperty = PhysicalPixels(res[6])
        val frameExtents = X11Helper.getGtkFrameExtents(x11, window) ?: run {
            val (transformX, transformY) = getPhysicalTransform()
            PhysicalExtents(
                top = transformY,
                left = transformX,
                right = transformX,
                bottom = transformY,
            )
        }
        val minWidth = minWidthProperty - frameExtents.left - frameExtents.right
        val minHeight = minHeightProperty - frameExtents.top - frameExtents.bottom
        log("minWidth=$minWidth, minHeight=$minHeight")
        return if (minWidth.rawPhysical <= scale.rawValue && minHeight.rawPhysical <= scale.rawValue) {
            null
        } else {
            LogicalSize(
                width = physicalToLogicalInt(minWidth),
                height = physicalToLogicalInt(minHeight),
            )
        }
    }

    fun readTitle(): String? {
        return X11Helper.readNetWmName(x11, window)
    }

    fun screenshot(rect: LogicalRect): org.jetbrains.skia.Image {
        val x: Int = logicalToPhysical(rect.x).rawPhysical
        val y: Int = logicalToPhysical(rect.y).rawPhysical
        val width: Int = logicalToPhysical(rect.width).rawPhysical
        val height: Int = logicalToPhysical(rect.height).rawPhysical
        return X11Helper.screenshot(
            x11,
            window = window,
            x = x.toShort(),
            y = y.toShort(),
            width = width.toShort(),
            height = height.toShort(),
        )
    }
}

internal class X11Wm(private val scale: Scale) {
    companion object {
        // Used in "type" field in XEvent structures.
        // https://github.com/tcltk/tk/blob/d2311aed9c6a4952a78e049cb7fe2fa414b9b705/xlib/X11/X.h#L186-L193
        enum class EventType(val value: Byte) {
            KeyPress(2.toByte()),
            KeyRelease(3.toByte()),
            ButtonPress(4.toByte()),
            ButtonRelease(5.toByte()),
        }

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

    private val x11 = X11Client.connect()

    fun isMutter(): Boolean {
        return getWmName() == "Mutter"
    }

    fun getWmName(): String? {
        // https://specifications.freedesktop.org/wm/1.5/ar01s03.html#id-1.4.12
        val intBuffer = X11Helper.getRootWindowProperty(
            x11,
            propertyName = "_NET_SUPPORTING_WM_CHECK",
            type = Atom.WINDOW,
        )?.asIntBuffer() ?: return null
        val res = intBuffer.get()
        return X11Helper.readNetWmName(x11, res).also {
            log("wmName=$it")
        }
    }

    fun getVersion(): WmVersion {
        return getI3Version()?.let {
            WmVersion(
                name = "i3",
                major = it.major,
                minor = it.minor,
                patch = it.patch,
            )
        } ?: WmVersion(
            name = getWmName().orEmpty(),
            major = 0,
            minor = 0,
            patch = 0,
        )
    }

    private fun getWindowByTitleImpl(title: String, getTransform: () -> Pair<LogicalPixels, LogicalPixels>): X11WindowOperations? {
        val allWindows = X11Helper.netClientList(x11) ?: return null

        val windows = allWindows.filter { window ->
            val actualTitle = X11Helper.readNetWmName(x11, window)
            log("window $window title = \"$actualTitle\"")
            actualTitle == title
        }

        if (windows.isEmpty()) {
            return null
        }

        val window = windows.single()

        waitUntilTrue("Window $window has proper size") {
            val size = X11Helper.windowSize(x11, window)
            size.width.rawPhysical > scale.rawValue && size.height.rawPhysical > scale.rawValue
        }

        return X11WindowOperations(x11, window, scale, getTransform)
    }

    fun getWindowByTitle(title: String, getTransform: () -> Pair<LogicalPixels, LogicalPixels>): X11WindowOperations {
        return waitUntil(
            msg = {
                ProcessBuilder("xwininfo", "-root", "-tree").start().inputReader().readLines().let {
                    "getWindowByTitle($title) failed: ${it.joinToString("\n")}"
                }
            },
            getter = { getWindowByTitleImpl(title, getTransform) },
        ) { it != null }!!
    }

    private fun sendFakeInput(detail: Byte, eventType: EventType) {
        x11.send(
            FakeInput
                .builder()
                .detail(detail)
                .type(eventType.value)
                .time(0)
                .build(),
        )
        x11.sync()
    }

    fun <T> withKeyPress(key: UInt, block: () -> T): T {
        log("Key down: $key")
        sendFakeInput(key.toByte(), EventType.KeyPress)
        AutoCloseable {
            log("Key up: $key")
            sendFakeInput(key.toByte(), EventType.KeyRelease)
        }.use {
            return block()
        }
    }

    fun <T> withMouseButtonDown(button: MouseButton, block: () -> T): T {
        log("Mouse button down: $button")
        sendFakeInput(button.value.toByte(), EventType.ButtonPress)
        AutoCloseable {
            log("Mouse button up: $button")
            sendFakeInput(button.value.toByte(), EventType.ButtonRelease)
        }.use {
            return block()
        }
    }

    private fun moveMouseToImpl(x: Short, y: Short) {
        X11Helper.moveMouseTo(x11, x = x, y = y)

        waitUntil("Mouse moved to $x, $y", getter = { X11Helper.getMousePosition(x11) }) { it.first == x && it.second == y }
        log("wm.moveMouseTo $x, $y done")

        if (isMutter()) {
            Thread.sleep(100)
        }
    }

    fun moveMouseTo(pos: TestMousePosition) {
        val x = (pos.base.x + pos.offsetX).toRawPhysical(scale).roundToInt().toShort()
        val y = (pos.base.y + pos.offsetY).toRawPhysical(scale).roundToInt().toShort()
        moveMouseToImpl(x = x, y = y)
    }

    fun resetMousePosition() {
        moveMouseToImpl(x = 1, y = 1)
    }

    fun scrollMouseUp() {
        // https://askubuntu.com/a/1162351
        sendFakeInput(4, EventType.ButtonPress)
        sendFakeInput(4, EventType.ButtonRelease)
    }

    fun scrollMouseDown() {
        // https://askubuntu.com/a/1162351
        sendFakeInput(5, EventType.ButtonPress)
        sendFakeInput(5, EventType.ButtonRelease)
    }

    fun isComposerActive(): Boolean {
        return X11Helper.isComposerActive(x11, screen = 0)
    }
}
