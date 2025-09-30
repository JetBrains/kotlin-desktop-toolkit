[![internal JetBrains project](https://jb.gg/badges/internal.svg)](https://confluence.jetbrains.com/display/ALL/JetBrains+on+GitHub)
# Kotlin Window Management library


## Goals
* Provide a simple enough Kotlin API for interacting with OS
* Support all major desktop platforms: Linux, MacOS and Windows for now
* Make it Kotlin Multiplatform in future

## Non-Goals
* Providing bindings for Android or iOS. It's too different from Desktop
* Bindings for browser API
* API alignment across platforms. Desktop platforms are different and provide different capabilities, we are not going to hide this difference. Though for some common APIs we could provide a layer that simplifies usage of it across the platforms

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
| List screens          | ✅    | ✅      | 🚧      |     |
| Terminate application | ✅    | ✅      | ✅      |     |
| Show notification     | ❌    | ❌      | ❌      |     |
| System tray           | ❌    | ❌      | ❌      |     |
| Application icon      | ✅    | ❌      | 🚧      |     |
| Application menu      | ✅    | ❌      | ➖      |     |
| Accessibility         | ❌    | ❌      | ❌      |     |
| File choose dialog    | ✅    | ❌      | 🚧      |     |

#### Window

|                    | MacOS | Wayland | Windows | X11 |
| ------------------ | ----- | ------- | ------- | --- |
| Position           | ✅    | ➖ (`startMove`)   | ✅      |     |
| Size               | ✅    | ➖ (`startResize`) | ✅      |     |
| Max/Min size       | ✅    | ✅      | ✅      |     |
| Content size       | ✅    | ➖      | 🚧      |     |
| Current screen     | ✅    | ✅      | 🚧      |     |
| Full screen        | ✅    | ✅      | ❌      |     |
| Maximize/Minimize  | ✅    | ✅      | 🚧      |     |
| Request focus      | ✅    | ❌      | 🚧      |     |
| Set cursor icon    | ✅    | ✅      | 🚧      |     |
| Transparency       | ✅    | ✅      | ✅      |     |
| Background effects | ✅    | ❌      | ✅      |     |
| Close window       | ✅    |         | 🚧      |     |

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
| ApplicationOpenURL            | ✅    | ✅      | ➖      |     |
| ApplicationAppearanceChange   | ✅    | ✅      | ❌      |     |
| ApplicationDidFinishLaunching | ✅    | ✅      | ✅*     |     |

\* Currently on Windows, we can enqueue a callback before starting the event loop. We probably want to replace this logic with an `ApplicationDidFinishLaunching` event.

#### Theme

|                | MacOS | Wayland | Windows | X11 |
| -------------- | ----- | ------- | ------- | --- |
| isDark/isLight | ✅    | ✅      | ❌      |     |
| Sync with OS   | ✅    | ✅      | ❌      |     |

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
| System defined clipboard types  | 🚧    | ➖      | ❌      |     |
| Custom binary clipboard content | ✅    | ✅      | ❌      |     |
| Lazy fetch of clipboard content | ❌    | ✅      | ❌      |     |

#### Screen

|                     | MacOS | Wayland | Windows | X11 |
| ------------------- | ----- | ------- | ------- | --- |
| ScreenId            | ✅    | ✅      | 🚧      |     |
| IsPrimary           | ✅    | ❌      | 🚧      |     |
| Name                | ✅    | ✅      | 🚧      |     |
| Origin              | ✅    | ✅      | 🚧      |     |
| Size                | ✅    | ✅      | 🚧      |     |
| Scale               | ✅    | ✅      | 🚧      |     |
| FPS                 | ✅    | ✅      | 🚧      |     |
| Color space         | ❌    | ❌      | 🚧      |     |
| Persistent identity | ❌    | 🚧      | 🚧      |     |



#### Drag and Drop

|                       | MacOS | Wayland | Windows | X11 |
| --------------------- | ----- | ------- | ------- | --- |
| Window as drag target | ✅    | ✅      | ❌      |     |
| Drag entered          | ✅    | ✅      | ❌      |     |
| Drag updated          | ✅    | ✅      | ❌      |     |
| Drag exited           | ✅    | ❌      | ❌      |     |
| Drag performed        | ✅    | 🚧      | ❌      |     |
| Drag source           | ❌    | ✅      | ❌      |     |
