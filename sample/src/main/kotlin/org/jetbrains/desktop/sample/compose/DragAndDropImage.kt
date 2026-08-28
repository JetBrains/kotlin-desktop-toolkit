package org.jetbrains.desktop.sample.compose

import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Canvas
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asSkiaBitmap
import androidx.compose.ui.graphics.drawscope.CanvasDrawScope
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.LayoutDirection
import kotlin.math.roundToInt
import org.jetbrains.skia.EncodedImageFormat
import org.jetbrains.skia.Image

internal class DragAndDropImage(
    private val size: Size,
    private val density: Density,
    private val layoutDirection: LayoutDirection,
    private val drawDragDecoration: DrawScope.() -> Unit,
) {

    /**
     * The decoration is drawn at [size]; [canvasSize] can be larger to pad the bitmap, leaving the
     * extra edge transparent. On macOS this is used to round the canvas up to even pixels, which
     * works around blurry drag images (AIR-5862).
     */
    internal fun encodeToPng(canvasSize: Size = size): ByteArray? {
        val imageBitmap = ImageBitmap(
            width = canvasSize.width.roundToInt(),
            height = canvasSize.height.roundToInt()
        )
        val canvas = Canvas(imageBitmap)
        val canvasScope = CanvasDrawScope()
        canvasScope.draw(density, layoutDirection, canvas, size, drawDragDecoration)
        return imageBitmap.encodeToBytes(EncodedImageFormat.PNG, 100)
    }

    private fun ImageBitmap.encodeToBytes(format: EncodedImageFormat, quality: Int): ByteArray? {
        return Image.makeFromBitmap(this.asSkiaBitmap()).encodeToData(format, quality)?.bytes
    }
}
