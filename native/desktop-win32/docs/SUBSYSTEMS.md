# Subsystems

Per-subsystem reference. Each entry describes purpose, files, public API, key types, ownership/threading specifics, gotchas, and cross-references. Conventions and patterns common to all subsystems live in `ARCHITECTURE.md` and `FFI_CONVENTIONS.md`.

## Index

- [Application & event loop](#application--event-loop)
- [Window](#window)
- [Renderer (ANGLE)](#renderer-angle)
- [Geometry](#geometry)
- [Keyboard](#keyboard)
- [Pointer](#pointer)
- [Cursor](#cursor)
- [Screen](#screen)
- [Appearance](#appearance)
- [Clipboard](#clipboard)
- [DataObject](#dataobject)
- [DataReader](#datareader)
- [DataFormat / data_transfer](#dataformat--data_transfer)
- [Drag-and-drop](#drag-and-drop)
- [Global data (HGLOBAL)](#global-data-hglobal)
- [COM helpers](#com-helpers)
- [File dialog](#file-dialog)
- [Plumbing](#plumbing)

---

## Application & event loop

**Purpose.** UI-thread runtime for the whole toolkit. Initialises the OLE STA, creates a WinRT `DispatcherQueue` and a `CompositorController`, then runs a `GetMessage`/`DispatchMessage` pump. Translates raw `WM_*` messages to typed `Event` variants and forwards to a single Kotlin-supplied callback.

**Files.** `application.rs`, `application_api.rs`, `event_loop.rs`, `events.rs`, `events_api.rs` + Kotlin `Application.kt`, `Event.kt`.

**FFI surface (`application_api.rs`, `events_api.rs`).**
`application_init_apartment`, `application_init`, `application_run_event_loop`, `application_stop_event_loop`, `application_dispatcher_invoke`, `application_is_dispatcher_thread`, `application_open_url`, `application_drop`; `keyevent_translate_message`, `keydown_to_unicode`.

**Kotlin surface.** `Application` (`AutoCloseable`) — `runEventLoop`, `stopEventLoop`, `invokeOnDispatcher`, `onStartup`, `isDispatcherThread`, `newWindow`, `createAngleRenderer`. Companion extension `openURL`. `Event` `sealed class` with 22 subclasses 1-to-1 with the Rust enum.

**Key types.**
- `Application` (Rust) — owns `Rc<EventLoop>`, the `DispatcherQueueController`, `DispatcherQueue`, `CompositorController`. Heap-boxed; opaque to Kotlin as `AppPtr<'a> = RustAllocatedRawPtr<'a>` (alias).
- `EventLoop` — the message pump. `Window` keeps `Weak<EventLoop>` to avoid cycles.
- `Event` — `#[repr(C)]` enum, 22 variants. Mirrored 1-to-1 in Kotlin. Has `#[allow(dead_code)]` on the whole enum because variants are constructed by Rust but consumed only across FFI.
- `EventHandler = extern "C" fn(WindowId, &Event) -> bool` (returns "handled?")

**Ownership.** `application_init` heap-boxes the `Application` and returns the leaked pointer; `application_drop` reclaims. The `borrow::<Application>` path on each call leak-reconstructs the box (see `FFI_CONVENTIONS.md` → opaque pointers). Per-`WindowTitleChangedEvent` strings are `AutoDropStrPtr`, owned by the `Event` for the duration of the callback.

**Threading.** Everything UI-thread. `OleInitialize(None)` at `application.rs:31`. `DispatcherQueueController::CreateOnDedicatedThread` is **not** used — `DQTYPE_THREAD_CURRENT` ties the queue to the calling thread. `KEYEVENT_MESSAGES` and `LAST_KEYEVENT_MESSAGE_ID` are `thread_local!` (event_loop.rs:43-46). `LAST_EXCEPTION_MSGS` is also thread-local — errors on background threads are silently lost (see `ARCHITECTURE.md`).

**Gotchas.**
- `application_dispatcher_invoke` returns a `bool` (enqueue succeeded?) — `Application.invokeOnDispatcher` discards it. Enqueues after dispatcher shutdown silently lose the lambda.
- `EnableMouseInPointer(true)` (event_loop.rs:55) is process-wide and irreversible — third-party libs in the same process expecting raw `WM_MOUSE*` will silently break.
- `WM_NCPOINTERUP` always returns `None` (event_loop.rs:522-523) so window buttons keep working — the Kotlin `EventHandlerResult` from a non-client `PointerUp` is ignored.
- `WM_POINTERUPDATE` and `WM_POINTERDOWN` can both emit `Event::PointerDown` for the same gesture (button press inside an update message + dedicated down handler) → possible duplicate events. See TODO.md.
- Two commented-out variants in `events.rs:31`: `//WindowFocusChange`, `//WindowFullScreenToggle` — stale scaffolding.

**Cross-refs.** `geometry` (event payloads), `keyboard`, `pointer`, `appearance` (`SystemAppearanceChangeEvent` on `WM_SETTINGCHANGE`), `window`, `renderer_angle` (factory), `com.rs` (transitively via OLE init), `desktop_common::ffi_utils` (pointer wrappers), `desktop_common::logger` (error/FFI boundary).

---

## Window

**Purpose.** Top-level window (`WS_EX_NOREDIRECTIONBITMAP`) with a WinRT composition tree, DWM extended-frame effects (Mica/Acrylic/dark mode/custom titlebar), per-monitor DPI awareness, and per-window cursor. Composition: `Windows.UI.Composition` via `ICompositorDesktopInterop::CreateDesktopWindowTarget` (not DirectComposition — see `ARCHITECTURE.md`).

**Files.** `window.rs`, `window_api.rs` + Kotlin `Window.kt`.

**FFI surface.** `window_new`, `window_create`, `window_drop`, `window_destroy`, `window_show`, `window_maximize`, `window_minimize`, `window_request_redraw`, `window_request_close`, `window_get_client_size`, `window_get_rect`, `window_get_scale_factor`, `window_get_screen_info`, `window_is_maximized`, `window_is_minimized`, `window_set_backdrop_tint`, `window_remove_backdrop_tint`, `window_set_cursor_from_file`, `window_set_cursor_from_system`, `window_set_icon`, `window_set_immersive_dark_mode`, `window_set_min_size`, `window_set_title`, `window_set_rect`. **No `window_restore`** — there's currently no way to un-maximise a window through the API.

**Key types.**
- `Window` — `Rc<Window>` on Rust side. Holds `HWND` (via `AtomicPtr` set during `WM_NCCREATE`), `Weak<EventLoop>`, `CompositorController`, `composition_target: RefCell<Option<DesktopWindowTarget>>`, root `ContainerVisual`, backdrop `SpriteVisual`, cursor `RefCell<Option<Cursor>>`, `PointerClickCounter`. Opaque to Kotlin as `WindowPtr<'a> = RustAllocatedRcPtr<'a>` (alias).
- `WindowParams`, `WindowStyle`, `WindowTitleBarKind`, `WindowSystemBackdropType` — `#[repr(C)]` enums/structs.

**Ownership.** `window_new` does `Rc::new` → `Rc::into_raw`. `CreateWindowExW` is called with `lpCreateParams = Rc::downgrade(window).into_raw()`. In `WM_NCCREATE`, the `Weak` is reconstructed, upgraded, used to call `initialize_window`, then re-serialised via `Weak::into_raw` and stored as a Win32 window property under `KDT_WINDOW_PTR` (window.rs:473-479). On every wndproc message, `GetPropW` retrieves this raw pointer and wraps it in `ManuallyDrop::new(Weak::from_raw(...))` to avoid dropping the weak per message (window.rs:450). On `WM_NCDESTROY`, `RemovePropW` recovers the raw pointer and the weak is dropped. `window_drop` calls `to_rc::<Window>()` → `Rc::from_raw` → drop.

**Threading.** Single UI thread (the one that called `Application::new`). Not `Send` (uses `Rc`, `RefCell`, non-Send WinRT handles) — implicit, not type-asserted.

**DPI.** `Window::get_scale` (window.rs:187-190) calls `GetDpiForWindow(hwnd) / USER_DEFAULT_SCREEN_DPI` live on every call (not cached). `get_rect` uses `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)` rather than `GetWindowRect` — correct for Win11 invisible resize borders.

**Gotchas.**
- `WNDCLASS_INIT: OnceLock<u16>` (window.rs:82-94) uses non-atomic `get().is_none()` + `get_or_init` — racy if windows are created concurrently (today they aren't, but the code reads racy).
- Window is created at `1×1` then resized in `initialize_window` → fires `WM_SIZE` twice on every window birth. **This is intentional**: the managed layer specifies the requested size in *logical* pixels, but the DPI scale needed to convert to *physical* pixels can only be read from a real `HWND` (`GetDpiForWindow`). The minimal-size window is created first to obtain the HWND, then `set_position` applies the logical→physical conversion. Consequence: `WM_SIZE` fires twice during creation; handlers must be idempotent.
- `Window::drop` (window.rs:414-418) only logs a trace; doesn't verify the HWND was destroyed. If the `Rc` drops without `window_destroy`, the HWND leaks.
- `WindowTitleBarKind::Custom` exists but `extend_content_into_titlebar` is `pub(crate)` — no FFI export. Currently unreachable from Kotlin.
- `#[allow(dead_code)]` on `WindowTitleBarKind` (window_api.rs:79) and `WindowSystemBackdropType` (window_api.rs:88) — false positives because cbindgen reads them but rustc can't see that.

**Cross-refs.** `application` (`CompositorController` source, `Weak<EventLoop>`), `event_loop` (wndproc dispatcher), `events` (event payloads), `geometry`, `cursor` (per-window cursor), `pointer` (`PointerClickCounter` storage), `screen` (`window_get_screen_info`), `strings`, `utils` (Win11 build probes).

---

## Renderer (ANGLE)

**Purpose.** Per-window OpenGL ES 2.0 rendering surface backed by ANGLE → D3D11. The EGL window surface targets a WinRT `SpriteVisual` placed in the window's composition tree, so swap-buffers atomically commits to the compositor.

**Files.** `renderer_angle.rs`, `renderer_api.rs`, `renderer_egl_utils.rs` + Kotlin `Renderers.kt`.

**FFI surface.** `renderer_angle_device_create`, `renderer_angle_drop`, `renderer_angle_resize_surface`, `renderer_angle_make_current`, `renderer_angle_swap_buffers`, `renderer_angle_get_egl_get_proc_func`.

**Key types.**
- `AngleDevice` (renderer_angle.rs:43) — owns the EGL `Display`, `Context`, `Surface`, the `CompositorController`, and a `SpriteVisual`. Box-allocated, opaque pointer.
- `EglInstance` (`renderer_egl_utils.rs:7`) — `khronos_egl::DynamicInstance<EGL1_5>` type alias.
- `EglGetProcFuncData`, `EglSurfaceData`, `SurfaceParams` — small `#[repr(C)]` structs returned to Kotlin.

**Init sequence (renderer_angle.rs).**
1. `GetModuleFileNameW(get_dll_instance())` → resolve `libEGL.dll` from the same directory as `desktop_win32.dll`.
2. `eglGetPlatformDisplay(EGL_PLATFORM_ANGLE_ANGLE, EGL_PLATFORM_ANGLE_TYPE_D3D11_ANGLE, …)`.
3. `eglCreateContext` (OpenGL ES 2.0).
4. `eglCreateWindowSurface` against the `SpriteVisual` (an ANGLE-recognised native window handle).

**Threading.** EGL contexts are not implicitly thread-safe — `make_current` must run on the thread that issues GL calls. `AngleDevice` is implicitly `!Send` (WinRT handles inside it are `!Send`); not asserted at the type level.

**Gotchas.**
- `swap_buffers` calls `glFinish` unconditionally before `eglSwapBuffers` (renderer_angle.rs:145) — serialises the CPU on every frame, eliminating GPU/CPU pipelining. Likely intentional for composition correctness; perf concern worth re-evaluating.
- `swap_interval` hardcoded to 1; TODO at renderer_angle.rs:146 says "0 on resize" — no logic to detect resize.
- `core::mem::transmute` in the `get_egl_proc!` macro (renderer_egl_utils.rs:37) — no `// SAFETY` comment; correctness rests on EGL spec matching the local function-pointer typedef.
- `#[allow(clippy::bool_to_int_with_if)]` at renderer_angle.rs:143 is dead — function body has no conditional; leftover from a refactor.
- `libEGL.dll` lookup has no fallback — missing DLL surfaces as `ERROR_PATH_NOT_FOUND` with no helpful diagnostic.

**Cross-refs.** `application` (factory), `window` (provides the `SpriteVisual`), `geometry` (surface dimensions in physical pixels).

---

## Geometry

**Purpose.** Two-tier pixel model. `PhysicalPixels(i32)` for Win32 RECT/POINT; `LogicalPixels(f32)` for toolkit-facing values. All conversions explicit, applied with `floor(x * scale + 0.5)` rounding.

**Files.** `geometry.rs` + Kotlin `Geometry.kt`, `Converters.kt` (marshalling).

**FFI surface.** No `_api.rs` — types are passed by-value inside other FFI structs.

**Key types.** `PhysicalPixels(i32)`, `LogicalPixels(f32)` (both `#[repr(transparent)]`); `PhysicalPoint`, `PhysicalSize`, `LogicalPoint`, `LogicalSize`, `LogicalRect` (all `#[repr(C)]`).

**Conversions.**
- `LogicalPoint::from_physical(x, y, scale)`, `LogicalSize::from_physical` — divide by scale.
- `LogicalPoint::to_physical(scale)`, `LogicalSize::to_physical` — `floor(v.mul_add(scale, 0.5))`.

**Design note: managed (logical) ↔ Win32 (physical).** The Kotlin / managed layer expresses sizes and positions in **logical pixels** (DPI-independent floats). The Win32 API works in **physical pixels** (raw integer device units). The toolkit deliberately makes this conversion explicit and one-directional per call:
- Anything coming *out* of Win32 (e.g. `GetClientRect`, `GetCursorPos`, pointer event coordinates) is in physical pixels and is converted to logical via `from_physical` once it crosses into the toolkit's `Event` payloads / API returns.
- Anything going *into* Win32 (e.g. `SetWindowPos`, custom cursor positions) is converted from logical to physical via `to_physical` at the FFI boundary.

The DPI scale is owned by `Window` (`get_scale()` calls `GetDpiForWindow(hwnd)` live, never cached). This has a knock-on consequence in window creation — see the Window subsystem's "1×1 then resize" note.

**Exceptions: physical pixels exposed to managed code.** Some FFI surfaces deliberately leak `PhysicalPoint` / `PhysicalSize` straight to Kotlin without any logical-pixel conversion. These are the cases where the Win32 source is inherently physical and applying a single window's DPI scale would either be wrong (cross-monitor coordinates) or pointless (raw frame geometry):

| Site | Payload | Why physical |
|---|---|---|
| `screen_map_to_client(WindowPtr, PhysicalPoint) -> PhysicalPoint` (`screen_api.rs:36`) | both input and result | Maps screen-space to client-space; both endpoints are physical and the conversion is a `ScreenToClient` call. |
| `ScreenInfo.origin` (`screen.rs:27`) | should be `PhysicalPoint`; currently `LogicalPoint` (bug — see `TODO.md`) | Multi-monitor desktop origin: positions a monitor on a virtual desktop that may span multiple displays with different DPI scales, so no single monitor's scale converts losslessly. |
| Pointer events `locationOnScreen` (Down/Entered/Exited/Updated/Up + ScrollWheel) (`events.rs:114, 124, 133, 148, 159, 170`) | `PhysicalPoint` | Screen-space coordinates; the per-window scale doesn't apply when the cursor is over another monitor. (The window-relative `location` sibling, when present, is logical.) |
| `WindowMoveEvent.origin`, `WindowResizeEvent.size`, `WindowDrawEvent.size`, `WindowScaleChangedEvent.{origin,size}`, `NCCalcSizeEvent.{origin,size}` (`events.rs:84-85, 203, 216, 229, 242-243`) | `PhysicalPoint` / `PhysicalSize` | These events directly mirror Win32 messages whose payloads are physical-pixel `RECT`s. `WindowScaleChangedEvent` in particular precedes the new DPI taking effect, so converting via the *current* scale would give a stale value. The managed code is expected to either treat them as raw, or convert using a freshly-fetched scale. |
| `DropTarget.onDragEnter / onDragOver / onDrop` `point: PhysicalPoint` (`DragDrop.kt:53, 57, 61`) | `PhysicalPoint` | OLE delivers the drag point as a screen-space `POINTL` in physical pixels; the toolkit passes it through unchanged so callers can decide how (and against which window) to convert. |

When adding a new event or FFI return value carrying coordinates, default to `LogicalPoint` / `LogicalSize`. Use `PhysicalPoint` / `PhysicalSize` only when one of the above conditions applies — and document why at the call site.

Some of these exceptions are up for re-evaluation. The `DropTarget` callbacks, for example, could plausibly convert to logical pixels at the FFI boundary using the target window's scale (since the drop *target* always has a well-defined scale, even if the source coordinate is screen-space). Doing so would save every caller the same conversion. Whether this is worth the loss of fidelity (e.g. for callers wanting raw screen coordinates to compare against `Screen` data) is the open question. See `TODO.md` → "Open design questions".

**Cross-refs.** Used by `events`, `event_loop`, `window`, `window_api`, `pointer`, `drag_drop`, `screen`, `screen_api`. Note: not used by `appearance` or any renderer file directly. Foundational.

---

## Keyboard

**Purpose.** Two roles: (a) instantaneous polling of virtual-key state (`GetKeyState`/`GetKeyboardState`); (b) on-demand decoding of in-flight `WM_KEYDOWN` messages to Unicode (`ToUnicodeEx`) or `WM_CHAR` (`TranslateMessageEx`).

**Files.** `keyboard.rs`, `keyboard_api.rs`, `events_api.rs` (the translate/unicode helpers) + Kotlin `Keyboard.kt`.

**FFI surface.** `keyboard_get_key_state`, `keyboard_get_state`, `keyevent_translate_message`, `keydown_to_unicode`.

**Key types.** `VirtualKey(u16)`, `PhysicalKeyStatus` (decoded `LPARAM`: scancode, repeat count, `KF_*` flags), `KeyState { is_down, is_toggled }`.

**Mechanism for translate / unicode.** When `WM_KEYDOWN` arrives, the original `MSG` is stashed in the `KEYEVENT_MESSAGES: thread_local!<HashMap<u64, MSG>>` keyed on an auto-incrementing `u64` ID. The Kotlin `KeyDownEvent` carries that ID; calling `KeyDown.toUnicode()` or `KeyEvent.translate()` re-enters Rust which looks the message up. Restricted to the UI thread by the `thread_local!`.

**Gotchas.**
- `keydown_to_unicode` calls `ToUnicodeEx` with the "do not change keyboard state" flag (bit 2) — dead-key state in the OS is not consumed by the probe. But the negative return value (which signals "dead key stored") is collapsed via `unsigned_abs()` (events_api.rs:58), so the caller can't tell "dead key applied" from "regular character emitted".
- No `WM_IME_*` handlers anywhere. IME composition characters arrive only through `WM_CHAR` / `WM_DEADCHAR`. Full IME composition window is unhandled.
- `PhysicalKeyStatus.scan_code` is 8 bits (`LOBYTE(HIWORD(lparam))`). Extended scancodes (e0-prefixed) carry only the trailing byte; the prefix must be reconstructed from `is_extended_key`.
- `VirtualKey` width inconsistency: Rust `u16` (keyboard.rs:10), FFI `i32` (keyboard_api.rs:29), Kotlin `Int` (`@JvmInline value class`).
- `keyboard_get_state` returns `AutoDropArray<u8>` (256 bytes); Kotlin reads into `KeyboardState(ByteArray)` with bit-mask helpers.

**Cross-refs.** `events` (`KeyDownEvent`, `CharacterReceivedEvent`), `event_loop` (`thread_local!`, dispatcher), `desktop_common::ffi_utils` (`AutoDropArray`, `RustAllocatedStrPtr`).

---

## Pointer

**Purpose.** Translates `WM_POINTER*` (and non-client `WM_NCPOINTER*`) into typed events. Maintains per-window `PointerClickCounter` for OS double-click logic.

**Files.** `pointer.rs` (no `_api.rs`) + Kotlin `Pointer.kt`. Pointer events surface only via `Event` enum variants.

**Key types.**
- `PointerInfo` — enum (Touch/Pen/Common) wrapping `POINTER_TOUCH_INFO` / `POINTER_PEN_INFO` / `POINTER_INFO`. The touch/pen extras (contact area, pressure, tilt) are stored but not currently exposed in events.
- `PointerState`, `PointerButton`, `PointerButtons` (bitmask), `PointerModifiers` (bitmask).
- `PointerClickCounter` — tracks button identity, time window, move threshold (`SM_CXDOUBLECLK`/`SM_CYDOUBLECLK`).

**Mechanism.** `EnableMouseInPointer(true)` (event_loop.rs:55, process-wide) routes `WM_MOUSE*` through the `WM_POINTER*` path. `PointerInfo::try_from_message` dispatches on `dwInputType` to call `GetPointerTouchInfo` / `GetPointerPenInfo` / `GetPointerInfo`. `PointerModifiers` is populated via `core::mem::transmute::<u32, PointerModifiers>(dwKeyStates)` (pointer.rs:153).

**Gotchas.**
- WM_POINTERUPDATE + WM_POINTERDOWN both emit `PointerDown` for the same gesture (see "Application & event loop" gotchas).
- `PointerClickCounter::register_click` uses `GetMessageTime()` (i32) — wraps every ~49 days. Wrap-safe for short intervals via `cast_unsigned()` subtraction.
- `PointerModifiers` bit values (Shift=4, Ctrl=8) are documented only on the Kotlin side (`Pointer.kt:37-40`); Rust has no named constants.
- `pointer.rs:162` carries `#[allow(clippy::cast_precision_loss)]` on a DPI-math expression — real precision concern at high resolutions; suppressed silently.

**Cross-refs.** `events`, `event_loop`, `window` (`PointerClickCounter` storage, `is_pointer_in_window`), `geometry`.

---

## Cursor

**Purpose.** Show/hide cursor counter (`ShowCursor`) and per-window cursor image selection (system cursors via `LR_SHARED`, file cursors via `LR_LOADFROMFILE`).

**Files.** `cursor.rs`, `cursor_api.rs` + Kotlin `Cursor.kt`. Note: cursor *image* setters live in `window_api.rs` (`window_set_cursor_from_file`, `window_set_cursor_from_system`); `cursor_api.rs` only has show/hide.

**FFI surface.** `cursor_show`, `cursor_hide` (both return `CursorDisplayCounter(i32)`).

**Key types.** `Cursor` — RAII wrapper around `HCURSOR` plus `is_system: bool` flag. `Drop` calls `DestroyCursor` only when `is_system == false` (Win32 contract: `LR_SHARED` cursors must not be destroyed). `CursorIcon` enum maps to PCWSTR system cursor IDs.

**Per-window cursor.** Stored as `RefCell<Option<Cursor>>` inside `Window`. Set via `Window::set_cursor` (`window.rs:266`); the previous `Cursor` drops (and is freed if it was a file cursor). Initialised to `Arrow` in `initialize_window` (`window.rs:487`). Re-applied to the DC on `WM_SETCURSOR` for the `HTCLIENT` hit.

**Gotchas.**
- `CursorIcon::Unknown` panics: `cursor.rs:52` — `panic!("Can't create Unknown cursor")`. Triggered if the integer 0 is ever passed across FFI from Kotlin.
- `CursorDisplayCounter` semantics (counter goes negative; visible only when ≥ 0) are not documented at the Kotlin layer.
- `cursor_api.rs` is incomplete relative to the cursor feature set — set-cursor APIs live elsewhere.

**Cross-refs.** `window` (per-window cursor state), `window_api` (set-cursor FFI), `strings` (file path to `HSTRING`).

---

## Screen

**Purpose.** Enumerate monitors and convert screen-space points to client-space. On-demand snapshot — no caching, no change events.

**Files.** `screen.rs`, `screen_api.rs` + Kotlin `Screen.kt`.

**FFI surface.** `screen_list` → `AutoDropArray<ScreenInfo>`, `screen_list_drop`, `screen_info_drop`, `screen_map_to_client`.

**Key types.** `ScreenInfo` — `#[repr(C)]` struct: `is_primary` (`bool`), `name` (`AutoDropStrPtr`), `origin` (`LogicalPoint`), `size` (`LogicalSize`), `scale` (`f32`), `maximum_frames_per_second` (`u32`).

**Mechanism.** `EnumDisplayMonitors` callback fills `Vec<ScreenInfo>` via `GetMonitorInfoW` + `EnumDisplaySettingsW` + `GetDpiForMonitor(MDT_EFFECTIVE_DPI)` + `EnumDisplayDevicesW`.

**Gotchas.**
- `EnumDisplayMonitors` aborts on first per-monitor failure (returns `FALSE` from the callback). One bad monitor → entire `screen_list` errors out.
- `is_primary` detected by `dmPosition == (0,0)` (`screen.rs:97`) rather than the canonical `MONITORINFOF_PRIMARY` flag.
- `dpi_y` is fetched but discarded — asymmetric DPI silently mishandled.
- No `WM_DISPLAYCHANGE` handler anywhere — monitor topology changes invisible. Stale data until caller re-invokes `screen_list`.
- `screen_info_drop` is exported but never called from Kotlin — possibly vestigial.
- `// todo color space?` and `// todo stable uuid?` at screen.rs:31-32.

**Cross-refs.** `geometry` (point/size types), `strings` (monitor name), `window` (`window_get_screen_info`), `desktop_common::ffi_utils` (`AutoDropArray`, `AutoDropStrPtr`).

---

## Appearance

**Purpose.** Detect dark vs light mode by inspecting WinRT `UISettings::GetColorValue(Foreground)` and applying a luminance heuristic.

**Files.** `appearance.rs`, `appearance_api.rs` + Kotlin `Appearance.kt`.

**FFI surface.** `application_get_appearance` → `Appearance` (C enum).

**Mechanism.** `UISettings` COM object cached in `static CACHED_UI_SETTINGS: OnceLock<UISettings>` (appearance.rs:35). Each `get_current()` call reads the foreground colour and tests `(5*G + 2*R + B) > 8*128` (appearance.rs:50). Light foreground → dark theme.

**Change events.** Delivered via `WM_SETTINGCHANGE` filtered on `wparam == 0 && lparam == "ImmersiveColorSet"` (event_loop.rs:258-270). Event handler re-queries `Appearance::get_current()` and fires `SystemAppearanceChangeEvent`.

**Gotchas.**
- High-contrast mode changes (also via `WM_SETTINGCHANGE`) excluded by the `wparam == 0` filter — high contrast is not modelled.
- `WM_SETTINGCHANGE` is broadcast per-window, so the appearance event fires once per window. Apps with multiple windows see N redundant events for one OS change.
- No registry / `DwmSetWindowAttribute` consultation here — DWM titlebar tinting is handled in `window.rs` / `event_loop.rs`, not in `appearance.rs`.

**Cross-refs.** `events` (`SystemAppearanceChangeEvent`), `event_loop` (`on_settingchange`), `window` (DWM dark titlebar), WinRT `Windows.UI.ViewManagement`.

---

## Clipboard

**Purpose.** Two access paths to the Windows clipboard:
1. **Legacy Win32** (`clipboard_*`): `Open/Get/Set/EmptyClipboard`. HGLOBAL-only.
2. **OLE** (`ole_clipboard_*`): `OleGetClipboard` / `OleSetClipboard` returning/accepting an `IDataObject`. Supports any TYMED.

**Files.** `clipboard.rs`, `clipboard_api.rs` + Kotlin `Clipboard.kt` (object) and `OleClipboard` (object inside `Clipboard.kt`).

**FFI surface.** `clipboard_count_formats`, `clipboard_enum_formats`, `clipboard_is_format_available`, `clipboard_empty`, `clipboard_get_sequence_number`, `clipboard_{get,try_get,set}_{data,file_list,html_fragment,text}`, `clipboard_get_html_format_id`, `ole_clipboard_{empty,get_data,set_data}`, `native_byte_array_drop`, `native_optional_byte_array_drop`.

**Key types.** `Clipboard` (RAII wrapper around `OpenClipboard`/`CloseClipboard`, asserts `is_open`).

**Throwing vs `try_*`.** Both variants exist for every read: throwing version returns `R::default()` and surfaces an exception; `try_*` returns `FfiOption<R>` and (currently) swallows all errors to `None`. Per-user note: `try_*` should swallow only "format not found" — see TODO.md.

**Gotchas.**
- `ole_clipboard_set_data` calls `OleFlushClipboard` immediately (clipboard_api.rs:208-209). The original `IDataObject` is no longer the live clipboard object after the call; subsequent mutations to it have no effect.
- `Clipboard::is_format_available` returns `Ok(false)` for the documented "ok HRESULT means false" Win32 quirk in `IsClipboardFormatAvailable` (clipboard.rs:55).
- DataReader path is **not** used here — `GetClipboardData` always returns HGLOBAL, so `hglobal_reader` is called directly.

**Cross-refs.** `window` (parent HWND for `OpenClipboard`), `data_object` (OLE path target), `data_reader` (used only by `data_object_api`, not here), `global_data` (HGLOBAL helpers), `data_transfer` (`DataFormat`).

---

## DataObject

**Purpose.** Rust-implemented `IDataObject` backed by a `papaya::HashMap<u32, HGlobalData>`. Both ends of the OLE clipboard / drag-drop path use it: it's the Rust-side container that Kotlin populates and hands to Win32, and it's also the wrapper that reads back from any `IDataObject` (own or external) via `data_object_api`'s `com_data_object_*` functions.

**Files.** `data_object.rs`, `data_object_api.rs` + Kotlin `DataObject.kt`, `DataFormat.kt`.

**FFI surface.** `data_object_create() -> i64`, `data_object_drop(i64)`, `data_object_add_from_{bytes,file_list,html_fragment,text}`, `data_object_into_com() -> ComInterfaceRawPtr`, `com_data_object_is_format_available`, `com_data_object_enum_formats`, `com_data_object_{read,try_read}_{bytes,file_list,html_fragment,text}`, `com_data_object_release`, `native_u32_array_drop`.

**Key types.**
- `DataObject` (Rust) — `#[implement(IDataObject)]`. `papaya::HashMap<u32, HGlobalData>` keyed on cfFormat (cast to u32). Implements `GetData`, `EnumFormatEtc`, `QueryGetData`. `SetData`, `GetDataHere`, `DAdvise*` return `E_NOTIMPL` / `OLE_E_ADVISENOTSUPPORTED`.
- Global registry: `static DATA_OBJECT_REGISTRY: papaya::HashMap<i64, ComObject<DataObject>>` keyed on `AtomicI64` IDs. `data_object_create` inserts; `data_object_into_com` removes and converts to `ComInterfaceRawPtr` for OS hand-off.
- `DataObject` (Kotlin, `AutoCloseable`) — wraps the `ComInterfaceRawPtr`. `requireOpen` guard. `read*` (throwing) and `tryRead*` (nullable) variants.
- `DataObjectBuilder` (Kotlin) — used inside `DataObject.build { … }` block-scoped builder. Hides the `data_object_create` → populate → `data_object_into_com` lifecycle.

**Lifecycle.** Kotlin: `DataObject.build { addTextItem(…); addListOfFiles(…) }` returns a `DataObject` whose `comInterfacePtr` is the result of `data_object_into_com`. The Rust-side struct lives via the COM refcount inside `ComObject<DataObject>`. `DataObject.close()` calls `com_data_object_release` → `IUnknown::Release`.

**Gotchas.**
- `tryRead*` collapses all errors to `None`; should be format-not-found only. See TODO.md.
- `DataObject` Kotlin class is **not thread-safe**: `requireOpen` reads `comInterfacePtr` without sync; concurrent `close()` + `read*()` is a data race.
- Module-blanket `#![allow(clippy::inline_always)]` and `#![allow(clippy::ref_as_ptr)]` (data_object.rs:1-2) — inherited from windows-core's `implement!` macro expansion.
- Format-id casts: `data_format.id() as u16` (data_object.rs:159) carries `#[allow(clippy::cast_possible_truncation)]`. Registered IDs (0xC000–0xFFFF) and CF_* values fit, but the suppression is undocumented. Could use `try_into()` with an error.

**Cross-refs.** `com.rs` (`ComInterfaceRawPtr`), `data_reader` (read path), `data_transfer` (format IDs), `global_data` (HGLOBAL backing), `papaya`, `windows::Win32::System::Com::IDataObject`, `windows-core::implement`.

---

## DataReader

**Purpose.** RAII wrapper that calls `IDataObject::GetData` once and dispatches subsequent `get_text` / `get_bytes` / `get_file_list` / `get_html` to either an HGLOBAL reader or an IStream reader, depending on the medium type returned. Lives across one read call; never stored, never returned across FFI.

**Files.** `data_reader.rs`. No `_api.rs` — used internally by `data_object_api`.

**Key types.**
- `DataReader { source: DataSource, guard: StgMediumGuard }`.
- `DataSource::HGlobal(HGlobalData)` or `DataSource::IStream(IStream)`.
- `StgMediumGuard { medium: STGMEDIUM }` with `Drop` that calls `ReleaseStgMedium` unconditionally (the Win32 helper dispatches based on `tymed` and the `pUnkForRelease` slot internally).

**Mechanism.** `DataReader::create(data_object, data_format)` requests `TYMED_HGLOBAL | TYMED_ISTREAM`. On HGLOBAL: `HGlobalData::copy_from(handle)` (deep copy). On IStream: clone (`AddRef` increment). Then dispatches `get_*` to `hglobal_reader` or `istream_reader` submodules.

**Gotchas.**
- `istream_reader::get_text` is **not** compatible with `IStream_ReadStr` (data_reader.rs:154-159 documents this) — `IStream_ReadStr` uses a 4-byte length prefix + UTF-16 wire format (a shlwapi-implementation convention for application-private persistence). This reader treats the stream as raw UTF-16 LE, correct for clipboard data. Mixing the two paths would corrupt reads.
- `#[expect(dead_code, reason = "retained solely for its Drop side-effect")]` on `guard` (data_reader.rs:35-36) — legitimate, well-documented use.

**Cross-refs.** `data_object` (called via `IDataObject` trait), `global_data` (`HGlobalData`, `hglobal_reader`).

---

## DataFormat / data_transfer

**Purpose.** Defines the `DataFormat` enum and the `data_transfer_register_format` FFI for registering arbitrary named clipboard formats.

**Files.** `data_transfer.rs`, `data_transfer_api.rs` + Kotlin `DataFormat.kt`.

**FFI surface.** `data_transfer_register_format(name: BorrowedStrPtr) -> u32`.

**Key types.**
- `DataFormat::Text` (CF_UNICODETEXT = 13), `::FileList` (CF_HDROP = 15), `::HtmlFragment` (`LazyLock<u32>` calling `RegisterClipboardFormatW("HTML Format")` once), `::Other(u32)`.
- Kotlin `DataFormat` is a `@JvmInline value class` over `Int`. `Text = 13` and `FileList = 15` are hardcoded literals. `Html` is a lazy property calling the native helper `clipboard_get_html_format_id()`.

**Gotchas.**
- `DataFormat.Html` (Kotlin) is lazy and triggers a native FFI call on first access — accessing it before `KotlinDesktopToolkit.init()` loads the DLL will crash. No guard.
- Hardcoded CF values in Kotlin (`13`, `15`) bypass any compile-time link to the Rust enum. They're correct (Win32 constants are stable), but the linkage is by convention only.

**Cross-refs.** `clipboard`, `data_object`, `data_reader`, `global_data`.

---

## Drag-and-drop

**Purpose.** Bidirectional OLE drag-drop: implements both `IDropSource` (start a drag) and `IDropTarget` (receive a drop), bridging the COM callbacks to Kotlin function pointers via `DropTargetCallbacks` / `DragSourceCallbacks` structs.

**Files.** `drag_drop.rs`, `drag_drop_api.rs` + Kotlin `DragDrop.kt`.

**FFI surface.** `drag_drop_register_target`, `drag_drop_start`, `drag_drop_revoke_target`.

**Key types.**
- `DropTarget`, `DragSource` — `windows-core` `implement!`-decorated COM impls.
- `DropTargetCallbacks` — `extern "C"` function pointers held by `DropTarget`. Allocated by Kotlin in an `Arena.ofShared()` whose lifetime matches the `DragDropManager`.
- `DragSourceCallbacks` — same pattern for the source side.
- `DragDropManager` (Kotlin, `AutoCloseable`).

**Mechanism.**
- Drop side: `RegisterDragDrop(hwnd, drop_target)` registers our `IDropTarget`. Win32 calls `DragEnter` / `DragOver` / `DragLeave` / `Drop` with an `IDataObject`. We wrap that as `ComInterfaceRawPtr` and upcall to Kotlin.
- Source side: `DoDragDrop(data_object, drop_source, allowed_effects, &mut effect)`. `DragSource::GiveFeedback` always returns `DRAGDROP_S_USEDEFAULTCURSORS`.
- `drag_drop_revoke_target` calls `RevokeDragDrop`.

**Gotchas.**
- **`ComInterfaceRawPtr` lifetime in callbacks** (drag_drop.rs:97 — `// TODO: figure out lifetime`). The `DataObject(rawPtr)` Kotlin instance constructed inside `dragEnter` / `drop` wraps a pointer whose validity is bounded by the COM callback. If Kotlin stores that `DataObject` beyond the callback, the pointer escapes. Currently no enforcement.
- Module-blanket `#![allow(clippy::inline_always)]` and `#![allow(clippy::ref_as_ptr)]` (drag_drop.rs:1-2) — same as `data_object.rs`, inherited from `implement!`.
- COM callbacks arrive on the STA thread; the confined `Arena` for callback stubs is single-thread, so this is consistent only as long as drag-drop stays on the STA thread.

**Cross-refs.** `data_object` (the wrapped `IDataObject`), `com.rs` (`ComInterfaceRawPtr`), `geometry` (`PhysicalPoint` from `POINTL`), `window`.

---

## Global data (HGLOBAL)

**Purpose.** All HGLOBAL allocation, copying, locking, and freeing for clipboard / data-object payloads. Houses two submodules: `hglobal_writer` (build a payload) and `hglobal_reader` (read a payload).

**Files.** `global_data.rs`. No `_api.rs` — used internally only.

**Key types.**
- `HGlobalData { handle: HANDLE, is_owned: bool }` — RAII; `Drop` calls `GlobalFree` if `is_owned`.
  - `HGlobalData::alloc_and_init`, `alloc_from`, `copy_from`, `copied`, `detach()` (relinquish ownership).
- `hglobal_writer::new_text` / `new_bytes` / `new_file_list` / `new_html` — produce filled `HGlobalData`.
- `hglobal_reader::get_text` / `get_bytes` / `get_file_list` / `get_html` — lock, read, unlock.

**Gotchas.**
- `new_file_list` takes `&Vec<&str>` (global_data.rs:100) — should be `&[&str]` (clippy `ptr_arg` smell).
- `global_mem_copy` doesn't handle zero-length globals — `GlobalAlloc(GMEM_FIXED, 0)` semantics are platform-specific.
- HTML format read/write goes through WinRT `HtmlFormatHelper::CreateHtmlFormat` / `GetStaticFragment` — a rare WinRT call in an otherwise pure-Win32 path.

**Cross-refs.** `data_reader`, `data_object`, `clipboard`, `data_transfer`, `strings` (UTF-16/UTF-8 conversions), Win2D-adjacent WinRT (`Windows.ApplicationModel.DataTransfer.HtmlFormatHelper`).

---

## COM helpers

**Purpose.** Bridge `windows-core` `ComObject<T>` to a raw pointer ABI suitable for FFI. Carries an `IUnknown` strong ref through a `*mut c_void`.

**Files.** `com.rs`. No `_api.rs`.

**Key types.**
- `ComInterfaceRawPtr` — `#[repr(transparent)]` over `*mut c_void`. Constructors: `from_object(ComObject)`, `new(IDataObject)`. Methods: `borrow<T>()` (typed reinterpret without changing refcount). `Drop` calls `IUnknown::from_raw(...).Release()`.

**Gotchas.**
- Distinguished from the `RustAllocated*Ptr` aliases — `ComInterfaceRawPtr` is a real struct with its own Drop. See `FFI_CONVENTIONS.md`.
- All construction is `unsafe`; no safety comments on the unsafe blocks.

**Cross-refs.** Used by `data_object_api`, `drag_drop`, `clipboard_api` (OLE path).

---

## File dialog

**Purpose.** Modal `IFileOpenDialog` / `IFileSaveDialog` invocation. Always blocking; the dialog pumps its own internal message loop.

**Files.** `file_dialog.rs`, `file_dialog_api.rs` + Kotlin `FileDialog.kt`.

**FFI surface.** `open_file_dialog_run_modal(window_ptr, common_options, open_options) -> AutoDropArray<RustAllocatedStrPtr>`, `save_file_dialog_run_modal(window_ptr, common_options) -> RustAllocatedStrPtr`.

**Key types.**
- `FileOpenDialog`, `FileSaveDialog` — wrappers around the COM interfaces.
- `FileDialogOptions` — `#[repr(C)]`: title, prompt, label, default name, default folder, `shows_hidden_files`.
- `FileOpenDialogOptions` — `choose_directories`, `allows_multiple_selection`.

**Mechanism.** `CoCreateInstance(CLSID_FileOpenDialog/SaveDialog, CLSCTX_INPROC_SERVER)` → set options via `GetOptions`/`SetOptions` → `Show(parentHwnd)` → result via `GetResult`/`GetResults` → `IShellItem::GetDisplayName(SIGDN_FILESYSPATH)`.

**Cancel sentinels.**
- Open: returns zero-length `AutoDropArray` (Kotlin `emptyList()`).
- Save: returns empty `CString` (Kotlin `takeUnless { isEmpty() }` → null). Implicit convention; no typed `FfiOption`.

**Gotchas.**
- **No file-type filter support.** `COMDLG_FILTERSPEC` / `SetFileTypes` not used. Capability gap vs. macOS. See TODO.md.
- `filter_map` (file_dialog.rs:134) silently drops items that fail `GetDisplayName` or UTF-8 conversion — partial result returned with no caller signal.
- `retrieved_items` count from `IEnumShellItems::Next` is ignored.
- `SetDefaultFolder` (not `SetFolder`) — shell MRU can override the suggested initial folder.
- 14 unsafe blocks, no `// SAFETY` comments anywhere.

**Cross-refs.** `window` (parent HWND), `application` (OLE STA), `strings` (path conversions), `desktop_common::ffi_utils` (string/array marshalling).

---

## Plumbing

**Purpose.** Cross-cutting infrastructure: cdylib entry, module index, string/array helpers, Win32 macros, version probes, plus the entire `desktop-common` crate (FFI types, logger, error/FFI boundary).

**Files (Rust).** `lib.rs`, `logger_api.rs`, `win32/mod.rs`, `win32/strings.rs`, `win32/strings_api.rs`, `win32/utils.rs`, `desktop-common/src/{lib,ffi_utils,logger,logger_api}.rs`.

**Files (Kotlin).** `KotlinDesktopToolkit.kt`, `Logger.kt`, `Strings.kt`, `Arrays.kt`, `Converters.kt`, `org/jetbrains/desktop/common/Platform.kt`.

**FFI surface.**
- `logger_api` (in `desktop-common`): `logger_init`, `logger_check_exceptions`, `logger_clear_exceptions`.
- `desktop-win32::logger_api`: thin Win32-side glue (e.g. `logger_output_debug_string`).
- `strings_api`: `native_string_drop`, `native_optional_string_drop`, `native_string_array_drop`, `native_optional_string_array_drop`.

**Highlights.**
- `lib.rs` captures `HINSTANCE` in `DllMain` (`DLL_PROCESS_ATTACH`) into a `static AtomicPtr`, exposed via `get_dll_instance()`. Used by ANGLE for resolving `libEGL.dll` next to the DLL.
- `mod.rs` declares all 36 `win32/*` files as `pub mod` siblings — no visibility narrowing; the only API-control mechanism is which symbols get `#[no_mangle]`.
- `strings.rs` provides the only UTF-16 ↔ UTF-8 converters in the crate (`copy_from_wide_string`, `copy_from_utf8_bytes`, `copy_from_utf8_string`). All three strip a trailing NUL if present and return `anyhow::Result`.
- `utils.rs` exposes `LOWORD` / `HIWORD` / `LOBYTE` / `GET_X_LPARAM` / `GET_Y_LPARAM` / `GET_WHEEL_DELTA_WPARAM` macros (every body suppresses `cast_possible_truncation` + `cast_sign_loss`), plus `is_windows_11_build_22000_or_higher` / `is_windows_11_build_22621_or_higher` via `RoIsApiContractPresent`.
- `desktop-common::ffi_utils` defines the entire pointer/array/option zoo (see `FFI_CONVENTIONS.md`).
- `desktop-common::logger` implements `ffi_boundary`, `catch_panic`, `PanicDefault`, the panic hook, and `log4rs` setup. `RUST_LIB_BACKTRACE` is set via `unsafe { std::env::set_var(...) }` (logger.rs:157-158) — `set_var` became `unsafe` in Rust 1.81 due to multi-threaded data-race risk; no safety comment.

**Kotlin highlights.**
- `KotlinDesktopToolkit.init` loads `desktop_win32_<arch>[+debug].dll` (resolved from `kdt.library.folder.path` system property). `AtomicBoolean`-guarded; double-init warns. `// todo check that native library version is consistent with Kotlin code` (KotlinDesktopToolkit.kt:19).
- `Logger.kt` defines `ffiDownCall`, `ffiUpCall`, `NativeError`, `Logger` facade with `inline` lambda methods. Default appender writes to `System.err`; rolling file appender on the Rust side (2 MB trigger, 3 archives).
- `Strings.kt`, `Arrays.kt` — marshalling helpers. `try { read } finally { ffiDownCall { drop(...) } }` is the established pattern for Rust-allocated returns.
- `Converters.kt` — geometry struct conversions.
- `Platform.kt` (in `org.jetbrains.desktop.common`) defines `isAarch64()`. **Apparently unused from the win32 layer** — `KotlinDesktopToolkit.kt:38` re-implements its own `isAarch64`. Investigate before removing.

**Smells (selected).**
- `ffi_utils.rs:1`: `#![allow(clippy::missing_safety_doc)]` module-wide — all `unsafe fn` lack `# Safety` documentation.
- `logger.rs:181`: typo `"File appender creatrion failed"`.
- `logger.rs:195`: `// todo store handler and allow to change logger severity` — log levels frozen at init.
- Universal lack of `// SAFETY:` comments on `unsafe` blocks throughout the crate.

**Cross-refs.** Every other subsystem uses these helpers. `ffi_boundary` wraps every `_api.rs` function. `RustAllocated*Ptr`, `BorrowedStrPtr`, `AutoDropArray`, `FfiOption` are the core FFI vocabulary.
