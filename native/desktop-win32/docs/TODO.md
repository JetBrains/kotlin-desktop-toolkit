# TODO / Known issues

Inventory of confirmed bugs, likely bugs, capability gaps, inline TODOs, smells, and open design questions surfaced during the documentation pass. Sorted by status (confirmed → likely → gap → smell → deferred review). Each item lists file:line and what to do.

This list is point-in-time. Verify against current code before acting.

## Confirmed bugs (per code-owner review)

### `tryRead*` swallows too many error kinds
- **Where**: `data_object_api.rs:32-44` (`IntoFfiOption` impl).
- **What**: Every `Err(...)` from a read is converted to `FfiOption::none()` with a `trace!` log. This hides allocation failures, type mismatches, and other genuine errors behind the same `null` that "format not found" produces.
- **Intended behaviour**: only `DV_E_FORMATETC` and `DV_E_TYMED` (i.e. format-not-available) should swallow to `None`. Everything else should propagate as an exception via `ffi_boundary` → `LAST_EXCEPTION_MSGS` → `NativeError`.
- **Fix**: replace the blanket `IntoFfiOption` with a guard that inspects the error and only swallows the format-not-found variants. Consider matching on `WinError::code()` for `DV_E_FORMATETC` / `DV_E_TYMED`, plus an `Option`-returning helper in `data_object` that returns `Ok(None)` for those cases natively.

## Likely bugs / suspect designs (verify before fixing)

### `ComInterfaceRawPtr` lifetime in drag-drop callbacks
- **Where**: `drag_drop.rs:97` — `// TODO: figure out lifetime` on `ComInterfaceRawPtr::new(data_object)?` inside `IDropTarget::DragEnter`. Same pattern in `Drop`.
- **What**: The `IDataObject` reference is valid only for the duration of the COM callback. The `DataObject(rawPtr)` Kotlin wrapper constructed from it can be stored beyond the callback, leaving Kotlin holding a dangling pointer.
- **Options**: (a) document that the `DataObject` passed to drop callbacks is callback-scoped only and add a Kotlin-side guard that closes it on callback return; (b) deep-copy / clone the IDataObject inside the callback so the wrapped pointer is independently AddRef'd.

### `Window::drop` doesn't verify HWND destruction
- **Where**: `window.rs:414-418`.
- **What**: Only logs a trace; doesn't check `hwnd.is_null()` or call `DestroyWindow`. If the `Rc<Window>` drops without a prior `window_destroy`, the HWND leaks (and the window stays visible).
- **Fix**: assert (or call `DestroyWindow` defensively) in `Drop`, or document that `window_destroy` is mandatory before drop and have the Kotlin `AutoCloseable` enforce it.

### `WNDCLASS_INIT` registration race
- **Where**: `window.rs:82-94`.
- **What**: `OnceLock<u16>` checked with `get().is_none()` then populated via `get_or_init`. The two operations are not atomic together; concurrent window creation could call `RegisterClassExW` twice and lose the first atom.
- **Fix**: use `get_or_init` alone (it's race-free) and remove the redundant `is_none` precheck.

### Duplicate `PointerDown` events
- **Where**: `event_loop.rs:432` (`on_pointerupdate`) + `event_loop.rs:490` (`on_pointerdown`); also click-counter increment at both sites (`event_loop.rs:440`, `:490`).
- **What**: A single physical button press can produce both a `WM_POINTERUPDATE` with a button-press change and a dedicated `WM_POINTERDOWN`. Both handlers emit `Event::PointerDown` and update the click counter, leading to a double `PointerDown` and an inflated click count for the same gesture.
- **Fix**: pick one handler as the source of truth for `PointerDown`; have the other one detect and skip the redundant emission.

### `ToUnicodeEx` dead-key vs character distinction lost
- **Where**: `events_api.rs:58` — `char_count.unsigned_abs()`.
- **What**: `ToUnicodeEx` returns negative when a dead key was stored (no character emitted yet). Collapsing the sign loses the distinction; the caller can't tell "dead key applied" from "regular character emitted".
- **Fix**: branch on the sign before computing the buffer slice; expose the dead-key signal as a separate result variant.

### `DataObject` Kotlin class is not thread-safe
- **Where**: `DataObject.kt:121-125` (`requireOpen`) + `close()`.
- **What**: `requireOpen` reads `comInterfacePtr` without synchronisation; `close()` mutates it. Concurrent `close()` + `read*()` is a data race that can produce a use-after-free of the COM ref.
- **Fix**: make access serialised — either document single-threaded use and add an assert, or add a `synchronized` / `AtomicReference` guard. The latter has perf cost on the hot read path; choose based on whether callers really do cross threads.

### `EnumDisplayMonitors` aborts on first per-monitor failure
- **Where**: `screen.rs:50-53` (`monitor_enum_proc` returns `FALSE` on inner-call failure, which terminates enumeration).
- **What**: A single bad monitor (e.g. detached, transient driver hiccup) makes `screen_list` fail entirely instead of returning the others.
- **Fix**: collect successful entries and skip failures with a `log::warn!`, returning whatever we have. Decide if "no monitors at all" should still error.

### `file_dialog` `filter_map` silently drops items
- **Where**: `file_dialog.rs:134` — `filter_map(|item| parse_shell_item(&item?).ok())`.
- **What**: Items that fail `GetDisplayName` or UTF-8 conversion are silently elided. Caller sees a shorter result list with no indication anything went wrong.
- **Fix**: at minimum log each skipped item with the shell item's path/CLSID. Consider returning a typed partial-result error if the loss is meaningful.

### `ScreenInfo.origin` should be `PhysicalPoint`, not `LogicalPoint`
- **Where**: `screen.rs:27` declares `pub origin: LogicalPoint`; assigned at `screen.rs:98` via `LogicalPoint::from_physical(…)`.
- **What**: A monitor's origin on the virtual desktop positions it on a coordinate space that may span multiple displays, each with a different DPI scale. Converting via any single monitor's scale loses precision — `LogicalPoint` for screen-space data is misleading because no one scale applies losslessly. The toolkit's own Geometry-exceptions rule (see `SUBSYSTEMS.md` → Geometry → Exceptions) calls out this case explicitly.
- **Fix**: change the field type to `PhysicalPoint`. Drop the `LogicalPoint::from_physical` conversion at the assignment site; pass the raw physical origin through. Kotlin callers that need logical coordinates within a specific monitor can convert using that monitor's `scale`. Verify no existing Kotlin caller assumes the value is logical.
- **Note**: `ScreenInfo.size` is also currently `LogicalSize`; it's defensible (a single monitor's size in its own logical pixels makes sense), but consider whether the same cross-monitor argument applies — discuss when fixing.

## Capability gaps

### IME support (WM_IME_*)
- **Where**: `event_loop.rs` — no `WM_IME_STARTCOMPOSITION`, `WM_IME_COMPOSITION`, `WM_IME_ENDCOMPOSITION`, `WM_IME_NOTIFY`, `WM_INPUTLANGCHANGE` handlers.
- **What**: IME composed characters currently arrive only via `WM_CHAR` / `WM_DEADCHAR`. The toolkit cannot:
  - Show or position an IME composition window.
  - Inspect the in-progress composition string.
  - Distinguish committed vs. tentative input.
  - React to input-language changes.
- **Impact**: CJK / IME users get the final text but no on-the-fly composition feedback or the ability to render their own composition UI.
- **Fix**: design the IME event surface (likely additional `Event` variants — `ImeCompositionStart`, `ImeCompositionUpdate`, `ImeCompositionEnd`, `ImeInputLanguageChanged`) and wire `WM_IME_*` handlers in `event_loop.rs`. Decide whether to use the legacy IMM API or the modern Text Services Framework. Coordinate with the macOS / Linux backends if a unified IME API is desired.

### No file-type filter in file dialog
- **Where**: `file_dialog.rs` — `COMDLG_FILTERSPEC` and `IFileDialog::SetFileTypes` not used.
- **What**: Open / save dialogs cannot restrict the file-type dropdown. Likely capability gap vs. the macOS counterpart.
- **Fix**: add a `file_types: BorrowedArray<FileTypeFilter>` (where `FileTypeFilter = { name, pattern }`) parameter to `FileDialogOptions`. Marshal to `COMDLG_FILTERSPEC[]` in `file_dialog.rs` and call `SetFileTypes` before `Show`.

### No window_restore (un-maximise) FFI
- **Where**: `window_api.rs` — `window_show` / `window_maximize` / `window_minimize` exist; no restore.
- **Fix**: add `window_restore` calling `ShowWindow(hwnd, SW_RESTORE)`.

### No WM_DISPLAYCHANGE handler
- **Where**: `event_loop.rs` — message not handled.
- **What**: Monitor topology changes (connect/disconnect/reorder) are invisible. `screen_list` returns stale data until the caller polls again.
- **Fix**: handle `WM_DISPLAYCHANGE` and emit a new `Event::ScreensChanged` variant.

### `WindowTitleBarKind::Custom` has no FFI activation path
- **Where**: `window_api.rs:79` (enum exists), but `Window::extend_content_into_titlebar` is `pub(crate)` — no `window_extend_content_into_titlebar` export.
- **Fix**: either expose the FFI function or remove the unreachable enum variant.

### High-contrast appearance not modelled
- **Where**: `event_loop.rs:258-270` filters `WM_SETTINGCHANGE` on `wparam == 0 && lparam == "ImmersiveColorSet"` — high contrast arrives with `wparam` == `SPI_SETHIGHCONTRAST` (non-zero) and is excluded.
- **Fix**: extend `Appearance` to include `HighContrast` (or expose it as a separate signal), and handle `SPI_SETHIGHCONTRAST` in `on_settingchange`.

### Asymmetric DPI silently mishandled
- **Where**: `screen.rs:92-95` — `dpi_y` retrieved from `GetDpiForMonitor` then discarded.
- **Fix**: surface both axes (or document that the toolkit assumes square DPI and add an assertion / log when they differ).

### Color space and stable monitor UUID
- **Where**: `screen.rs:31-32` — `// todo color space?` `// todo stable uuid?`. Fields not on `ScreenInfo`.
- **Fix**: when ready, add color-space metadata (HDR detection, sRGB vs. wide gamut) and a stable monitor identifier (e.g. EDID-derived) so apps can persist per-monitor user state.

### Native library version handshake
- **Where**: `KotlinDesktopToolkit.kt:19` — `// todo check that native library version is consistent with Kotlin code`.
- **Fix**: expose a `kdt_get_version() -> u32` (or struct) FFI; have Kotlin check on init and refuse to load on mismatch.

### Runtime log level changes
- **Where**: `desktop-common::logger.rs:195` — `// todo store handler and allow to change logger severity`. The `log4rs::init_config` handle is dropped.
- **Fix**: store the `Handle` in a static so `logger_set_level(...)` can adjust live.

### `cursor_api.rs` is incomplete
- **Where**: `cursor_api.rs` only exposes `cursor_show` / `cursor_hide`. Image-setting FFIs live in `window_api.rs` (`window_set_cursor_from_file` / `window_set_cursor_from_system`).
- **Fix**: either move the per-window cursor setters into `cursor_api.rs` (with the window pointer as a parameter) or accept the split and document why.

## Inline TODOs in the code

| File:line | Comment |
|---|---|
| `drag_drop.rs:97` | `// TODO: figure out lifetime` (see Likely bugs above) |
| `renderer_angle.rs:146` | `// TODO: 0 on resize` (swap_interval should switch to 0 during resize) |
| `screen.rs:31-32` | `// todo color space?` and `// todo stable uuid?` |
| `desktop-common::logger.rs:195` | `// todo store handler and allow to change logger severity` |
| `KotlinDesktopToolkit.kt:19` | `// todo check that native library version is consistent with Kotlin code` |

## Performance concerns

### `glFinish` before every `eglSwapBuffers`
- **Where**: `renderer_angle.rs:145`.
- **What**: Forces the CPU to wait for all GPU work each frame, eliminating CPU/GPU pipelining.
- **Investigation**: confirm whether composition correctness genuinely requires this. If only required for the first frame after a resize, gate it on a "needs-finish" flag.

### Per-call `GetDpiForWindow`
- **Where**: `window.rs:187-190` — `Window::get_scale` is uncached.
- **Note**: deliberate to reflect per-monitor DPI changes in real time. Worth measuring under high message-rate scenarios (heavy pointer input, animations).

## Code smells worth reviewing

### `borrow` / `borrow_mut` leaks Box every call (deferred)
- **Where**: `desktop-common::ffi_utils.rs:105-112`.
- **Status**: per code-owner — review later. Reading: intentional, gives `&R` from a raw `*mut T` without consuming the box (effectively `Box::leak(Box::from_raw(p))`). Sound under the toolkit's single-thread-of-ownership assumption; type-level safety is by convention.
- **Open question**: whether to formalise the assumption (e.g. `!Send` newtype or a phantom marker) or to refactor to a different pattern (e.g. `Pin<&T>` from a stored `Pin<Box<T>>`).

### Universal lack of `// SAFETY` comments
- **Where**: every `unsafe` block in `data_object.rs`, `data_reader.rs`, `drag_drop.rs`, `screen.rs`, `pointer.rs`, `keyboard*.rs`, `cursor.rs`, `file_dialog.rs`, `window.rs`, `renderer_angle.rs`, `renderer_egl_utils.rs`, `event_loop.rs`, `events_api.rs`, `desktop-common::ffi_utils.rs` (module-wide `#![allow(clippy::missing_safety_doc)]` at line 1), `desktop-common::logger.rs`.
- **Fix (incremental)**: add `// SAFETY:` comments as files are touched. Remove the module-wide allow once the backlog is drained.

### Module-blanket clippy suppressions
- `desktop-common::ffi_utils.rs:1` — `#![allow(clippy::missing_safety_doc)]`.
- `data_object.rs:1-2` and `drag_drop.rs:1-2` — `#![allow(clippy::inline_always)]`, `#![allow(clippy::ref_as_ptr)]`. Inherited from windows-core's `implement!` macro expansion; keep but consider documenting why.

### `unsafe { std::env::set_var(...) }` without safety comment
- **Where**: `desktop-common::logger.rs:157-158`.
- **What**: `set_var` became `unsafe` in Rust 1.81 due to multi-threaded data-race risk. Called from FFI init, which may run after other threads exist.
- **Fix**: add a safety comment justifying the call (init is called once, before background threads), or move to a build-time `RUST_LIB_BACKTRACE` setting.

### Typo
- `desktop-common::logger.rs:181` — `"File appender creatrion failed"` (creation).

### `&Vec<&str>` instead of `&[&str]`
- **Where**: `global_data.rs:100` — `pub fn new_file_list(file_names: &Vec<&str>)`.
- **Fix**: take `&[&str]` (clippy `ptr_arg`).

### `screen_info_drop` has no caller
- **Where**: `screen_api.rs:51`. Kotlin calls `screen_list_drop` (which recursively drops elements via `AutoDropArray`). The single-element drop is exported but never invoked from Kotlin.
- **Fix**: remove if vestigial, or document the use case (e.g. future per-element marshalling).

### `PointerModifiers` flag values only documented in Kotlin
- **Where**: `pointer.rs` + `Pointer.kt:37-40`. Rust `PointerModifiers(u32)` populated via `core::mem::transmute` (`pointer.rs:153`) without named constants.
- **Fix**: add `pub const SHIFT: u32 = 4;` and `pub const CTRL: u32 = 8;` (and any others) on `PointerModifiers`, replacing the transmute with a typed mask.

### `CursorIcon::Unknown` panics
- **Where**: `cursor.rs:52` — `panic!("Can't create Unknown cursor")`. Triggered if the integer 0 ever crosses FFI from Kotlin (Kotlin omits `Unknown`, but the discriminant 0 is reachable).
- **Fix**: either remove `Unknown` (no defaultable variant exists), or treat 0 as a no-op error path and let the Kotlin side enforce the absence of `Unknown`.

### `Platform.kt` `INSTANCE` apparently unused
- **Where**: `org.jetbrains.desktop.common.Platform.kt`. `KotlinDesktopToolkit.kt:38` re-implements `isAarch64()` locally.
- **Investigate**: is `Platform.INSTANCE` consumed by macOS / Linux only? If so, document; if not, delete.

### `DataFormat.Html` lazy + native call → potential pre-init crash
- **Where**: `DataFormat.kt`. The `Html` lazy property triggers `clipboard_get_html_format_id()` on first read. Accessing it before `KotlinDesktopToolkit.init()` will crash.
- **Fix**: either make `init()` eagerly resolve `DataFormat.Html`, or have the property check `KotlinDesktopToolkit.isInitialized` and throw a clearer error.

### `GetMessageTime` 49-day wrap
- **Where**: `pointer.rs:250` — `PointerClickCounter::register_click` uses `GetMessageTime()` (i32). Subtraction is wrap-safe via `cast_unsigned()` for differences under 2^31 ms, but a 49-day gap silently mis-classifies clicks.
- **Note**: practically never hit (Windows reboots before this matters), but document.

### `VirtualKey` width inconsistency
- **Where**: Rust `VirtualKey(u16)` (keyboard.rs:10); FFI `keyboard_get_key_state(vkey: i32)` (keyboard_api.rs:29); Kotlin `Int` (`Keyboard.kt:44`).
- **Fix**: pick one width. `u16` matches Win32 `VK_*` constants exactly; `i32` is the JExtract-friendly width. Decide and document.

### Hardcoded CF values in Kotlin
- **Where**: `DataFormat.kt:9-10` — `Text = 13`, `FileList = 15`.
- **Note**: Win32 constants are stable, but the linkage to Rust `DataFormat::Text` / `::FileList` is by convention only. A future renumbering on either side wouldn't fail any test.
- **Fix**: query both via FFI helpers (like `clipboard_get_html_format_id()` does), or generate Kotlin constants from the Rust enum.

## Commented-out features

- `events.rs:31` — `//WindowFocusChange(WindowFocusChangeEvent)` and `//WindowFullScreenToggle(WindowFullScreenToggleEvent)`. The payload struct types are not defined anywhere. Either implement or delete.

## Open design questions

- **`AssertUnwindSafe` applied universally** in `ffi_boundary` (`desktop-common::logger.rs:312`). Partial mutation after panic unwind is not protected against. Worth deciding whether the toolkit's "panics are unrecoverable" stance is the policy and documenting it, or to add per-callsite `UnwindSafe` bounds.
- **Background-thread panics silently lost** (thread-local `LAST_EXCEPTION_MSGS`). Decide whether to introduce a process-wide fallback channel for panics on dispatcher worker threads.
- **`Platform.kt` orphan** (see smell above) — clarify ownership and use site.
- **Physical-pixel exceptions in the FFI surface** (see `SUBSYSTEMS.md` → Geometry). Several events and callbacks expose `PhysicalPoint` / `PhysicalSize` directly to managed code. Some of these defensible (multi-monitor screen-space, pre-scale-change events); some are convenience-vs-fidelity tradeoffs that are worth re-evaluating one by one. The clearest candidate for conversion is the `DropTarget` callback `point` parameter (`DragDrop.kt:53, 57, 61`) — the target window has a well-defined scale and converting at the boundary would save every caller the same arithmetic. Decide per-call whether to convert at the boundary, expose both representations, or keep raw physical.
- **Migrate from `anyhow` to `thiserror` for library-public errors.** The crate currently uses `anyhow::Error` as the unified error type throughout. `thiserror` is the recommended approach for libraries: it produces typed errors with stable variant names, lets callers branch on error kinds, and avoids the per-construction allocation overhead of `anyhow::Error`. `anyhow` is appropriate for binaries and for purely internal helper paths where the caller really doesn't care about the variant — keep it in those niches. Migrate the library-public surface first (anything observable in `*_api.rs` return shapes or surfaced through `LAST_EXCEPTION_MSGS`).

## Documentation TODOs

- Add `// SAFETY:` comments to `unsafe` blocks throughout the crate.
- Document `CursorDisplayCounter` semantics in Kotlin (counter goes negative; visible only when ≥ 0).
- Document the WinRT-Composition-vs-DirectComposition distinction inline in `renderer_angle.rs` / `window.rs` (currently only in `ARCHITECTURE.md`).
- Document `EnableMouseInPointer(true)` process-wide irreversibility prominently — third-party libraries in the same process expecting raw `WM_MOUSE*` will silently break.
