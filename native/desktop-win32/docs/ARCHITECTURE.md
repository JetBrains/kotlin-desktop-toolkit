# `desktop-win32` Architecture

Operational guide to the Windows native crate. Audience: future-me and other agents who need to navigate, modify, or debug this code without re-reading every file.

For per-subsystem details see `SUBSYSTEMS.md`. For FFI patterns see `FFI_CONVENTIONS.md`. For known issues see `TODO.md`. For a quick agent on-ramp see `AGENTS.md`.

## Purpose & scope

`desktop-win32` is the Windows backend of `kotlin-desktop-toolkit`. It exposes a flat C ABI from Rust (via `cbindgen`) that JExtract turns into Java bindings, which are wrapped by hand-written Kotlin in `kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/`. The crate is `#![cfg(target_os = "windows")]`-gated and compiles to a Windows DLL (`desktop_win32_x64.dll` / `desktop_win32_arm64.dll`, with optional `+debug` infix).

Non-goal: API parity with macOS / Linux backends. The toolkit deliberately exposes platform-shaped APIs.

## Scope: Win32-first, WinRT only where necessary

This crate is **the Win32-focused backend**. Window creation, message pump, input, clipboard, drag-drop, file dialog, screen enumeration, cursor — all leverage classic Win32 APIs (`CreateWindowExW`, `GetMessageW`, `RegisterDragDrop`, `IFileOpenDialog`, `EnumDisplayMonitors`, `LoadImageW`, etc.). The crate falls back to WinRT only where the Win32 surface is missing, would require materially more code to do correctly, or doesn't integrate with modern Windows-shell features the toolkit must expose.

**Hard rule: this crate does not depend on WinUI 3 or the Windows App SDK, and it never will.** WinUI 3 / Windows App SDK is a higher-level UI framework with its own component model, its own packaging requirements, and its own lifecycle assumptions. A WinUI 3-based backend, if ever built, will live in a **separate crate** (e.g. `desktop-winui` or similar). Inside `desktop-win32`:

- Do not propose WinUI 3 controls, `Microsoft.UI.*` namespaces, or `Microsoft.WindowsAppSDK` references.
- Do not propose `XamlRoot`, `ContentDialog`, `WinRT.Microsoft.UI.Xaml.*`, or any XAML-island bridging.
- If a feature seems to require WinUI 3, the answer in this crate is "we don't support that here" — surface it for review and consider whether a separate WinUI 3 backend is the right home, not a partial dependency in this one.

**Distinction worth holding onto.** "Don't depend on WinUI APIs" is *not* the same as "don't read WinUI docs." When the toolkit needs to replicate Win11-native UX behaviour (caption-button colours, hover/pressed semantics, animation curves, RTL mirroring rules, etc.), WinUI / Windows App SDK documentation and source are valid references for the *behavioural contract* — we cite them as conventions to follow, not as APIs we consume. Where WinUI describes a behaviour but does not expose concrete values, fall back to other public Win32-host implementations chosen on a per-feature basis, citing each value by repo path and revision. The rule: WinUI for the contract; Win32 / WinRT for the implementation; per-feature reference implementations only for values that WinUI keeps closed-source.

### WinRT exceptions in this crate (and why)

The four WinRT subsystems used here, each with explicit rationale:

| WinRT API | Where | Why a Win32 alternative isn't used |
|---|---|---|
| `Windows.UI.Composition` (`Compositor`, `ContainerVisual`, `SpriteVisual`, `DesktopWindowTarget`) — controlled-commit variant via `Core::CompositorController` | `application.rs`, `window.rs`, `renderer_angle.rs` | This is the modern composition surface DWM uses natively. It integrates correctly with `WS_EX_NOREDIRECTIONBITMAP`, with DWM's Mica / Acrylic / dark-mode titlebar effects, and with ANGLE's window-surface targeting — none of which classic Win32 GDI / layered windows / DirectComposition support without significant manual work. The HWND ↔ visual-tree bridge uses `ICompositorDesktopInterop::CreateDesktopWindowTarget`, which is the official Win32-interop interface for hosting a WinRT visual tree in a classic Win32 window. We deliberately use `CompositorController` (controlled commit) rather than `Compositor` (auto-commit) so swap-buffers and backdrop changes can commit atomically. |
| `Windows.System.DispatcherQueueController` (with `DQTYPE_THREAD_CURRENT`) | `application.rs` | The toolkit posts cross-thread work back to the UI thread. The Win32 alternative — `PostMessageW` to a hidden owner window — is workable, but the WinRT `DispatcherQueue` is required anyway for `CompositorController` to schedule its commits, and once it's present it's the natural primitive for `application_dispatcher_invoke`. Not introducing a second mechanism keeps the threading model uniform. |
| `Windows.UI.ViewManagement.UISettings` (`GetColorValue(Foreground)`) | `appearance.rs` | The canonical signal for "is the user in dark mode" on Windows is the foreground colour returned by `UISettings`. The Win32 alternative is reading `HKCU\…\Personalize\AppsUseLightTheme` from the registry, which only covers Win10+ apps mode (not the full system mode), is undocumented as a stable API, and misses the auto-update on theme switch. We still consume the `WM_SETTINGCHANGE` (`ImmersiveColorSet`) Win32 signal to trigger re-query — but the answer comes from WinRT. |
| `Windows.ApplicationModel.DataTransfer.HtmlFormatHelper` (`CreateHtmlFormat`, `GetStaticFragment`) | `global_data.rs` (HTML clipboard format encode / decode) | The "HTML Format" clipboard payload requires a specific header with `Version:` / `StartHTML:` / `EndHTML:` / `StartFragment:` / `EndFragment:` byte offsets. Computing those offsets by hand is error-prone (encoding rules, byte-level placement, the spec's tolerance for absent fields). The WinRT helper does it correctly per the documented Windows clipboard contract. Win32 has no equivalent helper. |

When a future change tempts you to reach for another WinRT API, ask: "Is there a Win32 alternative? If so, why is it materially worse?" Document the answer in this table when adding the dependency.

## Repository layout (in scope)

```
native/
  desktop-common/                 cross-platform plumbing crate
    src/
      ffi_utils.rs                pointer/array/option wrappers, AutoDrop, FfiOption, PanicDefault
      logger.rs                   log4rs setup, ffi_boundary, panic-to-exception channel
      logger_api.rs               LogLevel, LoggerConfiguration, ExceptionsArray (#[repr(C)])
      lib.rs                      re-exports the three modules above
  desktop-win32/
    Cargo.toml                    deps: windows 0.62.x, windows-core, khronos-egl, papaya, anyhow
    cbindgen.toml                 prefix=Native, parse_deps=true (parses desktop-common too)
    src/
      lib.rs                      DllMain → captures HINSTANCE; declares win32 + logger_api
      logger_api.rs               thin Win32 logger glue
      win32/
        mod.rs                    declares 36 sibling pub mod entries
        application.rs            Application: OLE STA, DispatcherQueue, CompositorController
        application_api.rs        FFI: app lifecycle + dispatcher dispatch
        event_loop.rs             window_proc: WM_* → Event dispatch; thread_local key/exception stash
        events.rs                 Event enum (22 variants, #[repr(C)]) + payload structs
        events_api.rs             FFI: keyevent_translate_message, keydown_to_unicode
        window.rs                 Window struct: HWND, WinRT composition tree, DWM, cursor, icon
        window_api.rs             FFI: window lifecycle + per-window setters
        renderer_angle.rs         AngleDevice: ANGLE/EGL on D3D11, surface=SpriteVisual
        renderer_api.rs           FFI: angle device create/drop/resize/make_current/swap
        renderer_egl_utils.rs     EglInstance type alias, get_egl_proc! macro
        geometry.rs               PhysicalPixels/LogicalPixels + Point/Size/Rect (#[repr(C)])
        keyboard.rs               VirtualKey, PhysicalKeyStatus (LPARAM decode)
        keyboard_api.rs           FFI: keyboard_get_key_state, keyboard_get_state
        pointer.rs                PointerInfo (Touch/Pen/Common), PointerClickCounter, modifiers
        cursor.rs                 Cursor RAII (HCURSOR + is_system flag), CursorIcon → PCWSTR
        cursor_api.rs             FFI: cursor_show / cursor_hide (counter)
        screen.rs                 EnumDisplayMonitors → ScreenInfo; ScreenToClient
        screen_api.rs             FFI: screen_list / screen_map_to_client / drop fns
        appearance.rs             UISettings → Dark/Light via foreground-luminance heuristic
        appearance_api.rs         FFI: application_get_appearance
        clipboard.rs              Win32 GetClipboardData / SetClipboardData RAII wrapper
        clipboard_api.rs          FFI: clipboard_* (Win32 path) + ole_clipboard_* (OLE path)
        drag_drop.rs              IDropSource / IDropTarget via windows-core implement!
        drag_drop_api.rs          FFI: drag_drop_register_target / start / revoke
        data_object.rs            IDataObject impl backed by papaya::HashMap<u32, HGlobalData>
        data_object_api.rs        Global registry (id→ComObject); read API + try* variants
        data_reader.rs            STGMEDIUM RAII; HGLOBAL/IStream-uniform get_*
        data_transfer.rs          DataFormat enum (Text=13, FileList=15, HtmlFragment lazy)
        data_transfer_api.rs      FFI: data_transfer_register_format
        global_data.rs            HGlobalData RAII; hglobal_writer + hglobal_reader submodules
        com.rs                    ComInterfaceRawPtr: refcount-carrying void* wrapper
        file_dialog.rs            IFileOpen/SaveDialog wrappers
        file_dialog_api.rs        FFI: open_/save_file_dialog_run_modal
        strings.rs                Internal UTF-16/UTF-8 helpers (HSTRING ↔ CString)
        strings_api.rs            FFI: native_string_drop / native_string_array_drop / *_optional_*
        utils.rs                  LOWORD/HIWORD/GET_*_LPARAM macros; Win11 build probes

kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/
  win32/                          Kotlin bindings — paired with each *_api.rs above
    KotlinDesktopToolkit.kt       library load, kdt.* system properties, init guard
    Logger.kt                     ffiDownCall, ffiUpCall, NativeError, log appenders
    Strings.kt                    stringFromNative / listOfStringsFromNative + drop in finally
    Arrays.kt                     byteArrayFromNative / intArrayFromNative
    Converters.kt                 geometry toNative / fromNative
    Application.kt, Window.kt, Renderers.kt, Geometry.kt, Event.kt,
    Keyboard.kt, Pointer.kt, Cursor.kt, Screen.kt, Appearance.kt,
    Clipboard.kt, DragDrop.kt, DataObject.kt, DataFormat.kt, FileDialog.kt
  common/
    Platform.kt                   internal OS/arch detection (apparently unused from win32)
```

## The FFI pipeline

```
┌─────────── Rust ───────────┐                ┌───── generated ─────┐                ┌──── Kotlin ────┐
│ desktop-common (FFI types) │                │  C header (cbindgen)│                │ JExtract Java   │
│ desktop-win32 (impl + api) │  ─cbindgen──▶  │  Native* prefix     │  ─JExtract──▶  │ NativeFoo bindings + 
│   Cargo: cdylib + rlib     │                │  enum prefix=Native │                │ desktop_win32_h │
│   builds desktop_win32.dll │                │  parse_deps=true    │                │ extension fns   │
└────────────────────────────┘                │  (incl. common)     │                └────────┬────────┘
            │                                 └─────────────────────┘                         │
            │                                                                                 │
            └──────────── compiled DLL ─────loaded by KotlinDesktopToolkit.init──────────────▶│
```

- `cbindgen.toml` sets `[export] prefix = "Native"`, `[enum] prefix_with_name = true`, and `parse_deps = true` with `include = ["desktop-win32", "desktop-common"]`. This means `desktop-common` types (e.g. `AutoDropArray<T>`, `BorrowedStrPtr`, `FfiOption<T>`) appear in the Win32 header directly — there is no Rust `pub use` re-export.
- Items annotated with `/// cbindgen:ignore` are excluded (e.g. `DLL_HINSTANCE`, `DllMain`, `DATA_OBJECT_REGISTRY`).
- All exported functions are wrapped with `ffi_boundary` (see error model below).

## Module conventions

- **`*.rs`** — implementation. Internal types may be `pub` (the whole `win32` tree is `pub mod`), but only items explicitly used as `extern "C"` parameters end up exported by cbindgen.
- **`*_api.rs`** — FFI surface. Every function is `#[unsafe(no_mangle)] pub extern "C"` and wrapped in `ffi_boundary("name", || { ... })`.
- **One-to-one Kotlin pairing** — each `xxx_api.rs` has a corresponding `Xxx.kt` (sometimes split across multiple Kotlin files for related concerns).
- The exception is `pointer.rs` — there is no `pointer_api.rs`. Pointer events are push-only via the `Event` enum; `pointer.rs` types stay `pub(crate)`.

## Threading model

The toolkit assumes a **single UI thread** that:
1. Calls `application_init_apartment()` — calls `OleInitialize(None)` (single-threaded apartment for OLE).
2. Calls `application_init(callbacks)` — creates the `DispatcherQueueController` with `DQTYPE_THREAD_CURRENT` (the queue is thread-local). Stores `Application` as `Rc<Application>`.
3. Calls `application_run_event_loop()` — enters a classic `GetMessage`/`TranslateMessage`/`DispatchMessage` pump until `WM_QUIT`.

Per-thread state stored in `thread_local!`:
- `event_loop.rs`: `KEYEVENT_MESSAGES: RefCell<HashMap<u64, MSG>>` and `LAST_KEYEVENT_MESSAGE_ID: RefCell<u64>` — stash of in-flight key messages so Kotlin can call `keyevent_translate_message`/`keydown_to_unicode` after `window_proc` has returned.
- `desktop-common/logger.rs`: `LAST_EXCEPTION_MSGS: RefCell<...>` — pending exception strings, capacity 10.

Process-wide one-shot calls during init:
- `EnableMouseInPointer(true)` (event_loop.rs:55) — process-wide and irreversible. WM_MOUSE* are synthesised through WM_POINTER*. Third-party code in the same process expecting raw mouse messages will silently break.
- `SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` — per-thread, set in `EventLoop::new`.

Cross-thread work uses `application_dispatcher_invoke`, which posts a Kotlin trampoline (`pollCallbacks`) onto the WinRT `DispatcherQueue`. The Kotlin side queues lambdas in a `ConcurrentLinkedQueue` and drains it on the UI thread. Note: the `bool` returned by `application_dispatcher_invoke` is silently discarded by `Application.invokeOnDispatcher` — enqueues after dispatcher shutdown drop the work.

`OleInitialize` is required for COM/OLE drag-drop, the OLE clipboard path, and the IFileOpen/Save dialogs. `CoInitializeEx` is not called separately (`OleInitialize` calls it internally for an STA).

WinRT `UISettings` (appearance) and `CompositorController` (composition) live in the same STA.

## Ownership model

The crate funnels Rust-allocated heap objects through three boxed-pointer wrappers, all in `desktop-common::ffi_utils`:

| Wrapper | Backing | Lifetime | Drop |
|---|---|---|---|
| `RustAllocatedRawPtr<'a>` | `Box<T>` (Box::into_raw) | opaque to Kotlin | explicit `*_drop` calls `Box::from_raw` and lets it fall |
| `RustAllocatedRcPtr<'a>` | `Rc<T>` (Rc::into_raw) | opaque to Kotlin | `*_drop` reconstructs the `Rc` and lets refcount reach zero |
| `ComObject<T>` (windows-core) | COM refcount on a `windows-core` `implement!`-decorated struct | until `Release()` reaches zero | Kotlin holds a `ComInterfaceRawPtr` → `IUnknown::Release` on drop |

Conventions:
- **Application** is `Box`-based: `application_init` does `Box::new` → `Box::into_raw`; `application_drop` reverses it. **Window** is `Rc`-based: a `Weak<Window>` raw pointer is also stashed as a Win32 window property (`KDT_WINDOW_PTR`) so `window_proc` can resolve a `&Window` cheaply on every message without touching the strong refcount. **AngleDevice** is `Box`-based, one per window. **DataObject** uses `ComObject` (COM ref counting) with a global ID-keyed registry on the Rust side until ownership is converted to a raw pointer for hand-off to OLE.
- **Strings out of Rust**: `RustAllocatedStrPtr` (raw `*const c_char` over a `'static` `CString::into_raw`). Kotlin reads with `getUtf8String(0)` and frees via `native_string_drop` in a `finally` block (Strings.kt). Optional variants use `FfiOption<RustAllocatedStrPtr>` and `native_optional_string_drop`.
- **Strings into Rust**: `BorrowedStrPtr<'a>` — Kotlin allocates in a confined `Arena`; Rust borrows for the duration of the call.
- **Arrays out of Rust**: `AutoDropArray<T>` — `(*const T, usize)`. `Drop` reconstructs `Box<[T]>` and drops it (which recursively drops `T`). Kotlin must call the matching `*_drop` function (no drop-fn stored in the struct itself).
- **Optional values across FFI**: `FfiOption<T: PanicDefault>` — `(is_some: bool, value: T)`. The `T` is always present in memory; on `None` it equals `T::default()`.

The `RustAllocatedRawPtr::borrow` / `borrow_mut` methods are unconventional: each call does `Box::leak(Box::from_raw(ptr))` to obtain `&R` / `&mut R` without consuming the `Box`. Type-level safety relies on (a) the toolkit's single-thread-of-ownership assumption and (b) callers never holding a `&R` across a possible `to_owned`/`drop`. **Marked open for review (see TODO.md).**

## Error handling model

Rust → Kotlin errors flow as `anyhow::Result<T>` through `ffi_boundary`, surfaced on the Kotlin side as a `NativeError` thrown by `ffiDownCall`. Panic catching inside `ffi_boundary` is a **safety net for unexpected unwinds**, not a designed error path: if Rust code reaches the panic branch, treat it as a bug to fix rather than as the channel doing its job.

```
extern "C" fn foo_api(...) -> R {
    ffi_boundary("foo_api", || {
        // anyhow::Result<R>; ? operator works freely
    })
}
```

Inside `ffi_boundary` (logger.rs:312):
1. `Ok(Ok(result))` → return result. **Designed happy path.**
2. `Ok(Err(anyhow_err))` → `error!(…)` log; append message to `LAST_EXCEPTION_MSGS`; return `R::default()` via the `PanicDefault` trait. **Designed error path.**
3. `Err(panic_payload)` (from `panic::catch_unwind(AssertUnwindSafe(closure))`) → append payload string; return `R::default()`. **Safety net only — should never fire in correct code.** `AssertUnwindSafe` is applied unconditionally, so partial mutation after a panic unwind is not protected against. Reaching this path means somewhere a `?` was missed, an `unwrap` slipped through, or an external API panicked unexpectedly.

The Kotlin side polls the thread-local store after every native call:

```kotlin
fun <T> ffiDownCall(body: () -> T): T {
    val result = body()
    val exceptions = checkExceptions()        // logger_check_exceptions FFI
    if (exceptions.isNotEmpty()) {
        logger_clear_exceptions()
        throw NativeError(exceptions)         // subclass of java.lang.Error
    }
    return result
}
```

Conventions:
- `ffiDownCall { ... }` must wrap **only** the native call. Never `Arena.use`, never `withPointer`, never helper calls (helpers wrap their own native calls). See `FFI_CONVENTIONS.md`.
- `ffiUpCall { defaultResult, body }` is the inverse, wrapping Kotlin callbacks invoked from Rust. Catches all `Throwable`, logs it, returns `defaultResult`. Kotlin exceptions never propagate into Rust — they're silently swallowed.
- Background threads do not flush errors to Kotlin: `LAST_EXCEPTION_MSGS` is thread-local, so errors (and unexpected panics) on dispatcher-queue worker threads are visible only to whoever calls `logger_check_exceptions` on the same thread.
- `tryRead*` variants of clipboard/data-object read functions return `FfiOption<T>` instead of throwing — currently they swallow **all** errors, not just format-not-found (open bug: TODO.md).
- The crate uses `anyhow::Error` as the unified error type today. Migrating to `thiserror`-defined typed errors is on the roadmap (see TODO.md) since typed errors are the recommended approach for libraries — they let callers branch on error kinds and keep error names stable across the codebase.

## Subsystem map

```
                ┌─────────────────────────────────┐
                │   Kotlin app                    │
                └──────┬──────────────────────────┘
                       │ JExtract
                ┌──────▼─────────────────────────┐
                │ Application  ── runs ──▶ EventLoop ─────────────┐
                │  │                                              │
                │  ├── creates ──▶ Window ──▶ AngleDevice         │
                │  │                                              │
                │  └── invokes ──▶ DispatcherQueue                │
                │                                                 │
                │  Input  : keyboard / pointer / cursor ◀─ event_loop dispatches
                │  Display: screen / appearance         ◀─ on demand / WM_SETTINGCHANGE
                │  Geometry types are shared by everything that has coordinates.
                │                                                 │
                │  Data transfer:                                 │
                │   clipboard  ─┐                                 │
                │   drag_drop  ─┼─▶ DataObject (COM impl)         │
                │   data_object_api global registry               │
                │                │                                │
                │                └─▶ DataReader ─▶ HGLOBAL or IStream
                │                                  │              │
                │                                  └─▶ global_data.rs
                │                                                 │
                │  COM helpers: com.rs (ComInterfaceRawPtr)       │
                │  File dialog: file_dialog (uses COM, Window)    │
                │                                                 │
                │  Plumbing: strings, utils, logger_api, lib.rs   │
                └─────────────────────────────────────────────────┘
```

## Headline data flows

### 1. Application bootstrap

```
Kotlin: KotlinDesktopToolkit.init(…)                                  // System.load DLL
        Application().runEventLoop                                    // wraps the steps below
  → application_init_apartment()                                       // OleInitialize
  → application_init(ApplicationCallbacks{ event_handler })            // Box<Application> + DispatcherQueue
  → window/renderer creation, dispatch enqueues                        // app remains UI-thread bound
  → application_run_event_loop()                                       // GetMessage loop until WM_QUIT
  → application_drop()                                                 // Box::from_raw, drop
```

### 2. Window creation

```
Kotlin: Window.new(appPtr) → Window.create(params)
  → window_new(app, id) → Rc::new(Window) → Rc::into_raw
  → window_create(ptr, params) → CreateWindowExW(WS_EX_NOREDIRECTIONBITMAP, lpCreateParams=Weak::into_raw)
       │
       └── WM_NCCREATE → on_nccreate: Weak::from_raw → upgrade → initialize_window;
                          re-store Weak::into_raw as Win32 prop KDT_WINDOW_PTR
       …each subsequent WM_*: GetPropW(KDT_WINDOW_PTR) → ManuallyDrop<Weak> → upgrade → wndproc handler
       …
       WM_NCDESTROY → RemovePropW; the recovered Weak is dropped.
  → DwmExtendFrameIntoClientArea / SetWindowAttribute(IMMERSIVE_DARK_MODE / SYSTEMBACKDROP_TYPE)
  → CompositorController (WinRT Windows.UI.Composition) → ContainerVisual + SpriteVisual;
    HWND bridge via ICompositorDesktopInterop::CreateDesktopWindowTarget (window.rs:493-494)
```

### 3. ANGLE rendering

```
Kotlin: Application.createAngleRenderer(window)
  → renderer_angle_device_create(window_ptr)
       │
       └── load libEGL.dll from same dir as desktop_win32.dll
       └── eglGetPlatformDisplay(EGL_PLATFORM_ANGLE_ANGLE, D3D11)
       └── eglCreateContext (OpenGL ES 2.0)
       └── eglCreateWindowSurface targeting the SpriteVisual
  → renderer_angle_make_current(...)
  → render frame, glFinish (note: kills CPU/GPU pipelining), eglSwapBuffers → CompositorController::Commit
```

### 4. Drag-and-drop (drop side)

```
RegisterDragDrop on HWND with our DropTarget COM impl
  → user drags onto window
       Win32 → DropTarget::DragEnter(IDataObject, …)
            → ComInterfaceRawPtr::new(data_object) → wraps strong ref
            → upcall to DropTargetCallbacks.dragEnter (Kotlin) with point & DataObject
            → ⚠ TODO: lifetime — Kotlin must NOT escape the DataObject beyond the callback
       Win32 → DropTarget::DragOver / DragLeave / Drop  …same pattern
```

### 5. Clipboard read (OLE path)

```
Kotlin: OleClipboard.readClipboard()
  → ole_clipboard_get_data() → OleGetClipboard(IDataObject) → ComInterfaceRawPtr
  → DataObject(ptr) — Kotlin AutoCloseable, requireOpen guards
  → DataObject.tryReadText() → com_data_object_try_read_text(ptr)
       │
       └── DataReader::create(data_object, DataFormat::Text)
              → IDataObject::GetData(FORMATETC{TYMED_HGLOBAL|TYMED_ISTREAM})
              → match medium.tymed:
                     HGLOBAL → HGlobalData::copy_from(handle)              (deep copy of the handle's bytes)
                     ISTREAM → istream clone (AddRef)
              → guard: StgMediumGuard { medium }                            (Drop calls ReleaseStgMedium unconditionally)
       └── reader.get_text() → hglobal_reader::get_text or istream_reader::get_text
  → DataObject.close() → com_data_object_release(ptr) → IUnknown::Release
```

## Composition: WinRT `Windows.UI.Composition`, not DirectComposition

A common conflation — these are distinct stacks that can both produce visual trees rendered by DWM, but the APIs, types, and lifetimes differ:

| | DirectComposition (DComp) | Windows.UI.Composition (WinRT) — what we use |
|---|---|---|
| Header / namespace | `Windows.Win32.Graphics.DirectComposition` (`IDCompositionDevice`, `IDCompositionTarget`, `IDCompositionVisual`) | `Windows.UI.Composition` (`Compositor`, `ContainerVisual`, `SpriteVisual`, `Visual`) — projected through the `windows` crate's `UI::Composition` module |
| Threading | Free-threaded; commits explicit | UI-thread by default; commits batched by the `Compositor` or driven manually via `CompositorController` (`UI::Composition::Core`) — the controlled-commit variant we use |
| Effects pipeline | `IDCompositionEffectGroup`, transforms, filters at the COM level | Composition `Effect` graph (Win2D, brushes, animations); supports DWM backdrops |
| HWND hosting | `IDCompositionDesktopDevice::CreateTargetForHwnd` | `ICompositorDesktopInterop::CreateDesktopWindowTarget` (a Win32 interop interface that returns a WinRT `DesktopWindowTarget`) |
| Used in this crate | **No** | **Yes** — exclusively |

Where in the code:
- `application.rs:6, 26, 38` — `CompositorController::new()` is owned by `Application` and cloned into each `Window` and `AngleDevice`.
- `window.rs:13-14, 25, 66-67, 493-494` — `ContainerVisual` / `SpriteVisual` / `DesktopWindowTarget`, with the HWND bridged via `ICompositorDesktopInterop` (the only Win32-flavoured COM interface in the rendering path).
- `renderer_angle.rs:8, 43, 48` — ANGLE's EGL window surface targets the `SpriteVisual`.

`Win32_Graphics_Dwm` is enabled in `Cargo.toml` (DWM titlebar attributes, extended frame bounds), but `Win32_Graphics_DirectComposition` is **not** enabled. Anyone tempted to "drop down to DComp for X" should first check whether the `Windows.UI.Composition` API (or Win2D) covers the case — the two cannot be mixed naively in the same visual tree.

## Kotlin layer

This document so far focuses on the Rust side. The Kotlin counterpart at `kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/` is covered subsystem-by-subsystem in `SUBSYSTEMS.md`. The conventions that span all classes are described in `FFI_CONVENTIONS.md`. At a glance:

- **Bootstrap.** `KotlinDesktopToolkit.init(...)` loads `desktop_win32_<arch>[+debug].dll` via `System.load` (resolved from the `kdt.library.folder.path` system property) and then initialises the native logger. Init is `AtomicBoolean`-guarded; calling twice is a no-op with a warning. `// todo check that native library version is consistent with Kotlin code` (KotlinDesktopToolkit.kt:19) — there is no version handshake.
- **Lifecycle wrappers.** `Application`, `Window`, `AngleRenderer`, `DataObject`, `DragDropManager` are `AutoCloseable`. They hold an opaque `MemorySegment` returned by Rust and call the matching `*_drop` FFI on `close()`. Most internally expose an `internal inline fun withPointer(block)` to hand the raw segment to a caller without copying.
- **FFI boundary.** Every native call is wrapped in `ffiDownCall { ... }` (Logger.kt) — must wrap **only** the native call itself. Callbacks invoked from Rust into Kotlin go through `ffiUpCall { defaultResult, body }`, which catches every `Throwable` and returns `defaultResult` (Kotlin exceptions never cross into Rust).
- **Marshalling.** `Strings.kt`, `Arrays.kt`, and `Converters.kt` form the support layer used by the wrapper classes. `Arena.ofConfined().use { arena -> ... ffiDownCall { native(...) } ... }` is the canonical scope for native struct allocation.
- **Use-after-close guard.** `DataObject.requireOpen` (DataObject.kt:121-125) is the prototype: an inline helper that throws `IllegalStateException` if the underlying pointer is `MemorySegment.NULL`. The pattern should be applied wherever a Kotlin wrapper holds an opaque Rust pointer with a documented lifetime.
- **Builders for COM objects.** `DataObject.build { … }` is a block-scoped builder that hides the `data_object_create` / `data_object_into_com` lifecycle so callers never see a "half-built" `DataObject`.
- **Two-tier event delivery.** The Rust event handler is one C function pointer per `Application`; on the Kotlin side `Event.fromNative(tag, segment)` reads the tag integer and dispatches to the matching `sealed class Event` subclass (22 variants mirror the Rust `Event` enum 1-to-1).

## Dependencies on Win11 builds / runtime quirks

`utils.rs` exposes two Windows version probes via `RoIsApiContractPresent`:
- `is_windows_11_build_22000_or_higher` — gates dark-mode DWM titlebar API.
- `is_windows_11_build_22621_or_higher` — gates Mica / Acrylic via `DWMWA_SYSTEMBACKDROP_TYPE`.

ANGLE depends on a co-located `libEGL.dll`; `GetModuleFileNameW(get_dll_instance())` is used to resolve the path. No fallback if it's missing.

The crate uses `windows` crate v0.62.x, `windows-core` 0.62.x, `khronos-egl` 6.0 (dynamic), and `papaya` 0.2 (concurrent hash map, used by `data_object` and `data_object_api`).
