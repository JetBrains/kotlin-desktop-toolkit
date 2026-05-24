# TODO / Known issues

Inventory of confirmed bugs, likely bugs, capability gaps, inline TODOs, smells, and open design questions surfaced during the documentation pass. Sorted by status (confirmed → likely → gap → smell → deferred review). Each item lists the relevant file or function and what to do.

This list is point-in-time. Verify against current code before acting.

## Confirmed bugs (per code-owner review)

### `tryRead*` swallows too many error kinds
- **Where**: `data_object_api.rs` (`IntoFfiOption` impl).
- **What**: Every `Err(...)` from a read is converted to `FfiOption::none()` with a `trace!` log. This hides allocation failures, type mismatches, and other genuine errors behind the same `null` that "format not found" produces.
- **Intended behaviour**: only `DV_E_FORMATETC` and `DV_E_TYMED` (i.e. format-not-available) should swallow to `None`. Everything else should propagate as an exception via `ffi_boundary` → `LAST_EXCEPTION_MSGS` → `NativeError`.
- **Fix**: replace the blanket `IntoFfiOption` with a guard that inspects the error and only swallows the format-not-found variants. Consider matching on `WinError::code()` for `DV_E_FORMATETC` / `DV_E_TYMED`, plus an `Option`-returning helper in `data_object` that returns `Ok(None)` for those cases natively.

## Likely bugs / suspect designs (verify before fixing)

### `Window::drop` doesn't verify HWND destruction
- **Where**: `window.rs` (`impl Drop for Window`).
- **What**: Only logs a trace; doesn't check `hwnd.is_null()` or call `DestroyWindow`. If the `Rc<Window>` drops without a prior `window_destroy`, the HWND leaks (and the window stays visible).
- **Fix**: assert (or call `DestroyWindow` defensively) in `Drop`, or document that `window_destroy` is mandatory before drop and have the Kotlin `AutoCloseable` enforce it.

### Duplicate `PointerDown` events
- **Where**: `event_loop.rs` (`on_pointerupdate` + `on_pointerdown`); also click-counter increment at both sites.
- **What**: A single physical button press can produce both a `WM_POINTERUPDATE` with a button-press change and a dedicated `WM_POINTERDOWN`. Both handlers emit `Event::PointerDown` and update the click counter, leading to a double `PointerDown` and an inflated click count for the same gesture.
- **Fix**: pick one handler as the source of truth for `PointerDown`; have the other one detect and skip the redundant emission.

### `ToUnicodeEx` dead-key vs character distinction lost
- **Where**: `events_api.rs` — `char_count.unsigned_abs()`.
- **What**: `ToUnicodeEx` returns negative when a dead key was stored (no character emitted yet). Collapsing the sign loses the distinction; the caller can't tell "dead key applied" from "regular character emitted".
- **Fix**: branch on the sign before computing the buffer slice; expose the dead-key signal as a separate result variant.

### `DataObject` Kotlin class is not thread-safe
- **Where**: `DataObject.kt` (`requireOpen`) + `close()`.
- **What**: `requireOpen` reads `comInterfacePtr` without synchronisation; `close()` mutates it. Concurrent `close()` + `read*()` is a data race that can produce a use-after-free of the COM ref.
- **Fix**: document single-threaded use and add a single-thread assert in `requireOpen`.

### `EnumDisplayMonitors` aborts on first per-monitor failure
- **Where**: `screen.rs` (`monitor_enum_proc` returns `FALSE` on inner-call failure, which terminates enumeration).
- **What**: A single bad monitor (e.g. detached, transient driver hiccup) makes `screen_list` fail entirely instead of returning the others.
- **Fix**: collect successful entries and skip failures with a `log::warn!`, returning whatever we have. Decide if "no monitors at all" should still error.

### `file_dialog` `filter_map` silently drops items
- **Where**: `file_dialog.rs` — `filter_map(|item| parse_shell_item(&item?).ok())`.
- **What**: Items that fail `GetDisplayName` or UTF-8 conversion are silently elided. Caller sees a shorter result list with no indication anything went wrong.
- **Fix**: at minimum log each skipped item with the shell item's path/CLSID. Consider returning a typed partial-result error if the loss is meaningful.

## Capability gaps

### Caption-button proactive device-loss detection
- **Where**: `composition.rs` — `with_d2d_render_target` device-loss recovery (caption-buttons spec §6.2).
- **What**: idle Custom-titlebar windows don't notice device loss until the next state change rasterises; worst case one frame of stale visuals.
- **Trigger to add it back**: acceptance testing or production telemetry showing noticeable visual glitches under real device loss (driver reset, GPU swap, hardware change).
- **Sketch when implementing**: `std::thread::spawn` is the simplest closure-capable wait — capture `dispatcher_queue.clone()` by move into the thread closure, `WaitForMultipleObjects` on `[device_removed_event, cancel_event]`, dispatch via `DispatcherQueue::TryEnqueue` on the device-removed branch, drop signals the cancel event and joins. Win32 threadpool wait (`CreateThreadpoolWait`) is the documented Microsoft pattern but its `extern "system" fn` callback boundary forces a `*mut c_void` context dance that's unnecessary for our scale (one wait per `D2dContext` per process).
- **Sources**: [`ID3D11Device4::RegisterDeviceRemovedEvent`](https://learn.microsoft.com/windows/win32/api/d3d11_4/nf-d3d11_4-id3d11device4-registerdeviceremovedevent), [Composition native interop](https://learn.microsoft.com/windows/apps/develop/composition/composition-native-interop), spec §6.2.

### Verify `RenderingDeviceReplaced` fires synchronously on the `SetRenderingDevice` caller's thread

- **Assumption**: spec `2026-04-30-win32-caption-buttons-design.md` §6.2 (and §4.1) state that `RenderingDeviceReplaced` fires synchronously on the thread that called `SetRenderingDevice`. Microsoft's Composition native-interop sample is consistent with this (its handler runs on the worker thread that triggered `SetRenderingDevice` from `SetThreadpoolWait`), but no Microsoft doc page documents the thread-affinity contract — `[Threading(Both)]` / `[MarshalingBehavior(Agile)]` are thread-safety attributes, not thread-affinity ones.
- **Probe procedure**: instrument the strip's `RenderingDeviceReplaced` closure (registered inside `CaptionButtonStrip::new` per the spec §6.2) to record `GetCurrentThreadId()` and a "fired during SetRenderingDevice?" flag. Trigger device loss (driver toggle / D3D11 device-lost test). Compare against the UI thread id captured at strip construction.
- **Contingency**: if the probe shows off-UI-thread firing in some configuration, the strip's `WM_NCDESTROY` drop ordering (the WM_NCDESTROY drop ordering in spec §6.2) and the `Send` bound on the callback are still correct, but the spec §6.2 prose claiming the callback is on the UI thread "because the toolkit always invokes `SetRenderingDevice` from the UI thread" must be revised. Maintenance rule for the closure: keep `Send`-correct unless / until the assumption is empirically confirmed.
- **Sources**: spec §6.2.

### Caption-button RTL mirroring
- **Where**: `caption_buttons.rs` (`caption_kind_at_screen` and `StripGeometry::hit_test`); `event_loop.rs` strip consultation in `on_nchittest` / non-client pointer routing.
- **What**: Win32 `WindowStyle` has no layout-direction or `WS_EX_LAYOUTRTL` source, so the caption-buttons strip implements only the LTR layout. Apps that want RTL mirroring (Arabic, Hebrew) get caption buttons on the wrong edge.
- **Sketch when implementing**: add a layout-direction input on `WindowStyle` (Kotlin `WindowStyle.layoutDirection`, FFI plumbing); under `WS_EX_LAYOUTRTL`, either skip strip consultation in `on_nchittest` or anchor strip bounds at `client_rect.left` instead of `client_rect.right`. Add RTL test cases to the visibility/availability table (spec §4.2).
- **Sources**: spec `2026-04-30-win32-caption-buttons-design.md` §4.2 (visibility/availability table), [`WS_EX_LAYOUTRTL`](https://learn.microsoft.com/windows/win32/winmsg/extended-window-styles).

### Caption-button animation cadence — `CommitNeeded` fallback
- **Where**: caption-button strip's `ColorKeyFrameAnimation` (spec §5.2); strip's `compositor_controller.Commit()` callsite (spec §5.5).
- **What**: spec §5.2 / §5.5 commit once at `StartAnimation` and let the system compositor's thread advance the animation. Microsoft does not directly document whether `ColorKeyFrameAnimation` requires per-frame `Commit()` under the controlled-commit `CompositorController` variant; under-commit may produce visible stutter on idle windows (no ANGLE swap to drive frames).
- **Trigger to add it back**: spec §7.3 hover-fade acceptance bullet reports stutter on a window with no active ANGLE rendering.
- **Sketch when implementing**: subscribe to [`CompositorController.CommitNeeded`](https://learn.microsoft.com/uwp/api/windows.ui.composition.core.compositorcontroller.commitneeded) and call `Commit()` from the handler; alternatively run a UI-thread frame ticker for the duration of an in-flight strip animation.
- **Sources**: spec §5.2 / §5.5 / §7.3.

### `WM_NCMOUSEMOVE` fallback if `WM_NCPOINTER*` is missing on a supported config
- **Where**: `event_loop.rs` non-client pointer routing.
- **What**: spec §3.5 takes the `WM_NCPOINTER*` contract as established (`EnableMouseInPointer(true)` + the existing wndproc dispatch merging `WM_*POINTER*` and `WM_NC*POINTER*`). If field instrumentation surfaces a supported configuration where `WM_NCPOINTER*` is not delivered, the strip's hover state will not update.
- **Sketch when implementing**: add a parallel `WM_NCMOUSEMOVE` / `WM_NCMOUSELEAVE` arm that translates the mouse-only message to the same `strip.on_pointer_update(...)` calls the `WM_NCPOINTER*` path uses today; gate it so only one path drives the strip on any given system.
- **Sources**: spec §3.5.

### Win32 system-menu restoration
- **Where**: `event_loop.rs` — no Alt+Space or system-menu `WM_SYSCOMMAND` keyboard-system-command paths for non-system titlebar kinds (`WindowTitleBarKind::Custom` / `WindowTitleBarKind::None`). For `WindowTitleBarKind::Custom`, `WM_NCRBUTTONUP` over `HTCAPTION` still does nothing (no title-bar right-click system menu path). `WindowTitleBarKind::None` does not expose a synthetic `HTCAPTION` drag band.
- **What**: native windows show the system menu on Alt+Space and on right-click of the title-bar drag region (and over caption buttons in some configurations). Non-system titlebar windows lose the Alt+Space keyboard path; `WindowTitleBarKind::Custom` also loses the title-bar right-click path. Users cannot reliably reach Move / Size / Minimize / Maximize / Close from these affordances, and UIA invoke patterns on caption buttons also depend on the system-menu surface for accessibility tools.
- **Sketch when implementing**: `GetSystemMenu(hwnd, FALSE)` + `TrackPopupMenu(... TPM_RIGHTBUTTON | TPM_RETURNCMD ...)` + `SendMessageW(hwnd, WM_SYSCOMMAND, cmd, 0)` is the documented Win32 recipe. Hook `WM_NCRBUTTONUP` for the title-bar drag region; for the strip's hit-test area, add an explicit right-click / non-primary path before or inside the swallow path — current strip code tracks non-primary presses (`track_swallowed_press`) and drains the matching release (`consume_swallowed_release`), so a fall-through doesn't exist today. Also hook `WM_SYSCOMMAND` for `SC_KEYMENU` (Alt+Space). Coordinate with the Close-disable entry above so `SC_CLOSE` state stays aligned with caption-Close availability.
- **Sources**: spec `2026-04-30-win32-caption-buttons-design.md` §2 (out-of-scope handoff), [Microsoft `WM_SYSCOMMAND`](https://learn.microsoft.com/windows/win32/menurc/wm-syscommand), [Microsoft `TrackPopupMenu`](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-trackpopupmenu).

### Tall-mode title bars
- **Where**: caption-button strip — `CaptionButtonMetrics::new` returns a fixed 32 epx button height (spec `2026-04-30-win32-caption-buttons-design.md` §4.5). Spec §3.6 / §5.3 already cover the maximised layout transition (NCCALCSIZE inset, strip Y-shift, glyph swap); only the button-height policy is missing.
- **What**: Windows Terminal opts into the WinUI `AppWindowTitleBar.PreferredHeightOption.Tall` shape: 40 epx windowed, 32 epx maximised. The toolkit has no `WindowTitleBarHeight` enum and no `Standard` / `Tall` opt-in, so apps that want a tall title bar (Terminal-style tab strips, Edge-style chrome) cannot get one.
- **Sketch when implementing**: introduce `WindowTitleBarHeight { Standard, Tall }` on `WindowStyle` (`window_api.rs`, Kotlin `win32/Window.kt`); plumb through FFI; replace the hard-coded 32 epx in `CaptionButtonMetrics::new` with `resolve_button_height(WindowTitleBarHeight, is_maximized)`; extend `on_max_state_change` to recompute metrics + relayout when in Tall mode (the hook already runs on every max-state flip — it just doesn't recompute size today).
- **Sources**: [Microsoft `AppWindowTitleBar.PreferredHeightOption`](https://learn.microsoft.com/windows/windows-app-sdk/api/winrt/microsoft.ui.windowing.titlebarheightoption), [Windows Terminal `MinMaxCloseControl.xaml`](https://github.com/microsoft/terminal/blob/e4e3f08efca9d0ffba330eee12edbcb16897ddcb/src/cascadia/TerminalApp/MinMaxCloseControl.xaml), spec `2026-04-30-win32-caption-buttons-design.md` §4.5.

### Win32 Close-button disable support
- **Where**: Win32 `WindowStyle` (`window_api.rs`, Kotlin `win32/Window.kt`) has `is_minimizable` / `is_maximizable` but no `is_closable`; macOS `Window.Params` already has `isClosable`.
- **What we verified**: Win32 Close availability is not controlled by a `WS_CLOSEBOX`-style window bit. Dynamic Close enablement is controlled through the window menu's `SC_CLOSE` item: `GetSystemMenu(hwnd, FALSE)` provides the per-window system-menu copy, and `EnableMenuItem(..., SC_CLOSE, MF_BYCOMMAND | MF_GRAYED)` disables and grays the command. Raymond Chen documents that Close is the special case where the menu item state controls whether the caption Close button is enabled. `CS_NOCLOSE` also disables Close on the window menu, but it is a window-class style, not a per-window runtime mechanism.
- **Related evidence**: Chromium's `Window::EnableClose` uses `EnableMenuItem(GetSystemMenu(hwnd, false), SC_CLOSE, enable ? MF_ENABLED : MF_GRAYED)` followed by `SetWindowPos(... SWP_FRAMECHANGED ...)`. .NET exposes `VisualStyleElement.Window.CloseButton.Disabled`, so Disabled Close is a real themed caption-button state.
- **Deferred design**: decide a Win32 API surface. Candidate naming, to review against the cross-platform API shape: `WindowStyle.is_closable` / Kotlin `isClosable`, default `true`. Custom caption-button Close availability must stay aligned with the system-menu `SC_CLOSE` state.
- **Must verify during implementation**: the exact repaint/update requirement for this toolkit's custom-frame path after changing `SC_CLOSE` (`DrawMenuBar`, `SetWindowPos(... SWP_FRAMECHANGED ...)`, or both); behavior of Alt+F4 and system-menu Close when `SC_CLOSE` is grayed; interaction with future system-menu restoration and UIA invoke patterns.
- **Sources**: [Microsoft `GetSystemMenu`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getsystemmenu), [Microsoft `EnableMenuItem`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enablemenuitem), [Microsoft Window Class Styles (`CS_NOCLOSE`)](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-class-styles), Raymond Chen's [caption-button enablement post](https://devblogs.microsoft.com/oldnewthing/20100604-00/?p=13803) and [`SC_CLOSE` state post](https://devblogs.microsoft.com/oldnewthing/20110805-00/?p=9963), Chromium [`Window::EnableClose`](https://chromium.googlesource.com/chromium/src/+/51077acd7f613ca7bc38d61ad1d22be2233a6e15/chrome/views/window.cc), and [.NET `VisualStyleElement.Window.CloseButton.Disabled`](https://learn.microsoft.com/en-us/dotnet/api/system.windows.forms.visualstyles.visualstyleelement.window.closebutton.disabled).

### IME support (WM_IME_*)
- **Where**: `event_loop.rs` — no `WM_IME_STARTCOMPOSITION`, `WM_IME_COMPOSITION`, `WM_IME_ENDCOMPOSITION`, `WM_IME_NOTIFY`, `WM_INPUTLANGCHANGE` handlers.
- **What**: IME composed characters currently arrive only via `WM_CHAR` / `WM_DEADCHAR`. The toolkit cannot:
  - Show or position an IME composition window.
  - Inspect the in-progress composition string.
  - Distinguish committed vs. tentative input.
  - React to input-language changes.
- **Impact**: CJK / IME users get the final text but no on-the-fly composition feedback or the ability to render their own composition UI.
- **Fix**: design the IME event surface (candidate `Event` variants to review: `ImeCompositionStart`, `ImeCompositionUpdate`, `ImeCompositionEnd`, `ImeInputLanguageChanged`) and wire `WM_IME_*` handlers in `event_loop.rs`. Decide whether to use the legacy IMM API or the modern Text Services Framework. Coordinate with the macOS / Linux backends if a unified IME API is desired.

### No file-type filter in file dialog
- **Where**: `file_dialog.rs` — `COMDLG_FILTERSPEC` and `IFileDialog::SetFileTypes` not used.
- **What**: Open / save dialogs cannot restrict the file-type dropdown. Verify the intended cross-platform parity against the macOS counterpart before designing the API.
- **Fix**: add a `file_types: BorrowedArray<FileTypeFilter>` (where `FileTypeFilter = { name, pattern }`) parameter to `FileDialogOptions`. Marshal to `COMDLG_FILTERSPEC[]` in `file_dialog.rs` and call `SetFileTypes` before `Show`.

### No WM_DISPLAYCHANGE handler
- **Where**: `event_loop.rs` — message not handled.
- **What**: Monitor topology changes (connect/disconnect/reorder) are invisible. `screen_list` returns stale data until the caller polls again.
- **Fix**: handle `WM_DISPLAYCHANGE` and emit a new `Event::ScreensChanged` variant.

### Asymmetric DPI silently mishandled
- **Where**: `screen.rs` — `dpi_y` retrieved from `GetDpiForMonitor` then discarded.
- **Fix**: surface both axes (or document that the toolkit assumes square DPI and add an assertion / log when they differ).

### Color space and stable monitor UUID
- **Where**: `screen.rs` — `// todo color space?` `// todo stable uuid?`. Fields not on `ScreenInfo`.
- **Fix**: when ready, add color-space metadata (HDR detection, sRGB vs. wide gamut) and a stable monitor identifier (e.g. EDID-derived) so apps can persist per-monitor user state.

### Native library version handshake
- **Where**: `KotlinDesktopToolkit.kt` — `// todo check that native library version is consistent with Kotlin code`.
- **Fix**: expose a `kdt_get_version() -> u32` (or struct) FFI; have Kotlin check on init and refuse to load on mismatch.

### Runtime log level changes
- **Where**: `desktop-common::logger.rs` — `// todo store handler and allow to change logger severity`. The `log4rs::init_config` handle is dropped.
- **Fix**: store the `Handle` in a static so `logger_set_level(...)` can adjust live.

### `cursor_api.rs` is incomplete
- **Where**: `cursor_api.rs` only exposes `cursor_show` / `cursor_hide`. Image-setting FFIs live in `window_api.rs` (`window_set_cursor_from_file` / `window_set_cursor_from_system`).
- **Fix**: either move the per-window cursor setters into `cursor_api.rs` (with the window pointer as a parameter) or accept the split and document why.

## Inline TODOs in the code

| File | Comment |
|---|---|
| `screen.rs` | `// todo color space?` and `// todo stable uuid?` |
| `desktop-common::logger.rs` | `// todo store handler and allow to change logger severity` |
| `KotlinDesktopToolkit.kt` | `// todo check that native library version is consistent with Kotlin code` |

## Performance concerns

### `glFinish` before every `eglSwapBuffers`
- **Where**: `renderer_angle.rs`.
- **What**: Forces the CPU to wait for all GPU work each frame, eliminating CPU/GPU pipelining.
- **Investigation**: confirm whether composition correctness genuinely requires this. If only required for the first frame after a resize, gate it on a "needs-finish" flag.

### Per-call `GetDpiForWindow`
- **Where**: `window.rs` — `Window::get_scale` is uncached.
- **Note**: deliberate to reflect per-monitor DPI changes in real time. Worth measuring under high message-rate scenarios (heavy pointer input, animations).

## Code smells worth reviewing

### `borrow` / `borrow_mut` leaks Box every call (deferred)
- **Where**: `desktop-common::ffi_utils.rs`.
- **Status**: per code-owner — review later. Reading: intentional, gives `&R` from a raw `*mut T` without consuming the box (effectively `Box::leak(Box::from_raw(p))`). Sound under the toolkit's single-thread-of-ownership assumption; type-level safety is by convention.
- **Open question**: whether to formalise the assumption (e.g. `!Send` newtype or a phantom marker) or to refactor to a different pattern (e.g. `Pin<&T>` from a stored `Pin<Box<T>>`).

### Universal lack of `// SAFETY` comments
- **Where**: every `unsafe` block in `data_object.rs`, `data_reader.rs`, `drag_drop.rs`, `screen.rs`, `pointer.rs`, `keyboard*.rs`, `cursor.rs`, `file_dialog.rs`, `window.rs`, `renderer_angle.rs`, `renderer_egl_utils.rs`, `event_loop.rs`, `events_api.rs`, `desktop-common::ffi_utils.rs` (module-wide `#![allow(clippy::missing_safety_doc)]` at line 1), `desktop-common::logger.rs`.
- **Fix (incremental)**: add `// SAFETY:` comments as files are touched. Remove the module-wide allow once the backlog is drained.

### Module-blanket clippy suppressions
- `desktop-common::ffi_utils.rs` — `#![allow(clippy::missing_safety_doc)]`.
- `data_object.rs` and `drag_drop.rs` — `#![allow(clippy::inline_always)]`, `#![allow(clippy::ref_as_ptr)]`. Inherited from windows-core's `implement!` macro expansion; keep but consider documenting why.

### `unsafe { std::env::set_var(...) }` without safety comment
- **Where**: `desktop-common::logger.rs`.
- **What**: `set_var` became `unsafe` in Rust 1.81 due to multi-threaded data-race risk. Called from FFI init, which may run after other threads exist.
- **Fix**: add a safety comment justifying the call (init is called once, before background threads), or move to a build-time `RUST_LIB_BACKTRACE` setting.

### Typo
- `desktop-common::logger.rs` — `"File appender creatrion failed"` (creation).

### `Platform.kt` `INSTANCE` only consumed by macOS
- **Where**: `org.jetbrains.desktop.common.Platform.kt`. Used by `macos/KotlinDesktopToolkit.kt:44+`; Win32 and Linux re-implement `isAarch64()` / library-name resolution locally.
- **Fix**: consolidate the per-platform `KotlinDesktopToolkit.kt` helpers onto a single shared `Platform.INSTANCE`-based path, or move `Platform` into the `macos` package and drop the `common` location.

### `DataFormat.Html` lazy + native call → potential pre-init crash
- **Where**: `DataFormat.kt`. The `Html` lazy property triggers `clipboard_get_html_format_id()` on first read. Accessing it before `KotlinDesktopToolkit.init()` will crash.
- **Fix**: either make `init()` eagerly resolve `DataFormat.Html`, or have the property check `KotlinDesktopToolkit.isInitialized` and throw a clearer error.

### `GetMessageTime` 49-day wrap
- **Where**: `pointer.rs` — `PointerClickCounter::register_click` uses `GetMessageTime()` (i32). Subtraction is wrap-safe via `cast_unsigned()` for differences under 2^31 ms, but a 49-day gap silently mis-classifies clicks.
- **Note**: practically never hit (Windows reboots before this matters), but document.

### `VirtualKey` width inconsistency
- **Where**: Rust `VirtualKey(u16)` (keyboard.rs); FFI `keyboard_get_key_state(vkey: i32)` (keyboard_api.rs); Kotlin `Int` (`Keyboard.kt`).
- **Fix**: pick one width. `u16` matches Win32 `VK_*` constants exactly; `i32` is the JExtract-friendly width. Decide and document.

### Hardcoded CF values in Kotlin
- **Where**: `DataFormat.kt` — `Text = 13`, `FileList = 15`.
- **Note**: Win32 constants are stable, but the linkage to Rust `DataFormat::Text` / `::FileList` is by convention only. A future renumbering on either side wouldn't fail any test.
- **Fix**: query both via FFI helpers (like `clipboard_get_html_format_id()` does), or generate Kotlin constants from the Rust enum.

## Commented-out features

- `events.rs` — `//WindowFocusChange(WindowFocusChangeEvent)` and `//WindowFullScreenToggle(WindowFullScreenToggleEvent)`. The payload struct types are not defined anywhere. Either implement or delete.

## Open design questions

- **`AssertUnwindSafe` applied universally** in `ffi_boundary` (`desktop-common::logger.rs`). Partial mutation after panic unwind is not protected against. Worth deciding whether the toolkit's "panics are unrecoverable" stance is the policy and documenting it, or to add per-callsite `UnwindSafe` bounds.
- **Background-thread panics silently lost** (thread-local `LAST_EXCEPTION_MSGS`). Decide whether to introduce a process-wide fallback channel for panics on dispatcher worker threads.
- **Physical-pixel exceptions in the FFI surface** (see `SUBSYSTEMS.md` → Geometry). Several events and callbacks expose `PhysicalPoint` / `PhysicalSize` directly to managed code. Some of these defensible (multi-monitor screen-space, pre-scale-change events); some are convenience-vs-fidelity tradeoffs that are worth re-evaluating one by one. The clearest candidate for conversion is the `DropTarget` callback `point` parameter (`DragDrop.kt`) — the target window has a well-defined scale and converting at the boundary would save every caller the same arithmetic. Decide per-call whether to convert at the boundary, expose both representations, or keep raw physical.
- **Migrate from `anyhow` to `thiserror` for library-public errors.** The crate currently uses `anyhow::Error` as the unified error type throughout. `thiserror` is the recommended approach for libraries: it produces typed errors with stable variant names, lets callers branch on error kinds, and avoids the per-construction allocation overhead of `anyhow::Error`. `anyhow` is appropriate for binaries and for purely internal helper paths where the caller really doesn't care about the variant — keep it in those niches. Migrate the library-public surface first (anything observable in `*_api.rs` return shapes or surfaced through `LAST_EXCEPTION_MSGS`).

## Documentation TODOs

- Add `// SAFETY:` comments to `unsafe` blocks throughout the crate.
- Document `CursorDisplayCounter` semantics in Kotlin (counter goes negative; visible only when ≥ 0).
- Document the WinRT-Composition-vs-DirectComposition distinction inline in `renderer_angle.rs` / `window.rs` (currently only in `ARCHITECTURE.md`).
- Document `EnableMouseInPointer(true)` process-wide irreversibility prominently — third-party libraries in the same process expecting raw `WM_MOUSE*` will silently break.
