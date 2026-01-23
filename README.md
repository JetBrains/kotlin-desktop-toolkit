[![internal JetBrains project](https://jb.gg/badges/internal.svg)](https://confluence.jetbrains.com/display/ALL/JetBrains+on+GitHub)
# Kotlin Window Management library

Kotlin Window Management is a library that wraps OS-specific window management APIs into an idiomatic Kotlin interface.

The library serves as a foundation for the UI framework used in [Air](https://air.dev/) and will later provide
an OS integration layer for Compose for Desktop.

## Goals
* Provide a simple Kotlin API for OS features needed to build desktop applications
* Support all major desktop platforms: Linux (both X11 and Wayland), macOS and Windows
* Flatten OS quirks or at least document them
* Provide a safe API. Incorrect usage of the API should lead to Kotlin exceptions but not crashes
* Provide rendering contexts compatible with Skia, e.g., via [skiko](https://github.com/jetbrains/skiko)
* Make it Kotlin Multiplatform in the future

## Non-Goals
* Providing bindings for Android or iOS, they are too different from desktop
* Bindings for browser APIs
* API alignment across platforms. Desktop platforms differ in their capabilities; we do not intend to hide these differences

## Status

✅ - implemented

🚧 - in progress, partially implemented

❌ - not implemented yet

➖ - not applicable

#### Application

|                       | MacOS | Wayland | Windows | X11 |
| --------------------- | ----- | ------- | ------- | --- |
| Run event loop        | ✅    | ✅      | ✅      |     |
| Invoke on Main thread | ✅    | ✅      | ✅      |     |
| List screens          | ✅    | ✅      | ✅      |     |
| Terminate application | ✅    | ✅      | ✅      |     |
| Show notification     | ✅    | ✅      | ❌      |     |
| System tray           | ❌    | ❌      | ❌      |     |
| Application icon      | ✅    | ✅      | 🚧      |     |
| Application menu      | ✅    | ❌      | ➖      |     |
| Accessibility         | ❌    | ❌      | ❌      |     |
| File choose dialog    | ✅    | ✅      | 🚧      |     |

#### Window

|                    | MacOS | Wayland            | Windows | X11 |
| ------------------ | ----- | ------------------ | ------- | --- |
| Position           | ✅    | ➖ (`startMove`)   | ✅      |     |
| Size               | ✅    | ➖ (`startResize`) | ✅      |     |
| Max/Min size       | ✅    | ✅                 | ✅      |     |
| Content size       | ✅    | ➖                 | ✅      |     |
| Current screen     | ✅    | ✅                 | ✅      |     |
| Full screen        | ✅    | ✅                 | ❌      |     |
| Maximize/Minimize  | ✅    | ✅                 | ✅      |     |
| Request focus      | ✅    | ✅                 | 🚧      |     |
| Set cursor icon    | ✅    | ✅                 | ✅      |     |
| Transparency       | ✅    | ✅                 | ✅      |     |
| Background effects | ✅    | ❌                 | ✅      |     |
| Close window       | ✅    | ✅                 | ✅      |     |

#### Rendering

|           | MacOS | Wayland | Windows  | X11 |
| --------- | ----- | ------- | -------- | --- |
| Metal     | ✅    | ➖      | ➖       | ➖   |
| ANGLE     | ❌    | ❌      | ✅(DX11) | ❌   |
| DirectX12 | ➖    | ➖      | ❌       | ➖   |
| OpenGL    | ➖    | ✅      | ❌       | ❌   |
| Vulkan    | ❌    | ❌      | ❌       | ❌   |
| Software  | ❌    | ✅      | ✅(WARP) | ❌   |

#### Events

|                               | MacOS | Wayland | Windows | X11 |
| ----------------------------- | ----- | ------- | ------- | --- |
| KeyDown                       | ✅    | ✅      | ✅      |     |
| KeyUp                         | ✅    | ✅      | ✅      |     |
| ModifiersChanged              | ✅    | ✅      | ➖      |     |
| MouseMoved                    | ✅    | ✅      | ✅      |     |
| MouseDragged                  | ✅    | ❌      | ❌      |     |
| MouseEntered                  | ✅    | ✅      | ✅      |     |
| MouseExited                   | ✅    | ✅      | ✅      |     |
| MouseDown                     | ✅    | ✅      | ✅      |     |
| MouseUp                       | ✅    | ✅      | ✅      |     |
| ScrollWheel                   | ✅    | ✅      | ✅      |     |
| WindowSizeChange              | ✅    | ✅      | ✅      |     |
| WindowResize                  | ✅    | ✅      | ✅      |     |
| WindowMove                    | ✅    | ➖      | ✅      |     |
| WindowFocusChange             | ✅    | ✅      | ✅      |     |
| WindowCloseRequest            | ✅    | ✅      | ✅      |     |
| WindowFullScreenToggle        | ✅    | ✅      | ❌      |     |
| WindowChangedOcclusionState   | ✅    | ❌      | 🚧      |     |
| DisplayConfigurationChange    | ✅    | ✅      | 🚧      |     |
| ApplicationOpenURL            | ✅    | ✅      | ✅      |     |
| ApplicationAppearanceChange   | ✅    | ✅      | ✅      |     |
| ApplicationDidFinishLaunching | ✅    | ✅      | ✅*     |     |

\* On Windows, a callback is currently enqueued before starting the event loop. This may be replaced with an `ApplicationDidFinishLaunching` event in the future.

#### Theme

|                | MacOS | Wayland | Windows | X11 |
| -------------- | ----- | ------- | ------- | --- |
| isDark/isLight | ✅    | ✅      | ✅      |     |
| Sync with OS   | ✅    | ✅      | ✅      |     |

#### Input Methods

|                           | MacOS | Wayland | Windows | X11 |
| ------------------------- | ----- | ------- | ------- | --- |
| Custom text input context | ✅    | ✅      | ❌      |     |
| Order emoji popup         | ✅    | ➖      | ❌      |     |

#### Clipboard

|                                 | MacOS | Wayland | Windows | X11 |
| ------------------------------- | ----- | ------- | ------- | --- |
| Simple text copy/paste          | ✅    | ✅      | 🚧      |     |
| Copy files                      | ✅    | ✅      | 🚧      |     |
| System defined clipboard types  | ✅    | ➖      | ❌      |     |
| Custom binary clipboard content | ✅    | ✅      | ❌      |     |
| Lazy fetch of clipboard content | ❌    | ✅      | ❌      |     |

#### Screen

|                     | MacOS | Wayland | Windows | X11 |
| ------------------- | ----- | ------- | ------- | --- |
| ScreenId            | ✅    | ✅      | ➖      |     |
| IsPrimary           | ✅    | ❌      | ✅      |     |
| Name                | ✅    | ✅      | ✅      |     |
| Origin              | ✅    | ✅      | ✅      |     |
| Size                | ✅    | ✅      | ✅      |     |
| Scale               | ✅    | ✅      | ✅      |     |
| FPS                 | ✅    | ✅      | ✅      |     |
| Color space         | ❌    | ❌      | ❌      |     |
| Persistent identity | ✅    | 🚧      | ❌      |     |



#### Drag and Drop

|                       | MacOS | Wayland | Windows | X11 |
| --------------------- | ----- | ------- | ------- | --- |
| Window as drag target | ✅    | ✅      | ❌      |     |
| Drag entered          | ✅    | ✅      | ❌      |     |
| Drag updated          | ✅    | ✅      | ❌      |     |
| Drag exited           | ✅    | ✅      | ❌      |     |
| Drag performed        | ✅    | ✅      | ❌      |     |
| Drag source           | ✅    | ✅      | ❌      |     |

## Acknowledgements
Many libraries tackle the same problem from different angles, but each makes slightly different trade-offs compared to KDT.

To name a few:
* [AWT](https://docs.oracle.com/javase/8/docs/api/java/awt/package-summary.html)
* [gpui.rs](https://www.gpui.rs/)
* [JWM](https://github.com/humbleui/jwm)
* [JavaFX](https://openjfx.io/)
* [GLFW](https://www.glfw.org/)
* [SDL](https://www.libsdl.org/)
* [Electron](https://www.electronjs.org/)