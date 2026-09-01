package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.LogicalPixelsInt
import org.jetbrains.desktop.linux.LogicalPoint
import org.jetbrains.desktop.linux.LogicalRect
import org.jetbrains.desktop.linux.LogicalSize
import org.jetbrains.desktop.linux.PointerShape
import org.jetbrains.desktop.linux.Window
import org.jetbrains.desktop.linux.WindowFrame
import org.jetbrains.desktop.linux.WindowResizeEdge

internal class SkikoCustomBordersLinux {
    companion object {
        fun edgeToPointerShape(edge: WindowResizeEdge): PointerShape {
            return when (edge) {
                WindowResizeEdge.Top -> PointerShape.NResize
                WindowResizeEdge.Bottom -> PointerShape.SResize
                WindowResizeEdge.Left -> PointerShape.WResize
                WindowResizeEdge.TopLeft -> PointerShape.NwResize
                WindowResizeEdge.BottomLeft -> PointerShape.SwResize
                WindowResizeEdge.Right -> PointerShape.EResize
                WindowResizeEdge.TopRight -> PointerShape.NeResize
                WindowResizeEdge.BottomRight -> PointerShape.SeResize
            }
        }
    }

    private var rectangles = ArrayList<Pair<LogicalRect, WindowResizeEdge>>()

    fun configure(size: LogicalSize, frame: WindowFrame) {
        val w = size.width
        val h = size.height
        val resizerThickness = frame.resizerThickness
        rectangles.clear()

        rectangles.add(
            Pair(
                LogicalRect(
                    x = LogicalPixelsInt.Zero,
                    y = LogicalPixelsInt.Zero,
                    width = resizerThickness.left,
                    height = resizerThickness.top,
                ),
                WindowResizeEdge.TopLeft,
            ),
        )
        rectangles.add(
            Pair(
                LogicalRect(
                    x = w - resizerThickness.right,
                    y = LogicalPixelsInt.Zero,
                    width = resizerThickness.right,
                    height = resizerThickness.top,
                ),
                WindowResizeEdge.TopRight,
            ),
        )
        rectangles.add(
            Pair(
                LogicalRect(
                    x = LogicalPixelsInt.Zero,
                    y = h - resizerThickness.bottom,
                    width = resizerThickness.left,
                    height = resizerThickness.bottom,
                ),
                WindowResizeEdge.BottomLeft,
            ),
        )
        rectangles.add(
            Pair(
                LogicalRect(
                    x = w - resizerThickness.right,
                    y = h - resizerThickness.bottom,
                    width = resizerThickness.right,
                    height = resizerThickness.bottom,
                ),
                WindowResizeEdge.BottomRight,
            ),
        )

        rectangles.add(
            Pair(
                LogicalRect(
                    x = LogicalPixelsInt.Zero,
                    y = LogicalPixelsInt.Zero,
                    width = resizerThickness.left,
                    height = h,
                ),
                WindowResizeEdge.Left,
            ),
        )
        rectangles.add(
            Pair(
                LogicalRect(
                    x = w - resizerThickness.right,
                    y = LogicalPixelsInt.Zero,
                    width = resizerThickness.right,
                    height = h,
                ),
                WindowResizeEdge.Right,
            ),
        )
        rectangles.add(
            Pair(
                LogicalRect(x = LogicalPixelsInt.Zero, y = LogicalPixelsInt.Zero, width = w, height = resizerThickness.top),
                WindowResizeEdge.Top,
            ),
        )
        rectangles.add(
            Pair(
                LogicalRect(
                    x = LogicalPixelsInt.Zero,
                    y = h - resizerThickness.bottom,
                    width = w,
                    height = resizerThickness.bottom,
                ),
                WindowResizeEdge.Bottom,
            ),
        )
    }

    fun toEdge(locationInWindow: LogicalPoint): WindowResizeEdge? {
        for ((rect, edge) in rectangles) {
            if (rect.contains(locationInWindow)) {
                return edge
            }
        }
        return null
    }

    fun onMouseDown(event: Event.MouseDown, window: Window): EventHandlerResult {
        val edge = toEdge(event.locationInWindow)
        return if (edge != null) {
            window.startResize(event.serial, edge)
            EventHandlerResult.Stop
        } else {
            EventHandlerResult.Continue
        }
    }
}
