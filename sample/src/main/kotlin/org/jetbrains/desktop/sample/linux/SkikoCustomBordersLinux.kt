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
        rectangles.clear()

        rectangles.add(
            Pair(
                LogicalRect(
                    x = LogicalPixelsInt.Zero,
                    y = LogicalPixelsInt.Zero,
                    width = frame.left.padding,
                    height = frame.top.padding,
                ),
                WindowResizeEdge.TopLeft,
            ),
        )
        rectangles.add(
            Pair(
                LogicalRect(
                    x = w - frame.right.padding,
                    y = LogicalPixelsInt.Zero,
                    width = frame.right.padding,
                    height = frame.top.padding,
                ),
                WindowResizeEdge.TopRight,
            ),
        )
        rectangles.add(
            Pair(
                LogicalRect(
                    x = LogicalPixelsInt.Zero,
                    y = h - frame.bottom.padding,
                    width = frame.left.padding,
                    height = frame.bottom.padding,
                ),
                WindowResizeEdge.BottomLeft,
            ),
        )
        rectangles.add(
            Pair(
                LogicalRect(
                    x = w - frame.right.padding,
                    y = h - frame.bottom.padding,
                    width = frame.right.padding,
                    height = frame.bottom.padding,
                ),
                WindowResizeEdge.BottomRight,
            ),
        )

        rectangles.add(
            Pair(
                LogicalRect(
                    x = LogicalPixelsInt.Zero,
                    y = LogicalPixelsInt.Zero,
                    width = frame.left.padding,
                    height = h,
                ),
                WindowResizeEdge.Left,
            ),
        )
        rectangles.add(
            Pair(
                LogicalRect(
                    x = w - frame.right.padding,
                    y = LogicalPixelsInt.Zero,
                    width = frame.right.padding,
                    height = h,
                ),
                WindowResizeEdge.Right,
            ),
        )
        rectangles.add(
            Pair(
                LogicalRect(x = LogicalPixelsInt.Zero, y = LogicalPixelsInt.Zero, width = w, height = frame.top.padding),
                WindowResizeEdge.Top,
            ),
        )
        rectangles.add(
            Pair(
                LogicalRect(
                    x = LogicalPixelsInt.Zero,
                    y = h - frame.bottom.padding,
                    width = w,
                    height = frame.bottom.padding,
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
            window.startResize(edge)
            EventHandlerResult.Stop
        } else {
            EventHandlerResult.Continue
        }
    }
}
