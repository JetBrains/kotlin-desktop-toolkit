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
        const val BORDER_SIZE_LEFT: LogicalPixels = 5.0
        const val BORDER_SIZE_RIGHT: LogicalPixels = 5.0
        const val BORDER_SIZE_TOP: LogicalPixels = 5.0
        const val BORDER_SIZE_BOTTOM: LogicalPixels = 5.0

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
        val size = event.size
        rectangles.clear()
        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = 0.0,
                    y = 0.0,
                    width = BORDER_SIZE_LEFT,
                    height = BORDER_SIZE_TOP,
                ),
                WindowResizeEdge.TopLeft,
            ),
        )
        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = size.width - BORDER_SIZE_RIGHT,
                    y = 0.0,
                    width = BORDER_SIZE_RIGHT,
                    height = BORDER_SIZE_TOP,
                ),
                WindowResizeEdge.TopRight,
            ),
        )
        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = 0.0,
                    y = size.height - BORDER_SIZE_BOTTOM,
                    width = BORDER_SIZE_LEFT,
                    height = BORDER_SIZE_BOTTOM,
                ),
                WindowResizeEdge.BottomLeft,
            ),
        )
        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = size.width - BORDER_SIZE_RIGHT,
                    y = size.height - BORDER_SIZE_BOTTOM,
                    width = BORDER_SIZE_RIGHT,
                    height = BORDER_SIZE_BOTTOM,
                ),
                WindowResizeEdge.BottomRight,
            ),
        )

        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = 0.0,
                    y = 0.0,
                    width = BORDER_SIZE_LEFT,
                    height = size.height.toDouble(),
                ),
                WindowResizeEdge.Left,
            ),
        )
        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = size.width - BORDER_SIZE_RIGHT,
                    y = 0.0,
                    width = BORDER_SIZE_RIGHT,
                    height = size.height.toDouble(),
                ),
                WindowResizeEdge.Right,
            ),
        )
        rectangles.add(
            Pair(
                LogicalDoubleRect(x = 0.0, y = 0.0, width = size.width.toDouble(), height = BORDER_SIZE_TOP),
                WindowResizeEdge.Top,
            ),
        )
        rectangles.add(
            Pair(
                LogicalDoubleRect(
                    x = 0.0,
                    y = size.height - BORDER_SIZE_BOTTOM,
                    width = size.width.toDouble(),
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
