package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.LogicalPixels
import org.jetbrains.desktop.linux.LogicalPoint
import org.jetbrains.desktop.linux.PointerShape
import org.jetbrains.desktop.linux.Window
import org.jetbrains.desktop.linux.WindowResizeEdge

internal class SkikoCustomBordersLinux {
    companion object {
        val BORDER_SIZE_LEFT: LogicalPixels = LogicalPixels(5.0)
        val BORDER_SIZE_RIGHT: LogicalPixels = LogicalPixels(5.0)
        val BORDER_SIZE_TOP: LogicalPixels = LogicalPixels(5.0)
        val BORDER_SIZE_BOTTOM: LogicalPixels = LogicalPixels(5.0)

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

    private var rectangles = ArrayList<Pair<LogicalDoubleRect, WindowResizeEdge>>()

    fun configure(event: Event.WindowConfigure) {
        val w = event.size.width.toLogicalPixels()
        val h = event.size.height.toLogicalPixels()

        rectangles.clear()
        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = LogicalPixels.Zero,
                    y = LogicalPixels.Zero,
                    width = BORDER_SIZE_LEFT,
                    height = BORDER_SIZE_TOP,
                ),
                WindowResizeEdge.TopLeft,
            ),
        )
        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = w - BORDER_SIZE_RIGHT,
                    y = LogicalPixels.Zero,
                    width = BORDER_SIZE_RIGHT,
                    height = BORDER_SIZE_TOP,
                ),
                WindowResizeEdge.TopRight,
            ),
        )
        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = LogicalPixels.Zero,
                    y = h - BORDER_SIZE_BOTTOM,
                    width = BORDER_SIZE_LEFT,
                    height = BORDER_SIZE_BOTTOM,
                ),
                WindowResizeEdge.BottomLeft,
            ),
        )
        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = w - BORDER_SIZE_RIGHT,
                    y = h - BORDER_SIZE_BOTTOM,
                    width = BORDER_SIZE_RIGHT,
                    height = BORDER_SIZE_BOTTOM,
                ),
                WindowResizeEdge.BottomRight,
            ),
        )

        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = LogicalPixels.Zero,
                    y = LogicalPixels.Zero,
                    width = BORDER_SIZE_LEFT,
                    height = h,
                ),
                WindowResizeEdge.Left,
            ),
        )
        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = w - BORDER_SIZE_RIGHT,
                    y = LogicalPixels.Zero,
                    width = BORDER_SIZE_RIGHT,
                    height = h,
                ),
                WindowResizeEdge.Right,
            ),
        )
        rectangles.add(
            Pair(
                LogicalDoubleRect(x = LogicalPixels.Zero, y = LogicalPixels.Zero, width = w, height = BORDER_SIZE_TOP),
                WindowResizeEdge.Top,
            ),
        )
        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = LogicalPixels.Zero,
                    y = h - BORDER_SIZE_BOTTOM,
                    width = w,
                    height = BORDER_SIZE_BOTTOM,
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
