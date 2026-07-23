# Win32 IME (Input Method Editor) Support — Design

Status: implemented · Crate: `native/desktop-win32` · Backend: IMM32 · Kotlin API: pull `TextInputClient`

This document is self-contained. It explains how Windows IME input works, the baseline from which
the `desktop-win32` integration was built, the implemented design, and its verification strategy.

---

## 1. Purpose & scope

Let users type Chinese, Japanese, Korean (and other IME-composed) text into a window whose
text content is rendered entirely by the Kotlin application — there is no native edit control.

The application is modeled as a **pull text-input client**: it owns its text buffer, answers a
few queries about it (selection, caret rectangle), and receives edits (insert, set-preedit) from
the backend. Pull is the natural model for app-owned text — the app is the single source of truth
for its content and the backend never holds a copy. The interface resembles the macOS backend's
`NSTextInputClient`
([`TextInputClient.kt`](../../../../kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/macos/TextInputClient.kt)),
which is a useful FFI template; the win32 type is independent.

A single invariant runs through the whole design: **the marked (preedit) text lives in the
application's document.** The app renders the in-progress composition inline; the backend only
tells the app what to render and where to put the IME popups. The backend keeps no text buffer.

The design was implemented in two phases:

- **Phase 1** — CJK input works with the IME drawing its own composition and candidate windows.
  Committed and typed characters arrive through `TextInputClient.insertText`; a language-change
  signal fires; IME turns on/off per field via `setImeEnabled`.
- **Phase 2** — the application renders the in-progress composition ("preedit") inline itself: the
  backend reads the composition string and calls `setMarkedText` / `unmarkText`; the IME still draws
  only the candidate list.

This document specifies the **IMM32** implementation (the Input Method Manager: `Imm*` functions +
`WM_IME_*` messages) — the classic, universally-available Win32 IME API, used by winit, SDL, Zed,
Flutter, and GLFW.

---

## 2. Background: how Windows IME input works

Read this section once; the rest of the document assumes it.

### 2.1 The message flow

An IMM32 application owns an `HWND` and a message loop (`GetMessage` → `DispatchMessage`).
When an IME is active and the user presses keys:

1. During message retrieval the OS asks the active IME whether it wants the keystroke. If so,
   the `WM_KEYDOWN` is delivered with `wParam == VK_PROCESSKEY` (`0xE5` / `229`). The real
   virtual key is no longer on the keydown (the interleaved `WM_KEYUP` still carries it).
2. `TranslateMessage`, called on that `VK_PROCESSKEY` keydown, drives the IME to compose. The
   IME posts `WM_IME_*` messages back to the queue.
3. As composition proceeds the IME sends:
   - `WM_IME_STARTCOMPOSITION` — composition began.
   - `WM_IME_COMPOSITION` — the composition changed. `lParam` is a bitmask of `GCS_*` flags
     describing what is available to read.
   - `WM_IME_ENDCOMPOSITION` — composition ended (commit or cancel).
4. On commit, `WM_IME_COMPOSITION` carries `GCS_RESULTSTR` (the finalized text). If the app
   does **not** handle these messages, `DefWindowProc` forwards them to the default IME window;
   its Unicode `WM_IME_CHAR` handling ultimately produces `WM_CHAR` messages. This is the
   "IME-unaware" default used by Phase 1.
5. The candidate list is a separate window the IME draws and manages. The app only positions it.

`TranslateMessage` is what feeds these keystrokes to the IME. Keep the current standard ordering:
call it before `DispatchMessage` for `VK_PROCESSKEY`. `VK_PROCESSKEY` means the IME processed the
keystroke, including a key that begins composition; it is not a shortcut key. Code that needs the
original virtual key must call `ImmGetVirtualKey` before `TranslateMessage`.

### 2.2 Reading the composition string

Inside `WM_IME_COMPOSITION`, get the input context and call `ImmGetCompositionStringW` once per
`GCS_*` flag. Call with a null buffer to get the required size, then again to fill it.

| `GCS_*` flag     | Meaning                                        | Shape |
|------------------|------------------------------------------------|-------|
| `GCS_RESULTSTR`  | Finalized (committed) text                     | UTF-16 string |
| `GCS_COMPSTR`    | In-progress preedit text                       | UTF-16 string |
| `GCS_CURSORPOS`  | Caret position inside the preedit              | scalar (return value), **UTF-16 code units** |
| `GCS_COMPATTR`   | Per-character conversion attributes             | 8-bit `ATTR_*` status values |
| `GCS_COMPCLAUSE` | Clause segment boundaries                      | array of UTF-16 offsets (`u32`) |

Units matter: `ImmGetCompositionStringW` returns **byte** lengths for string and attribute buffers,
and `GCS_COMPCLAUSE` returns `u32` offsets. The backend validates the metadata against the preedit
and normalizes segment ranges to its UTF-16 indexing model. The call returns a signed count and can
fail with `IMM_ERROR_NODATA` (`-1`) or `IMM_ERROR_GENERAL` (`-2`); §7.3 handles those.

A single `WM_IME_COMPOSITION` can carry both `GCS_RESULTSTR` and `GCS_COMPSTR`; this is observed
with Japanese IMEs when one phrase commits as the next begins. Microsoft documents cancellation
as a message with **none of the complete set of `GCS_*` bits set** — not necessarily `lParam == 0`,
because status bits such as `CS_INSERTCHAR` / `CS_NOMOVECARET` may be present on their own. Some
Korean IMEs are observed to deliver `GCS_RESULTSTR` in a `WM_IME_COMPOSITION` after
`WM_IME_ENDCOMPOSITION`; this is compatibility behavior, not a Windows ordering guarantee.
Therefore `WM_IME_COMPOSITION` is always processed independently of `composition_active`.

### 2.3 Positioning, context, focus

- **Position** the IME UI by filling `COMPOSITIONFORM` / `CANDIDATEFORM` and calling
  `ImmSetCompositionWindow` / `ImmSetCandidateWindow` on the input context. Coordinates are
  client-relative **physical** pixels. This works even when the IME draws the UI itself
  (position is a property of the input context). `COMPOSITIONFORM.dwStyle` uses `CFS_POINT`;
  `CANDIDATEFORM.dwStyle` accepts only `CFS_CANDIDATEPOS` or `CFS_EXCLUDE` (never the
  composition styles). `CANDIDATEFORM.dwIndex` is `0`.
- Some IMEs ignore those calls and read the **system caret** via `GetCaretPos`. Placing a 1×1
  system caret (`CreateCaret` / `SetCaretPos`) at the insertion point covers them.
- **Enable/disable** IME per field with `ImmAssociateContextEx(hwnd, NULL, IACE_DEFAULT)` to
  attach the window's default input context and `ImmAssociateContextEx(hwnd, NULL, 0)` to detach
  it. The application never creates its own `HIMC`, so there is no context to destroy.
- **Finalize** an in-progress composition before detaching with
  `ImmNotifyIME(himc, NI_COMPOSITIONSTR, CPS_COMPLETE, 0)`; **cancel** it (discard without
  committing) with `CPS_CANCEL`.

### 2.4 Scope note on the IMM32 mandate

Microsoft's statement that "the system blocks IMEs implemented with IMM32" applies to *authoring*
an IME (a text-input processor DLL). *Consuming* IME input from an application via `Imm*` /
`WM_IME_*` is fully supported on desktop and is what winit, SDL, Zed, Flutter, and GLFW do.

---

## 3. Baseline before implementation

Before this design was implemented, the relevant files and behavior were:

- **`src/win32/event_loop.rs`**
  - `EventLoop::run` — the message pump: `GetMessageW` → `DispatchMessageW`. It calls
    `TranslateMessage` **only** on `VK_PROCESSKEY` key messages (`WM_KEYDOWN`/`WM_KEYUP`/`WM_SYSKEYDOWN`/`WM_SYSKEYUP`)
    so the IME composes; every other message is dispatched untranslated.
  - `EventLoop::window_proc` — the `WM_*` dispatch table. It handles `WM_KEYDOWN`/`WM_KEYUP`,
    `WM_CHAR`, `WM_SETFOCUS`/`WM_KILLFOCUS`, pointer and window messages. It has **no** `WM_IME_*`
    arms — those fall through to `DefWindowProcW`.
  - `on_keyevent` — returns early (drops the key, no `KeyDown`) when the virtual key is
    `VK_PROCESSKEY`; otherwise builds a `KeyEvent`, stashes the raw `MSG` in a thread-local
    `KEYEVENT_MESSAGES` map keyed by an `original_msg_id`, fires `Event::KeyDown`/`KeyUp`, then
    removes the stash entry.
  - `on_char` — dispatched for `WM_CHAR | WM_DEADCHAR | WM_SYSCHAR | WM_SYSDEADCHAR`. It builds a
    `CharacterReceivedEvent` from the low word and flags dead-key / system-key cases.
  - `WM_SETFOCUS` / `WM_KILLFOCUS` — fire `WindowKeyboardEnter` / `WindowKeyboardLeave`.
- **`src/win32/events.rs`** — the `#[repr(C)] Event` enum, event payload structs, and
  `pub type EventHandler = extern "C" fn(WindowId, &Event) -> bool` (return `true` = handled).
  `WindowTitleChangedEvent` carries a Rust-owned `AutoDropStrPtr` freed when the event drops — the
  template for a push event that carries a string.
- **`src/win32/events_api.rs`** — `keyevent_translate_message(msg_id)`: looks up the stashed
  `MSG` and calls `TranslateMessageEx` on it. This is the toolkit's **opt-in** character model:
  the Kotlin consumer decides, per keydown, whether the key produces a `WM_CHAR` by calling
  `Event.KeyEvent.translate()`.
- **`src/win32/window.rs`** — the `Window` struct. The `HWND` lives in an `AtomicPtr`, read via
  `Window::hwnd()`; `Window::get_scale()` is the current DPI scale. Per-window mutable state uses
  `Cell` / `RefCell` fields. `wndproc` handles `WM_NCDESTROY` specially: it reclaims the leaked
  `Weak<Window>` **before the HWND is recycled**; `Window::drop` only logs.
- **`src/win32/window_api.rs`** — per-window downcalls, all shaped
  `window_<verb>(window_ptr: WindowPtr, …)` and run through the `with_window(&window_ptr, "name", |window| { … })`
  helper (which wraps `ffi_boundary`).
- **`src/win32/keyboard.rs`** — `VirtualKey` (`#[repr(transparent)] u16`) and `PhysicalKeyStatus`.
- **`src/win32/geometry.rs`** — `LogicalPoint` / `LogicalSize` / `LogicalRect` and
  `PhysicalPoint` / `PhysicalSize`. `LogicalPoint::to_physical(scale)` and
  `LogicalSize::to_physical(scale)` exist; there is **no** `LogicalRect::to_physical` and no
  `PhysicalRect`.
- **`cbindgen.toml`** — generates the C header (prefix `Native`) consumed by JExtract.
- **`native/desktop-macos/src/macos/text_input_client.rs`** + Kotlin
  [`TextInputClient.kt`](../../../../kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/macos/TextInputClient.kt)
  — the pull client this design mirrors: a struct of callback function pointers passed down, called
  from native. The Kotlin `TextInputClientHolder` binds each interface method to an Arena-bound
  Panama upcall stub — the function pointers carry the client identity, so the table needs no
  separate handle field. Use it as the FFI template.
- **Kotlin** — `Event.kt` (sealed `Event` class; `Event.Companion.fromNative()` matches the
  native enum tag and unpacks the payload), `Window.kt` (window downcall wrappers),
  `Keyboard.kt` (`VirtualKey`, already defines `ImeProcessed = VirtualKey(229)`).
- **`Cargo.toml`** — the `windows` crate feature list. `Win32_Graphics_Gdi`,
  `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`, and `Win32_Globalization`
  are present; `Win32_UI_Input_Ime` is **not**.

At that baseline, the pump already drove the IME (it translated `VK_PROCESSKEY`), so CJK
composition with the IME's own default UI and `WM_CHAR` commit worked. The toolkit integration was
still missing: there were no `WM_IME_*` handlers, pull `TextInputClient`, caret positioning,
per-field enablement, or self-drawn preedit.

---

## 4. Design decisions

1. **IMM32 backend, pull-client API.** IMM32 is the minimal, universally-available API for a
   self-rendered text surface, and matches winit/SDL/Zed/Flutter/GLFW. It sits behind a pull
   `TextInputClient` (a win32 type; its shape resembles the macOS client, but the two stay separate).
   Pull — the app owns its buffer and answers queries, the backend holds no text — is the clean model
   for a self-rendered editor, and it keeps the message-driven IMM32 mechanics contained inside the
   backend rather than leaking into the Kotlin API.

2. **Pull `TextInputClient`.** The application owns its text buffer, answers `selectedRange` /
   `caretRect`, and receives `insertText` / `setMarkedText` / `unmarkText` /
   `discardMarkedText`. Callback inputs are borrowed; POD query results use caller-provided
   out-parameters, matching the macOS/Panama pattern and avoiding platform struct-return ABI rules.
   No callback returns Rust-owned data, so there is nothing for Kotlin to free. The backend
   translates IMM32 events into these calls and pulls the caret rectangle when it needs to position
   the IME UI. `markedRange` and replacement-range parameters are deliberately absent because this
   IMM32 flow never consumes them; reconversion can add them with an actual use case.

3. **Cancel vs. finalize are two distinct client calls.** `unmarkText` **finalizes**: the app
   accepts the marked text it is rendering as committed document text (it does not delete it) —
   matching the macOS meaning where unmark = accept. `discardMarkedText` **cancels**: the app drops
   the tentative preedit. The backend calls `discardMarkedText` when the composition is cancelled
   (Esc), and `unmarkText` when composition is interrupted by blur, disable, or a client swap.

4. **Keep the opt-in character model; drive the IME for IME-owned keys.** Normal and dead keys keep
   flowing through the existing `translate()` downcall. The pump translates `VK_PROCESSKEY` key
   messages so the IME composes, and `on_keyevent` drops `VK_PROCESSKEY` so it never surfaces as a
   `KeyDown`. Only `VK_PROCESSKEY` is translated: it marks a keystroke processed by the IME, and a
   field with IME disabled (detached context, §6.4) does not emit it — so key delivery needs no
   per-field gating, and normal-key character generation stays on the opt-in path. This wording
   describes the current implementation; it does not claim `VK_PROCESSKEY` occurs only after a
   composition has started.

5. **Character routing.** Only a printable, non-control `WM_CHAR` becomes `insertText`. Enter, Tab,
   Backspace, Esc, and arrows stay on the existing key/character paths (the app handles editing and
   commands); dead-key diacritics (`WM_DEADCHAR`) and Alt-mnemonics (`WM_SYSCHAR` /
   `WM_SYSDEADCHAR`) are **never** inserted. They are not consumed merely because a text client is
   active: preserving the existing `CharacterReceived` / `DefWindowProc` path is required for
   Alt-menu and system-key behavior. There is no `doCommand` channel.

6. **Client is per-window; enable is a per-field toggle.** `window_set_text_input_client` registers
   the window's one client — the app routes queries to whichever field it currently has focused.
   `window_set_ime_enabled(true/false)` attaches/detaches the IME as focus moves between text and
   non-text fields (a numeric/password field is `setImeEnabled(false)`).
   Client registration and HIMC association are orthogonal: `window_clear_text_input_client`
   finalizes any composition, drops the client, and removes the compatibility caret, but does not
   detach the `HIMC`. With no client, default window processing preserves the toolkit's existing IME
   behavior. Only an explicit `setImeEnabled(false)` detaches the context.

7. **Coordinates: the client speaks client-logical; the backend scales.** `caretRect(range)` returns
   coordinates relative to the window client area in logical units. The backend scales both corners
   to the client-relative physical pixels IMM32 wants (§6.3); no virtual-desktop conversion is
   involved, so the contract remains valid on mixed-DPI desktops. Ranges are UTF-16 code units. The
   document `selectedRange` identifies the insertion caret; the `selectedRange` passed to
   `setMarkedText` and every composition-segment range are preedit-relative.

8. **Phase 1 lets the IME draw; Phase 2 self-draws the preedit.** Reading `GCS_RESULTSTR` for a
   whole-phrase commit, or `GCS_COMPSTR` for inline preedit, only makes sense once the app owns
   `WM_IME_COMPOSITION` (Phase 2), because owning that message is what both suppresses the IME's
   inline drawing and stops default processing from re-emitting the result as `WM_CHAR`. Phase 2
   suppresses system preedit UI through both the `WM_IME_SETCONTEXT` UI-bit change and ownership of
   composition messages. In Phase 1 the IME draws the preedit and committed characters arrive as
   `WM_CHAR`, which the backend forwards to `insertText`.

9. **`InputLanguageChanged` stays a plain push `Event`.** It is a keyboard-locale signal, not text,
   so it is not part of the text client. It carries the `HKL` plus a resolved locale name (§6.5).

---

## 5. Architecture

The implementation keeps ownership boundaries explicit:

| File | Responsibility |
|---|---|
| `src/win32/text_input_client.rs` | Pure FFI callback ABI: ranges, composition segments and attributes, callback arguments, callback table, and safe wrappers. It does not depend on `ime.rs` or `window.rs`. |
| `src/win32/ime.rs` | IMM32 transport and `HIMC` guard, per-window IME state, composition readers and decoders, snapshot reduction, and focused unit tests. |
| `src/win32/ime_api.rs` | The five exported window downcalls. |
| `src/win32/window.rs` | IME state ownership, HWND-bound client/focus/enable/teardown lifecycle, compatibility caret, and caret-rectangle scaling. |
| `src/win32/event_loop.rs` | Thin dispatch for `WM_IME_*`, `WM_INPUTLANGCHANGE`, character fallback, focus, and DPI messages. |
| `src/win32/events.rs` | Native `InputLanguageChanged` push-event ABI. |
| `kotlin-desktop-toolkit/.../win32/TextInputClient.kt` | Public client model, borrowed-value decoding, and stable holder/upcall stubs. |
| `kotlin-desktop-toolkit/.../win32/Window.kt` | Holder ownership and the five public window operations. |
| `kotlin-desktop-toolkit/.../win32/Event.kt` | Managed language-event variant and decoder. |
| `sample/.../win32/ToyTextInputWin32.kt` | Editable reference client and inline-preedit renderer. |

### 5.1 Module `src/win32/ime.rs`

Holds all IMM32 logic and per-window IME state, keeping `event_loop.rs` a thin dispatcher and
`text_input_client.rs` a pure FFI callback ABI. Contents:

- A RAII input-context wrapper that is the *sole owner* of the raw `HIMC`. Every IMM32 call that
  needs the handle is an inherent method on the guard; the handle itself never leaves the module:

  ```rust
  pub(crate) struct ImmContext {
      hwnd: HWND,
      himc: HIMC,
  }

  impl ImmContext {
      pub(crate) fn get(hwnd: HWND) -> Option<Self> {
          // SAFETY: hwnd is a live window handle for the duration of the returned guard.
          let himc = unsafe { ImmGetContext(hwnd) };
          (!himc.is_invalid()).then_some(Self { hwnd, himc })
      }

      /// Two-call `ImmGetCompositionStringW` transport: probe for the byte size, then fill.
      /// `T` is the natural element type of the payload (`u16` strings, `u8` attributes).
      fn composition_payload<T: Copy + Default>(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<T>> { ... }

      /// `CFS_POINT` composition window at the caret origin; failures are logged, never propagated.
      pub(crate) fn set_composition_window(&self, origin: POINT) { ... }

      /// `CFS_EXCLUDE` candidate window around the caret rect; failures are logged, never propagated.
      pub(crate) fn set_candidate_window(&self, origin: POINT, exclude: RECT) { ... }

      /// Ask the IME to finalize the composition string (`CPS_COMPLETE` / `CPS_CANCEL`).
      pub(crate) fn notify_composition(&self, action: NOTIFY_IME_INDEX) -> bool { ... }
  }

  impl Drop for ImmContext {
      fn drop(&mut self) {
          // SAFETY: this guard pairs exactly one successful ImmGetContext with its original HWND.
          if !unsafe { ImmReleaseContext(self.hwnd, self.himc) }.as_bool() {
              log::warn!("ImmReleaseContext failed");
          }
      }
  }
  ```

- The per-window `ImeState` record and `ClientCallbackGuard` (§5.2).
- The `GCS_*` readers, direct composition-attribute transport, and the composition-apply logic (§7.3).

### 5.2 Per-window state (`window.rs`)

`Window` owns one `ime` field. `ImeState` is `Copy`, so it uses `Cell` rather than `RefCell`: state
transitions cannot panic on a reentrant Kotlin up-call, and callers always copy the callback table
before calling it.

```rust
ime: Cell<ImeState>,

#[derive(Clone, Copy)]
pub(crate) struct ImeState {
    client: Option<TextInputClient>,      // per-window callback table (app routes to its focused field)
    enabled: bool,                        // IME active for the currently focused field
    focused: bool,                        // this window currently holds keyboard focus
    composition_active: bool,             // between START and END in both phases
    app_has_marked_text: bool,            // Phase 2: app currently renders a preedit
    finalizing: bool,                     // suppress nested finalization / composition edits during either CPS path
    composition_revision: u64,            // invalidates an outer apply sequence after reentrant IME/focus state changes
    callback_depth: u32,                  // reject client-lifetime mutation from inside a client up-call
    pending_high_surrogate: Option<u16>,  // Phase 1: joins a split non-BMP WM_CHAR pair
}
```

All fields are private. Reads go through accessors (`enabled_client`, `active_client`,
`is_active`, `is_enabled`, `is_composition_active`, `app_has_marked_text`, `is_finalizing`,
`revision`) and every mutation goes through a transition method, so the revision invariant —
*every composition- or focus-relevant transition advances `composition_revision`* — is enforced by
`ImeState` itself rather than by caller convention:

```rust
impl ImeState {
    // Window creation leaves Windows' default HIMC association untouched, preserving today's
    // IME-enabled behavior. An explicit disable is the only operation that detaches it.
    pub(crate) const fn new() -> Self { ... }

    const fn advance_composition_revision(&mut self) -> u64 {
        self.composition_revision += 1;  // u64 cannot overflow in practice
        self.composition_revision
    }

    pub(crate) const fn reset_pending_surrogate(&mut self) {
        self.pending_high_surrogate = None;
    }

    pub(crate) const fn replace_client(&mut self, client: Option<TextInputClient>) {
        self.client = client;
        self.reset_pending_surrogate();
    }

    pub(crate) const fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.reset_pending_surrogate();
    }

    pub(crate) const fn set_focused(&mut self, focused: bool) {
        let changed = self.focused != focused;
        self.focused = focused;
        self.reset_pending_surrogate();
        if changed {
            self.advance_composition_revision();
        }
    }

    // START ownership: a fresh native composition never starts with an app-side preedit.
    pub(crate) const fn start_composition(&mut self) {
        self.composition_active = true;
        self.app_has_marked_text = false;
        self.reset_pending_surrogate();
        self.advance_composition_revision();
    }

    pub(crate) const fn set_app_marked(&mut self, value: bool) -> u64 {
        self.app_has_marked_text = value;
        self.advance_composition_revision()
    }

    pub(crate) const fn begin_finalizing(&mut self) {
        self.finalizing = true;
        self.advance_composition_revision();
    }

    pub(crate) const fn clear_composition_state(&mut self) -> u64 {
        self.composition_active = false;
        self.app_has_marked_text = false;
        self.finalizing = false;
        self.reset_pending_surrogate();
        self.advance_composition_revision()
    }
}
```

The `HIMC` is never stored — it is fetched via `ImmContext::get` when needed. The text buffer and
selection live in the Kotlin client, not here. The system caret is **not** tracked with a per-window
flag: it is a single per-GUI-thread resource, so only a focused, enabled window with a registered
client owns it (§6.4).

Helpers on `Window` are the only code allowed to touch the private `ime` cell. They are
`pub(crate)` where `event_loop.rs` needs them; sibling modules do not access `window.ime`
directly.

- `enabled_client() -> Option<TextInputClient>` — copies the client table (function pointers are
  `Copy`) when `enabled && client.is_some()`. `WM_IME_*` ownership uses this even during
  focus-transition message ordering.
- `active_client() -> Option<TextInputClient>` — `enabled_client()` only while `focused`.
  Positioning and thread-global caret operations use this stricter gate so an unfocused window
  cannot move another window's caret.
- `ime_focus_gained`, `ime_focus_lost`, `ime_start`, `ime_end`, and `ime_set_app_marked` — thin
  get/transition/set wrappers over the `ImeState` transitions above, used by `event_loop.rs` and
  the `CompositionSink` impl. The pure `ImeState` transitions own surrogate reset and revision
  advancement so client replacement, focus loss, and composition end are unit-testable without an
  HWND.
- `with_enabled_client` / `with_active_client` — increment `callback_depth` around each
  synchronous up-call. Client replacement/clear and enable changes fail while this depth is
  nonzero; notification downcalls remain allowed so a callback can update its buffer and request a
  caret refresh.
- `update_ime_windows()` (§6.3), `set_text_input_client` /
  `set_ime_enabled` (§6.4), `join_surrogate` (§6.2), `create_caret` / `destroy_caret`,
  `finalize_composition`, `clear_composition_state`, and `ime_teardown` (§6.4).

`composition_active` and `app_has_marked_text` are deliberately separate. Phase 1 tracks the
native composition even though the app draws nothing; client replacement, focus loss, disable, and
teardown must still finish it. Phase 2 additionally tracks whether a client-side preedit needs to
be accepted or discarded.

`composition_revision` makes a sequence of individually reentrant callbacks transactional enough
for one `WM_IME_COMPOSITION`: `apply_composition` records the revision, applies each local state
change *before* its corresponding client mutation, and checks the revision after every up-call. A
nested START/END or focus transition advances it, so the stale outer sequence stops without
overwriting the newer state.

Every up-call goes through one of the guarded helpers below; handlers never invoke a copied table
directly:

```rust
pub(crate) struct ClientCallbackGuard<'a>(&'a Cell<ImeState>);

impl<'a> ClientCallbackGuard<'a> {
    pub(crate) fn enter(state: &'a Cell<ImeState>) -> Self {
        let mut ime = state.get();
        ime.callback_depth += 1;
        state.set(ime);
        Self(state)
    }
}

impl Drop for ClientCallbackGuard<'_> {
    fn drop(&mut self) {
        let mut ime = self.0.get();
        ime.callback_depth = ime.callback_depth.checked_sub(1).expect("text input callback depth underflow");
        self.0.set(ime);
    }
}

fn with_enabled_client<R>(&self, f: impl FnOnce(TextInputClient) -> R) -> Option<R> {
    let client = self.enabled_client()?;
    let _guard = ClientCallbackGuard::enter(&self.ime);
    Some(f(client))
}

fn with_active_client<R>(&self, f: impl FnOnce(TextInputClient) -> R) -> Option<R> {
    let client = self.active_client()?;
    let _guard = ClientCallbackGuard::enter(&self.ime);
    Some(f(client))
}
```

### 5.3 The pull client, the FFI, and the language event

**Kotlin `TextInputClient`** — the application implements this; the backend calls it.

```kotlin
public interface TextInputClient {
    // Queries — the backend asks; the app answers from its own buffer.
    public fun selectedRange(): TextRange?
    public fun caretRect(range: TextRange): LogicalRect   // client-relative logical units

    // Mutations — the backend applies IME edits to the app document.
    public fun insertText(text: String)
    public fun setMarkedText(
        text: String,
        selectedRange: TextRange?,          // caret within the preedit (preedit-relative)
        segments: List<TextCompositionSegment>, // preedit-relative
    )

    // Finalize: accept the marked text as if it had been inserted normally.
    public fun unmarkText()
    // Cancel: drop the tentative preedit and leave the document unchanged.
    public fun discardMarkedText()

    public object Noop : TextInputClient {
        override fun selectedRange(): TextRange? = null
        override fun caretRect(range: TextRange): LogicalRect =
            LogicalRect(LogicalPoint.Zero, LogicalSize(0f, 0f))
        override fun insertText(text: String) = Unit
        override fun setMarkedText(
            text: String,
            selectedRange: TextRange?,
            segments: List<TextCompositionSegment>,
        ) = Unit
        override fun unmarkText() = Unit
        override fun discardMarkedText() = Unit
    }
}

public data class TextRange(val location: Long, val length: Long)          // UTF-16 code units
public data class TextCompositionSegment(val range: TextRange, val attribute: TextCompositionAttribute)
public enum class TextCompositionAttribute {
    Input, TargetConverted, Converted, TargetNotConverted, InputError, FixedConverted, Unspecified,
}
```

Ranges are UTF-16 code units; Kotlin `String` is UTF-16, so offsets line up with `String` indices.
There is no `markedRange`, replacement range, `textForRange`, or `characterIndexForPoint`: the
specified IMM32 flow does not consume them. Reconversion support can introduce the smallest required
surface when it is designed.

**FFI** mirrors `native/desktop-macos/src/macos/text_input_client.rs`: a `#[repr(C)]` struct of
function pointers. There is **no handle field** — each pointer is an Arena-bound Panama upcall stub
that already closes over the Kotlin client (`FFI_CONVENTIONS.md` → “`ffiUpCall`”, and the macOS
`TextInputClientHolder`). The internal holder type lives beside the public interface in
`win32/TextInputClient.kt`, but each Kotlin `Window` owns one holder instance and its shared Arena
for the whole window lifetime. The stubs close over that stable holder; its mutable
`textInputClient` selects the current recipient. A range out-value uses a sentinel `location`
(`usize::MAX`) for `null`.

```rust
const NOT_FOUND: usize = usize::MAX;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TextRange {
    pub location: usize, // NOT_FOUND == "none"
    pub length: usize,
}

#[repr(C)]
pub struct InsertTextArgs<'a> {
    pub text: BorrowedUtf8<'a>,
}

#[repr(C)]
pub struct SetMarkedTextArgs<'a> {
    pub text: BorrowedUtf8<'a>,
    pub selected_range: TextRange,    // preedit-relative caret; NOT_FOUND => none
    pub segments: BorrowedArray<'a, TextCompositionSegment>, // preedit-relative; borrowed for the call
}

#[repr(C)]
pub struct CaretRectArgs {
    pub range_in: TextRange,
    pub rect_out: LogicalRect,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TextCompositionSegment {
    pub range: TextRange,      // preedit-relative UTF-16
    pub attribute: TextCompositionAttribute,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum TextCompositionAttribute {
    Input = 0,
    TargetConverted = 1,
    Converted = 2,
    TargetNotConverted = 3,
    InputError = 4,
    FixedConverted = 5,
    Unspecified = 255,
}

pub type SelectedRangeCallback = extern "C" fn(range_out: &mut TextRange);
pub type CaretRectCallback = extern "C" fn(args: &mut CaretRectArgs);
pub type InsertTextCallback = extern "C" fn(args: InsertTextArgs);
pub type SetMarkedTextCallback = extern "C" fn(args: SetMarkedTextArgs);
pub type UnmarkTextCallback = extern "C" fn();
pub type DiscardMarkedTextCallback = extern "C" fn();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TextInputClient {
    pub selected_range:      SelectedRangeCallback,
    pub caret_rect:          CaretRectCallback,
    pub insert_text:         InsertTextCallback,
    pub set_marked_text:     SetMarkedTextCallback,
    pub unmark_text:         UnmarkTextCallback,
    pub discard_marked_text: DiscardMarkedTextCallback,
}
```

Callers use the table through thin wrappers beside it in `text_input_client.rs` that build the
borrowed arguments — mirroring the macOS `TextInputClientHandler`. Text borrows the caller's
`&str` bytes through length-bearing `BorrowedUtf8`; the `segments` slice is likewise borrowed
for the synchronous up-call only. Only the query direction maps the `NOT_FOUND` sentinel to
`Option`; `set_marked_text` passes the sentinel-carrying `TextRange` through unchanged (a `none`
range means the IME shows no composition cursor, §7.3):

```rust
impl TextRange {
    pub(crate) const fn none() -> Self { Self { location: NOT_FOUND, length: 0 } }
    const fn into_option(self) -> Option<Self> {
        if self.location == NOT_FOUND { None } else { Some(self) }
    }
}

impl TextInputClient {
    pub(crate) fn selected_range(self) -> Option<TextRange> {
        let mut out = TextRange::none();
        (self.selected_range)(&mut out);
        out.into_option()
    }

    pub(crate) fn caret_rect(self, range: TextRange) -> LogicalRect {
        let mut args = CaretRectArgs {
            range_in: range,
            rect_out: LogicalRect {
                origin: LogicalPoint::new(0.0, 0.0),
                size: LogicalSize::new(0.0, 0.0),
            },
        };
        (self.caret_rect)(&mut args);
        args.rect_out
    }

    pub(crate) fn insert_text(self, text: &str) {
        (self.insert_text)(InsertTextArgs { text: BorrowedUtf8::new(text) });
    }

    /// A `none` `selected_range` means the IME shows no composition cursor.
    pub(crate) fn set_marked_text(self, text: &str, selected_range: TextRange, segments: &[TextCompositionSegment]) {
        (self.set_marked_text)(SetMarkedTextArgs {
            text: BorrowedUtf8::new(text),
            selected_range,
            segments: BorrowedArray::from_slice(segments),
        });
    }

    pub(crate) fn unmark_text(self)         { (self.unmark_text)(); }
    pub(crate) fn discard_marked_text(self) { (self.discard_marked_text)(); }
}
```

`BorrowedUtf8` and `BorrowedArray` normally flow Kotlin → Rust; here they flow the other way (Rust
builds them, Kotlin reads them for the duration of the up-call). The length-bearing UTF-8 slice does
not silently turn an embedded NUL into empty text. Nothing Rust-allocated crosses that the Kotlin
side has to free.

Callback lifetime/order is part of the ABI contract:

1. `setTextInputClient(newClient)` calls the native registration/replacement downcall while the
   holder still points at the outgoing client, so native finalization reaches the correct recipient;
   only after that downcall succeeds does Kotlin assign `holder.textInputClient = newClient`.
2. `clearTextInputClient()` calls the native clear while the holder still points at the outgoing
   client, then assigns `TextInputClient.Noop`. It does **not** close the Arena.
3. `Window.destroy()` lets `WM_NCDESTROY` clear the native callback table before `Window.close()`
   closes the holder Arena. `Window.close()` must be idempotent and must drop the native `Window`
   before closing the Arena, so no native object retains a freed stub.
4. Native client-replacement, clear, and enable/disable downcalls reject calls made from inside a
   `TextInputClient` up-call (`callback_depth != 0`). This prevents the handler from continuing with
   a table whose logical recipient changed midway through one IMM message. Selection/layout
   notification downcalls remain reentrant and may synchronously query the caret.
5. `window_destroy` performs the same callback-depth check before `DestroyWindow`. Kotlin
   `Window.close()` first calls the native client-clear downcall and stops if it fails; only then does
   it call `window_drop` and close the Arena. A callback therefore cannot destroy the HWND or free
   the currently executing stub.

`InputLanguageChanged` stays in the `Event` enum, at its alphabetical position. (Review decision,
2026-07-17: native `Event` tag values are **not** ABI-stable — the C header, JExtract Java, and
Kotlin `fromNative` all regenerate in lockstep and there are no external header consumers, so no
append-only ordering is required.) Like `WindowTitleChanged`, its string is a Rust-owned `AutoDropStrPtr` freed when the
event drops after the synchronous handler call:

```rust
pub enum Event {
    // … existing variants …
    InputLanguageChanged(InputLanguageChangedEvent),
}

#[repr(C)]
pub struct InputLanguageChangedEvent {
    pub hkl: usize,                  // the input-locale handle (HKL); LOWORD is the LANGID
    pub locale_name: AutoDropStrPtr, // resolved BCP-47 name (e.g. "ja-JP"), empty if unresolved
}

impl From<InputLanguageChangedEvent> for Event {
    fn from(value: InputLanguageChangedEvent) -> Self {
        Self::InputLanguageChanged(value)
    }
}
```

**Downcalls** (`ime_api.rs`, mirroring the `window_*` / `with_window` convention):

```rust
#[unsafe(no_mangle)]
pub extern "C" fn window_set_text_input_client(window_ptr: WindowPtr, client: TextInputClient) {
    with_window(&window_ptr, "window_set_text_input_client", |window| {
        window.set_text_input_client(Some(client))
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_clear_text_input_client(window_ptr: WindowPtr) {
    with_window(&window_ptr, "window_clear_text_input_client", |window| {
        window.set_text_input_client(None)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_ime_enabled(window_ptr: WindowPtr, enabled: bool) {
    with_window(&window_ptr, "window_set_ime_enabled", |window| {
        window.set_ime_enabled(enabled)
    });
}

// The app calls these when the caret / selection moves or the layout reflows (§6.3).
#[unsafe(no_mangle)]
pub extern "C" fn window_notify_selection_changed(window_ptr: WindowPtr) {
    with_window(&window_ptr, "window_notify_selection_changed", |window| {
        window.update_ime_windows();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_notify_layout_changed(window_ptr: WindowPtr) {
    with_window(&window_ptr, "window_notify_layout_changed", |window| {
        window.update_ime_windows();
        Ok(())
    });
}
```

### 5.4 Cargo features

The `windows` dependency feature list in `native/desktop-win32/Cargo.toml` includes
`"Win32_UI_Input_Ime"`. This feature enables the whole `Windows::Win32::UI::Input::Ime`
module (functions, `COMPOSITIONFORM`/`CANDIDATEFORM`, `GCS_*`, `ATTR_*`, `IACE_DEFAULT`,
`NI_COMPOSITIONSTR`, `CPS_COMPLETE`, `CPS_CANCEL`, `ISC_SHOWUICOMPOSITIONWINDOW`). Everything else
this design uses is already enabled: the caret functions `CreateCaret` / `SetCaretPos` /
`DestroyCaret` live in **`Win32_UI_WindowsAndMessaging`** (`CreateCaret`'s `HBITMAP` parameter is
additionally cfg-gated on `Win32_Graphics_Gdi`, which is present);
`GetKeyboardLayoutNameW` is in `Win32_UI_Input_KeyboardAndMouse`; and `LCIDToLocaleName` is in
`Win32_Globalization`. No composition-font APIs are used, so no further features are needed.

---

## 6. Phase 1 — working IME with system-drawn UI

Goal: CJK typing works; the IME draws its own composition and candidate windows near the caret;
committed and typed characters arrive through `insertText`; `setImeEnabled` controls per-field IME
association independently of client registration; the client is notified of input-language changes.

### 6.1 Key delivery

Key delivery needs no Phase-1 code. The message pump translates `VK_PROCESSKEY` key messages so the
IME composes, and `on_keyevent` drops `VK_PROCESSKEY` so it never surfaces as a `KeyDown` (§3). Only
`VK_PROCESSKEY` is translated; normal and dead keys stay on the opt-in `translate()` downcall.

### 6.2 Committed / typed characters (`event_loop.rs::on_char`)

`on_char` is dispatched for `WM_CHAR | WM_DEADCHAR | WM_SYSCHAR | WM_SYSDEADCHAR`, so it must
discriminate. With an active text client, only a printable `WM_CHAR` becomes inserted text. Every
other case keeps flowing to the existing `CharacterReceived` event and, when unhandled, to
`DefWindowProc`; this preserves Alt/menu behavior. Any non-text message also interrupts a pending
surrogate pair.

```rust
fn on_char(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if window.active_client().is_none() {
        return character_received(event_loop, window, msg, wparam, lparam); // existing path, no client
    }
    if msg != WM_CHAR {
        window.clear_pending_surrogate();
        return character_received(event_loop, window, msg, wparam, lparam); // never inserted
    }
    let unit = LOWORD!(wparam.0);
    // Control units (Enter, Tab, Backspace, ...) must still reach the app as CharacterReceived,
    // never as an insertText edit.
    if matches!(unit, 0x00..=0x1F | 0x7F..=0x9F) {
        window.clear_pending_surrogate();
        return character_received(event_loop, window, msg, wparam, lparam);
    }
    if let Some(text) = window.join_surrogate(unit) { // Some once a full scalar is ready
        let _ = window.with_active_client(|client| client.insert_text(&text));
    }
    Some(LRESULT(0))
}
```

`character_received` is the current `on_char` body (builds `CharacterReceivedEvent`, fires it).

`ImeState::join_surrogate` joins a non-BMP character split across two `WM_CHAR` messages so
`insert_text` always gets a whole scalar, and defines the edge cases (`Window::join_surrogate` is
the get/transition/set wrapper):

```rust
pub(crate) fn join_surrogate(&mut self, unit: u16) -> Option<String> {
    let pending = self.pending_high_surrogate.take();
    if (0xD800..=0xDBFF).contains(&unit) {
        // high surrogate: stash and wait; an unpaired previous high surrogate is dropped
        self.pending_high_surrogate = Some(unit);
        return None;
    }
    if (0xDC00..=0xDFFF).contains(&unit) {
        // low surrogate: valid only after a high surrogate; a lone low surrogate is ignored
        return pending.map(|high| String::from_utf16_lossy(&[high, unit]));
    }
    // BMP unit: any stashed high surrogate was unpaired and is discarded by the take above
    Some(String::from_utf16_lossy(&[unit]))
}
```

The pending high surrogate is cleared on any non-text/control message, client switch, focus loss,
disable, and composition end (§6.4, §7.2) so a stale half can never join across an input boundary.

### 6.3 Positioning (`window.rs`)

The backend has no stored caret — it **pulls** a client-relative logical rectangle, then scales it
to the client-relative physical pixels IMM32 wants. Both query callbacks run under one callback
guard; `callback_depth` blocks client/enable mutation while they run, and the revision check after
the cycle catches any composition or focus transition a callback pumped synchronously before this
window touches the thread-global caret or its input context:

```rust
pub(crate) fn update_ime_windows(&self) {
    // Candidate UI follows the insertion caret, never the whole preedit.
    let revision = self.ime_revision();
    let caret_rect = self
        .with_active_client(|client| client.selected_range().map(|range| client.caret_rect(range)))
        .flatten();
    let Some(caret_rect) = caret_rect else {
        return;
    };
    if self.ime_revision() != revision {
        return;
    }
    let Some(context) = ImmContext::get(self.hwnd()) else {
        log::warn!("active IME client has no input context; skipping positioning");
        return;
    };

    let caret = client_logical_to_physical_rect(caret_rect, self.get_scale());
    let origin = POINT { x: caret.left, y: caret.top };
    // Composition window: a point at the caret. Candidate window: exclude the whole caret
    // rectangle so the list never overlaps the caret line.
    context.set_composition_window(origin);
    context.set_candidate_window(origin, caret);

    // Keep the 1x1 system caret on the insertion point for GetCaretPos-reading IMEs. This shim is
    // best effort: its failure must not roll back a valid HIMC association or client registration.
    // SAFETY: active_client requires focus, and lifecycle code attempted to create the thread caret.
    if let Err(err) = unsafe { SetCaretPos(origin.x, origin.y) } {
        log::warn!("SetCaretPos failed: {err}");
    }
}
```

There is no `LogicalRect::to_physical`, so scale both corners with the existing
`LogicalPoint::to_physical`. Converting the bottom-right corner independently preserves the full
`CFS_EXCLUDE` rectangle under fractional scale factors. The helper lives next to its sole caller
in `window.rs`:

```rust
fn client_logical_to_physical_rect(rect: LogicalRect, scale: f32) -> RECT {
    let top_left = rect.origin.to_physical(scale);
    let bottom_right = LogicalPoint::new(
        rect.origin.x.0 + rect.size.width.0,
        rect.origin.y.0 + rect.size.height.0,
    )
    .to_physical(scale);
    RECT {
        left: top_left.x.0,
        top: top_left.y.0,
        right: bottom_right.x.0,
        bottom: bottom_right.y.0,
    }
}
```

Call `update_ime_windows` from the enable path; from the Phase-1 `WM_IME_STARTCOMPOSITION` arm
(§6.4); from each `WM_IME_COMPOSITION` in Phase 2 (§7.2); from `on_dpichanged`; and from the
`window_notify_selection_changed` / `window_notify_layout_changed` downcalls. Those two downcalls
are how the candidate window tracks the caret while the user moves the caret with the mouse or arrow
keys, scrolls, or reflows — without them the popup goes stale (the backend gets no message when the
app's own selection moves).

### 6.4 Enable, disable, and focus lifecycle (`window.rs`)

Client registration and `set_ime_enabled` are independent: only the latter changes the HIMC
association. The system caret (one per GUI thread) is owned only while this window is focused,
IME-enabled, and has a registered client.

`Window::new` initializes `ime` with `ImeState::new()` and does not call `ImmAssociateContextEx`.
This preserves the existing Win32 behavior: Windows' automatically associated default `HIMC`
remains attached, so IME is initially enabled. `set_ime_enabled(false)` explicitly detaches that
context; a later `set_ime_enabled(true)` restores the thread's default context with `IACE_DEFAULT`.

```rust
pub(crate) fn set_text_input_client(&self, client: Option<TextInputClient>) -> anyhow::Result<()> {
    let current = self.ime.get();
    current.ensure_mutation_allowed("text input client change")?;
    if current.is_composition_active() {
        self.finalize_composition()?; // finish while the outgoing callback table is still valid
    }

    // Finalization can synchronously reenter the wndproc and change focus. Derive caret ownership
    // from fresh state after the outgoing-client callbacks finish.
    let current = self.ime.get();
    let was_active = current.is_active();
    let mut ime = current;
    ime.replace_client(client);
    self.ime.set(ime);

    let is_active = ime.is_active();
    if was_active && !is_active {
        if let Err(err) = self.destroy_caret() {
            log::warn!("DestroyCaret failed after clearing text input client: {err}");
        }
    } else if !was_active
        && is_active
        && let Err(err) = self.create_caret()
    {
        log::warn!("CreateCaret failed after registering text input client: {err}");
    }
    if is_active {
        self.update_ime_windows();
    }
    Ok(())
}

pub(crate) fn set_ime_enabled(&self, enabled: bool) -> anyhow::Result<()> {
    let current = self.ime.get();
    current.ensure_mutation_allowed("IME enablement change")?;
    if current.is_enabled() == enabled {
        return Ok(());
    }
    let hwnd = self.hwnd();
    if enabled {
        // SAFETY: hwnd is the live handle owned by this Window; a null HIMC plus IACE_DEFAULT asks
        // IMM32 to associate the thread's default input context.
        anyhow::ensure!(unsafe { ImmAssociateContextEx(hwnd, HIMC::default(), IACE_DEFAULT) }.as_bool(),
            "ImmAssociateContextEx(IACE_DEFAULT) failed");
        let mut ime = self.ime.get();
        ime.set_enabled(true); // change state only after the OS operation succeeds
        self.ime.set(ime);
        if ime.is_active() {
            if let Err(err) = self.create_caret() {
                log::warn!("CreateCaret failed after enabling IME: {err}");
            }
            self.update_ime_windows();
        }
    } else {
        self.finalize_composition()?;
        // Null HIMC + flags 0 is the de-facto detach idiom (winit, GLFW); Learn documents only
        // the IACE_* flags.
        // SAFETY: hwnd is live.
        anyhow::ensure!(unsafe { ImmAssociateContextEx(hwnd, HIMC::default(), 0) }.as_bool(),
            "ImmAssociateContextEx(detach) failed");
        let mut ime = self.ime.get();
        let destroy_caret = ime.is_active();
        ime.set_enabled(false); // change state only after the OS operation succeeds
        self.ime.set(ime);
        if destroy_caret && let Err(err) = self.destroy_caret() {
            log::warn!("DestroyCaret failed after disabling IME: {err}");
        }
    }
    Ok(())
}

// Resolve any in-progress composition when focus / enable / the client changes. Both CPS paths
// share one shape; only the notify action and the app-side unmark differ. CPS_CANCEL is the
// Phase-2 path (the app renders the preedit: accept it app-side, then discard the IME's own copy
// so a trailing WM_IME_COMPOSITION does not re-commit the same text). CPS_COMPLETE is the
// Phase-1 / idle path: it synchronously reenters WM_IME_COMPOSITION with GCS_RESULTSTR, and the
// finalizing arm of on_ime_composition routes that to insert_text (§7.2).
fn finalize_composition(&self) -> anyhow::Result<()> {
    let current = self.ime.get();
    if current.is_finalizing() || !current.is_composition_active() {
        return Ok(());
    }
    let context = ImmContext::get(self.hwnd()).context("window has no input context")?;
    let mut ime = current;
    ime.begin_finalizing(); // guard both CPS paths before either can synchronously reenter
    self.ime.set(ime);
    let (action, action_name) = if current.app_has_marked_text() {
        let _ = self.with_enabled_client(TextInputClient::unmark_text);
        (CPS_CANCEL, "CPS_CANCEL")
    } else {
        (CPS_COMPLETE, "CPS_COMPLETE")
    };
    let notified = context.notify_composition(action);
    self.clear_composition_state(); // also clears finalizing
    anyhow::ensure!(notified, "ImmNotifyIME({action_name}) failed");
    Ok(())
}

pub(crate) fn clear_composition_state(&self) -> u64 {
    let mut ime = self.ime.get();
    let revision = ime.clear_composition_state();
    self.ime.set(ime);
    revision
}

fn create_caret(&self) -> windows_core::Result<()> {
    // SAFETY: hwnd is live and focused; a null bitmap creates the hidden compatibility caret.
    unsafe { CreateCaret(self.hwnd(), None, 1, 1) }
}

fn destroy_caret(&self) -> windows_core::Result<()> {
    // SAFETY: lifecycle code calls this only while this focused window owns the thread caret.
    unsafe { DestroyCaret() }
}
```

Every focus/client/enable transition attempts the matching caret operation, but caret failures are
best-effort compatibility failures: log them without rolling back a successful client or HIMC
transition. A per-window "caret created" flag is deliberately avoided because `DestroyCaret` acts on
the single thread-global caret, so an unfocused window destroying "its" caret would tear down the
focused window's caret. The active-client gate keeps ownership with the one eligible focused window.

Focus and destruction hook the existing `WM_SETFOCUS` / `WM_KILLFOCUS` arms and the `WM_NCDESTROY`
path:

```rust
// window_proc dispatch table
WM_SETFOCUS  => on_setfocus(self, window),
WM_KILLFOCUS => on_killfocus(self, window),

fn on_setfocus(event_loop: &EventLoop, window: &Window) -> Option<LRESULT> {
    window.ime_focus_gained();
    event_loop.handle_event(window, Event::WindowKeyboardEnter)
}

fn on_killfocus(event_loop: &EventLoop, window: &Window) -> Option<LRESULT> {
    if let Err(err) = window.ime_focus_lost() {
        log::warn!("IME focus-loss update failed: {err:#}");
    }
    event_loop.handle_event(window, Event::WindowKeyboardLeave)
}
```

`ime_focus_lost` finalizes the composition before recording the loss of focus. Finalization can
synchronously reenter the window procedure and even restore focus, so the method then re-reads
`GetFocus`, updates `ImeState` from that result, and destroys the thread-global caret only if this
window is still unfocused.

**Teardown must precede weak reclamation inside `WM_NCDESTROY`.** The IME context, composition, and
caret depend on a live `HWND`. Keep `WINDOW_PTR_PROP_NAME` installed while `ime_teardown()`
finalizes/cancels the composition: `ImmNotifyIME` may synchronously deliver `WM_CHAR` or `WM_IME_*`,
and those nested messages must still recover the `Window`. Only then remove the property and reclaim
the leaked `Weak<Window>`, before the HWND can be recycled. This cannot be deferred to `Window::drop`,
which may run after the HWND is gone.

```rust
pub(crate) fn ime_teardown(&self) {
    if self.ime.get().is_composition_active()
        && let Err(err) = self.finalize_composition()
    {
        log::warn!("finalizing IME during teardown failed: {err:#}");
    }
    let ime = self.ime.get();
    if ime.is_enabled() {
        // Null HIMC + flags 0 is the de-facto detach idiom (winit, GLFW); Learn documents
        // only the IACE_* flags.
        // SAFETY: teardown runs before WM_NCDESTROY releases the live HWND.
        if !unsafe { ImmAssociateContextEx(self.hwnd(), HIMC::default(), 0) }.as_bool() {
            log::warn!("ImmAssociateContextEx(detach during teardown) failed");
        }
        if ime.is_active()
            && let Err(err) = self.destroy_caret()
        {
            log::warn!("DestroyCaret during IME teardown failed: {err}");
        }
    }
    // No state reset afterwards: WM_NCDESTROY removes the window property right after this
    // returns, so nothing can observe the record again.
}
```

Register the client once when the window has a text-capable field; clear it when the window no
longer has a text recipient. Neither operation changes the HIMC association. Call
`set_ime_enabled(false)` for a field that must not invoke IME (for example, a field restricted to raw
shortcut input), and `true` to restore the default context. The 1×1 caret is invisible (the app
draws its own cursor); it exists only so caret-reading IMEs position correctly.

The Phase-1 `WM_IME_STARTCOMPOSITION` arm records the native composition, positions the IME's own
composition window at the caret, then forwards so the IME still draws it. `start_composition`
clears `app_has_marked_text` — a fresh native composition never starts with an app-side preedit:

```rust
WM_IME_STARTCOMPOSITION => {
    if window.active_client().is_some() {
        window.ime_start();
        window.update_ime_windows();
    }
    None // forward to DefWindowProcW; the IME draws the composition
}
WM_IME_ENDCOMPOSITION => {
    window.clear_composition_state();
    None // forward; Phase 1 owns no preedit but must not leave composition_active stale
}
```

### 6.5 Language change (`event_loop.rs`)

The dispatch arm handles `WM_INPUTLANGCHANGE`, whose `lParam` is the new `HKL`. The event carries
it plus a resolved locale name so the app does not have to decode a bare `LANGID`. Fire the event
(discarding the handled result) and return `None` so `DefWindowProc` still activates the new locale.

```rust
// window_proc dispatch table
WM_INPUTLANGCHANGE => on_inputlangchange(self, window, lparam),

fn on_inputlangchange(event_loop: &EventLoop, window: &Window, lparam: LPARAM) -> Option<LRESULT> {
    let hkl = lparam.0.cast_unsigned();   // the input-locale handle
    let langid = u32::from(LOWORD!(hkl)); // LOWORD(HKL) = LANGID / LCID
    let locale_name = RustAllocatedStrPtr::allocate(resolve_locale_name(langid))
        .inspect_err(|err| log::error!("Failed to allocate the locale name: {err:?}"))
        .unwrap_or_else(|_| RustAllocatedStrPtr::null())
        .to_auto_drop();
    let _ = event_loop.handle_event(window, InputLanguageChangedEvent { hkl, locale_name });
    None
}

// BCP-47 name for the locale (e.g. "ja-JP"); empty if the LCID has no name.
fn resolve_locale_name(langid: u32) -> String {
    let mut buffer = [0u16; LOCALE_NAME_MAX_LENGTH as usize];
    // SAFETY: the buffer is writable for LOCALE_NAME_MAX_LENGTH UTF-16 units; a LANGID is a
    // valid SORT_DEFAULT LCID because its high word is zero.
    let length = unsafe { LCIDToLocaleName(langid, Some(buffer.as_mut_slice()), 0) };
    // The returned length counts the trailing NUL.
    if let Ok(length) = usize::try_from(length)
        && length > 1
    {
        String::from_utf16_lossy(&buffer[..length - 1])
    } else {
        String::new()
    }
}
```

`GetKeyboardLayoutNameW` is the alternative resolver when the raw KLID string is wanted instead of a
BCP-47 name.

`WM_INPUTLANGCHANGE` is sent to the topmost affected window when that application/thread's input
locale changes, IME or not (e.g. US → German); `DefWindowProc` propagates it to child windows. This
is why the event is named generally and stays separate from the text client. It does **not** report
IME sub-mode changes such as hiragana ↔ katakana within one Japanese IME.

### 6.6 Kotlin surface (Phase 1)

- `Window.kt`: `setTextInputClient(client: TextInputClient)`, `clearTextInputClient()`,
  `setImeEnabled(enabled: Boolean)`, `notifySelectionChanged()`, `notifyLayoutChanged()` — bind the
  Kotlin client to the native callback table (via a shared `Arena`, as the macOS
  `TextInputClientHolder` does) and call the downcalls. The `Arena` holding the upcall stubs must
  outlive the native table: clearing resets the stable holder to `Noop`; only `Window.close()` closes
  the Arena, after native callback teardown.
- `Event.kt`: add `InputLanguageChanged(hkl: Long, localeName: String)` to the sealed `Event` class
  and its case in `fromNative()`.

At the end of Phase 1, an application registers a `TextInputClient` for its window, answers
`selectedRange` / `caretRect`, and calls `notifySelectionChanged` / `notifyLayoutChanged` as the
caret moves. IME starts enabled because the window retains its default context; explicit
`setImeEnabled(false)` is reserved for fields that must detach it. The IME's own composition and
candidate windows track the caret, and committed CJK text arrives as `insertText` calls.

---

## 7. Phase 2 — inline (self-drawn) composition

Goal: the application renders the in-progress composition inline with clause styling and a
composition caret; the backend reads composition state and calls `setMarkedText` / `insertText` /
`unmarkText` / `discardMarkedText`; the IME still draws only the candidate list.

Phase 2 is additive — it adds message handlers without changing the client interface or the
Phase 1 downcalls. **Every arm that owns a message first checks `enabled_client()`; with no
IME-enabled, self-drawing client it returns `None` and falls through to `DefWindowProcW` unchanged.**
Owning these messages unconditionally would suppress both the system-drawn composition and the
committed-text `WM_CHAR` fallback for windows that have no text field — silently losing input.

### 7.1 Suppress the IME composition window (`WM_IME_SETCONTEXT`)

With an enabled client, clear the composition-window bit and forward, keeping the candidate-window
bit set so the IME still draws candidates. With no enabled client, fall through untouched.

```rust
fn on_ime_setcontext(window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if window.enabled_client().is_none() {
        return None; // let the system manage its own UI bits
    }
    let lparam = LPARAM(lparam.0 & !(ISC_SHOWUICOMPOSITIONWINDOW as isize));
    // SAFETY: all arguments came from this live window's wndproc; only the documented UI bit changed.
    Some(unsafe { DefWindowProcW(window.hwnd(), msg, wparam, lparam) })
}
```

### 7.2 Composition lifecycle

```rust
WM_IME_SETCONTEXT       => on_ime_setcontext(window, msg, wparam, lparam),
WM_IME_STARTCOMPOSITION => on_ime_startcomposition(window),
WM_IME_COMPOSITION      => on_ime_composition(window, lparam),
WM_IME_ENDCOMPOSITION   => on_ime_endcomposition(window),
```

In Phase 2 the `WM_IME_STARTCOMPOSITION` arm is upgraded from its Phase-1 form (§6.4): with an enabled
client it records `composition_active` and **owns** the message so the IME does not draw the preedit
inline. `WM_IME_ENDCOMPOSITION` discards any client-side preedit left without a result, then clears
native lifecycle state. A later `GCS_RESULTSTR` is still accepted as a compatibility path for IMEs
observed to send the result after END.

```rust
fn on_ime_startcomposition(window: &Window) -> Option<LRESULT> {
    window.enabled_client()?; // Phase-1 fallback: the IME draws its own composition
    window.ime_start();
    window.update_ime_windows();
    Some(LRESULT(0)) // own it: suppress inline drawing
}

fn on_ime_endcomposition(window: &Window) -> Option<LRESULT> {
    window.enabled_client()?;
    if window.ime_is_finalizing() {
        return Some(LRESULT(0)); // reentrant result of our own CPS_CANCEL
    }
    if window.ime_end() { // true when app_has_marked_text was still set
        let _ = window.with_enabled_client(TextInputClient::discard_marked_text);
    }
    Some(LRESULT(0))
}
```

`on_ime_composition` owns the message: returning `Some(LRESULT(0))` keeps the IME from drawing the
preedit and from re-emitting the result as `WM_IME_CHAR`/`WM_CHAR` later, when the client may
already be swapped, disabled, or unfocused. That ownership makes the `finalizing` reentries this
window inflicts on itself the only delivery path, so they must be told apart: a `CPS_CANCEL`
reentry (`app_has_marked_text` still set) is consumed without an edit because the client already
unmarked, while a `CPS_COMPLETE` reentry carries `GCS_RESULTSTR` and must route it to the client
synchronously via `apply_finalizing_composition` — otherwise the committed text is silently lost:

```rust
fn on_ime_composition(window: &Window, lparam: LPARAM) -> Option<LRESULT> {
    window.enabled_client()?;
    let finalizing = window.ime_is_finalizing();
    if finalizing && window.ime_app_has_marked_text() {
        // Reentry from our own CPS_CANCEL: the client already unmarked, so consume without an edit.
        return Some(LRESULT(0));
    }
    let Some(context) = ImmContext::get(window.hwnd()) else {
        log::warn!("enabled IME client has no input context; keeping composition ownership");
        return Some(LRESULT(0));
    };
    let Ok(gcs) = u32::try_from(lparam.0) else {
        log::warn!("WM_IME_COMPOSITION carried an invalid negative or oversized lParam");
        return Some(LRESULT(0));
    };
    if finalizing {
        // Reentry from our own CPS_COMPLETE: deliver the result to the client synchronously.
        apply_finalizing_composition(window, &context, gcs);
        return Some(LRESULT(0));
    }
    apply_owned_composition(window, &context, gcs);
    Some(LRESULT(0))
}

// Complete documented GCS set. Status-only CS_* bits must not be mistaken for GCS data.
const GCS_ANY: u32 = GCS_COMPREADSTR.0 | GCS_COMPREADATTR.0 | GCS_COMPREADCLAUSE.0
    | GCS_COMPSTR.0 | GCS_COMPATTR.0 | GCS_COMPCLAUSE.0 | GCS_CURSORPOS.0
    | GCS_DELTASTART.0 | GCS_RESULTREADSTR.0 | GCS_RESULTREADCLAUSE.0
    | GCS_RESULTSTR.0 | GCS_RESULTCLAUSE.0;

// Any of these flags can change app-rendered preedit state even when GCS_COMPSTR itself is absent.
const GCS_PREEDIT_UPDATE: u32 = GCS_COMPSTR.0 | GCS_COMPATTR.0 | GCS_COMPCLAUSE.0
    | GCS_CURSORPOS.0 | GCS_DELTASTART.0;
```

`apply_owned_composition` reads every required value into one `CompositionSnapshot` **before** the
first callback (once START/SETCONTEXT have suppressed the system preedit, default processing
cannot restore a coherent fallback mid-composition; a failed core read logs, keeps ownership, and
recovers on the next update or END), then `apply_composition` delivers the snapshot with direct
calls in a fixed order — commit first, then the new preedit, then positioning. It sets
`app_has_marked_text` before `setMarkedText` and clears all composition state before a
cancellation discard. Commit and empty-preedit callbacks deliberately keep the previous marked
state during the callback, so a nested focus loss still takes the Phase-2 cancel path rather than
re-emitting native text; their state is cleared only after the callback returns unchanged.
`apply_composition` records `composition_revision`, checks it after every client callback, and
abandons the remaining stale steps if a nested START/END or focus transition changed the revision.
Its `CompositionSink` abstraction makes delivery order, state transitions, and reentrant aborts
testable without an `HWND` or Kotlin callback stubs.

### 7.3 `GCS_*` readers and composition-attribute transport (`ime.rs`)

`ImmGetCompositionStringW` returns a **byte** length for buffers and a signed count that can be
`IMM_ERROR_NODATA` (`-1`) or `IMM_ERROR_GENERAL` (`-2`). Empty data is a successful zero, not an
error. Build a complete snapshot before any client callback, and treat attributes/clauses as
optional decoration while result/preedit strings are core data.

The transport is injectable so snapshot logic tests run without Win32. UTF-16 payloads are read
directly into `Vec<u16>` (the generic `composition_payload` transport from §5.1 sizes the buffer
in the payload's natural element type), so string data is never round-tripped through bytes:

```rust
pub(crate) trait CompositionSource {
    /// Raw byte payloads (`GCS_COMPATTR`, `GCS_COMPCLAUSE`).
    fn bytes(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u8>>;
    /// UTF-16 string payloads (`GCS_COMPSTR`, `GCS_RESULTSTR`).
    fn utf16(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u16>>;
    /// `None` when the IME shows no composition cursor.
    fn cursor(&self) -> Option<usize>;
}

impl CompositionSource for ImmContext {
    fn bytes(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u8>> {
        self.composition_payload(which)
    }

    fn utf16(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u16>> {
        self.composition_payload(which)
    }

    // GCS_CURSORPOS is the function's return value (null buffer). A negative value is the
    // documented "cursor not present" state — some IMEs hide the composition cursor — not an IMM
    // error; render such a preedit without a selection instead of failing the whole snapshot.
    fn cursor(&self) -> Option<usize> {
        // SAFETY: this guard owns a valid HIMC; GCS_CURSORPOS returns its scalar in the result.
        let cursor = unsafe { ImmGetCompositionStringW(self.himc, GCS_CURSORPOS, None, 0) };
        usize::try_from(cursor).ok()
    }
}

pub(crate) struct PreeditSnapshot {
    pub(crate) text: String,
    /// `TextRange::none()` when the IME shows no composition cursor.
    pub(crate) selected: TextRange,
    pub(crate) segments: Vec<TextCompositionSegment>,
}

struct CompositionSnapshot {
    result: Option<String>,
    preedit: Option<PreeditSnapshot>,
    cancelled: bool,
}

impl CompositionSnapshot {
    fn read(source: &impl CompositionSource, gcs: u32) -> anyhow::Result<Self> {
        let result = (gcs & GCS_RESULTSTR.0 != 0)
            .then(|| source.utf16(GCS_RESULTSTR).map(|units| String::from_utf16_lossy(&units)))
            .transpose()?;
        let preedit = if gcs & GCS_PREEDIT_UPDATE != 0 {
            let units = source.utf16(GCS_COMPSTR)?;
            let length = units.len();
            let text = String::from_utf16_lossy(&units);
            let selected = source.cursor().map_or_else(TextRange::none, |cursor| TextRange {
                location: cursor.min(length),
                length: 0,
            });
            let segments = match (
                source.bytes(GCS_COMPATTR),
                source.bytes(GCS_COMPCLAUSE).and_then(|bytes| decode_u32_bytes(&bytes)),
            ) {
                (Ok(attrs), Ok(clauses)) => segments_from_parts(&attrs, &clauses, length),
                (Err(err), _) | (_, Err(err)) => {
                    log::warn!("reading IME composition metadata failed: {err:#}");
                    fallback_segments(length)
                }
            };
            Some(PreeditSnapshot { text, selected, segments })
        } else {
            None
        };
        Ok(Self { result, preedit, cancelled: gcs & GCS_ANY == 0 })
    }
}
```

`segments_from_parts` converts `GCS_COMPATTR` status bytes and decoded `GCS_COMPCLAUSE` offsets into
one `TextCompositionSegment` per clause. **All emitted ranges are preedit-relative UTF-16.** A valid
clause array starts at zero, ends at the preedit length, stays in bounds, and is non-descending.
Known IMM32 attributes retain their SDK values (`Input = 0`, `TargetConverted = 1`, `Converted = 2`,
`TargetNotConverted = 3`, `InputError = 4`, `FixedConverted = 5`). Unknown status bytes map to the
synthetic `Unspecified = 255`. Invalid or unreadable attribute/clause metadata produces one
full-preedit `Unspecified` segment. Applications map composition attributes to presentation.

```rust
fn decode_u32_bytes(bytes: &[u8]) -> anyhow::Result<Vec<u32>> {
    anyhow::ensure!(bytes.len().is_multiple_of(size_of::<u32>()), "unaligned u32 byte count: {}", bytes.len());
    Ok(bytes
        .chunks_exact(size_of::<u32>())
        .map(|part| u32::from_ne_bytes([part[0], part[1], part[2], part[3]]))
        .collect())
}

fn composition_attribute_from_raw(value: u8) -> TextCompositionAttribute {
    match u32::from(value) {
        ATTR_INPUT => TextCompositionAttribute::Input,
        ATTR_TARGET_CONVERTED => TextCompositionAttribute::TargetConverted,
        ATTR_CONVERTED => TextCompositionAttribute::Converted,
        ATTR_TARGET_NOTCONVERTED => TextCompositionAttribute::TargetNotConverted,
        ATTR_INPUT_ERROR => TextCompositionAttribute::InputError,
        ATTR_FIXEDCONVERTED => TextCompositionAttribute::FixedConverted,
        _ => TextCompositionAttribute::Unspecified,
    }
}

fn segments_from_parts(attrs: &[u8], clauses: &[u32], preedit_len: usize) -> Vec<TextCompositionSegment> {
    let bounds = clauses.iter().map(|value| usize::try_from(*value)).collect::<Result<Vec<_>, _>>();
    let Ok(bounds) = bounds else {
        return fallback_segments(preedit_len);
    };
    if attrs.len() != preedit_len
        || bounds.len() < 2
        || bounds.first() != Some(&0)
        || bounds.last() != Some(&preedit_len)
        || bounds.iter().any(|value| *value > preedit_len)
        || bounds.windows(2).any(|pair| pair[0] > pair[1])
    {
        return fallback_segments(preedit_len);
    }

    bounds
        .windows(2)
        .filter_map(|pair| {
            let (start, end) = (pair[0], pair[1]);
            if start >= end {
                return None;
            }
            Some(TextCompositionSegment {
                range: TextRange { location: start, length: end - start },
                attribute: composition_attribute_from_raw(attrs[start]),
            })
        })
        .collect()
}

fn fallback_segments(preedit_len: usize) -> Vec<TextCompositionSegment> {
    (preedit_len != 0)
        .then_some(TextCompositionSegment {
            range: TextRange { location: 0, length: preedit_len },
            attribute: TextCompositionAttribute::Unspecified,
        })
        .into_iter()
        .collect()
}
```

### 7.4 DPI

`on_dpichanged` already recomputes the window scale; it calls `window.update_ime_windows()`, which
re-pulls `caretRect` and re-issues the composition/candidate positions at the new scale. No
composition font is set — the font governs only IME-drawn UI, and the application draws the preedit
with its own font.

### 7.5 Kotlin surface (Phase 2)

No new Kotlin API — Phase 2 lights up `setMarkedText` / `unmarkText` / `discardMarkedText` on the
existing `TextInputClient`. The application renders `text` at the caret and styles composition
segments from `segments`, places its composition caret at `selectedRange`, treats `insertText` during
composition as the commit, `unmarkText` as "keep what you are rendering", and `discardMarkedText` as
"drop what you are rendering".

---

## 8. FFI & Kotlin wiring

Rust definitions are the source of truth for the generated layers. They include the callback table
(`TextInputClient`) and its POD companions, the `InputLanguageChangedEvent` variant, and the five
downcalls in `ime_api.rs`. `cbindgen` regenerates the C header and JExtract regenerates the Java
bindings. `BorrowedUtf8`, `BorrowedArray<T>`, and `AutoDropStrPtr` use their existing mappings;
`BorrowedArray<TextCompositionSegment>` monomorphizes to a generated native layout like the other
generic instantiations.

The hand-written Kotlin layer provides the `TextInputClient` interface and stable shared-Arena
holder, the `InputLanguageChanged` event decoder, and the five `Window` methods. Full-build
verification in §9 keeps the Rust → header → Java → Kotlin chain aligned; the repository has no
separate `ffi-sync-checker` or `win32-doc-sync` task.

---

## 9. Verification

Run commands from the repository root. **Required bar:** `cargo test`, `cargo check`, and the full
Gradle build all pass:

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32
cargo check --manifest-path native/Cargo.toml -p desktop-win32
.\gradlew.bat build
```

`build` — not `lint` alone — is the integration bar because it
exercises the full Rust → cbindgen → JExtract → Kotlin binding path (a new callback table and event
variant can pass `lint` yet fail to generate or bind).

**Rust unit tests** live in `#[cfg(test)]` modules inside `src/win32/ime.rs` and
`src/win32/window.rs`, beside the private logic they exercise. Run focused tests during
development with:

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::ime::tests::
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::window::ime_state_tests::
```

IME message delivery itself is not scriptable, but the reducer and parsing logic are. Cover:

- **Range semantics** — document-global `selectedRange` passed unchanged to `caretRect`;
  preedit-relative selection and composition-segment ranges; the `NOT_FOUND` sentinel ↔ `null` round-trip.
- **Surrogate joining** — a BMP unit; a valid high+low pair; a lone high surrogate; a lone low
  surrogate; an interrupted pair; and clearing the pending unit on client switch / focus loss /
  composition end.
- **Composition attributes** — checked `ATTR_*` → `TextCompositionAttribute` mapping; multi-clause
  boundaries; reserved values; and malformed/unavailable metadata fallback to one full-preedit
  `Unspecified` segment.
- **Coordinate conversion** — client-relative logical → client-relative physical, including the
  full-rectangle `CFS_EXCLUDE` corner conversion.
- **Composition state transitions** — start → compose → commit; start → compose → cancel (empty
  full-GCS mask); empty `GCS_COMPSTR`; observed both-flags-in-one-message and post-END-result
  compatibility cases; cursor-, attribute-, and clause-only preedit refresh; core read failure
  preserving ownership and client/native state; and reentrant finalize.

**Kotlin unit tests** cover only the managed boundary: `NOT_FOUND` ↔ `null`, callback argument
decoding, the holder's recipient and arena lifetime, and `InputLanguageChanged` decoding.
They do not duplicate private Rust algorithms. Run the focused class with:

```powershell
.\gradlew.bat :kotlin-desktop-toolkit:generateBindingsForWin32
.\gradlew.bat :kotlin-desktop-toolkit:test --tests "org.jetbrains.desktop.win32.tests.TextInputClientTests"
```

**Manual check** (needs a real IME installed). Run the Win32 Skiko sample, which integrates the
editable `ToyTextInputWin32` client, then:

1. Install a Japanese or Chinese IME (Windows Settings → Language). Register the client and answer
   `selectedRange` / `caretRect`; verify initial typing works without an enable call, then exercise
   explicit `setImeEnabled(false)` / `true` detach and restore.
2. Type romaji/pinyin. With no enabled client, confirm the system-drawn composition and committed
   `WM_CHAR` fallback still work. With an enabled client, confirm the preedit renders inline via
   `setMarkedText`, the candidate window tracks the caret, and committing calls `insertText` with
   the phrase.
3. Press Esc mid-composition and confirm `discardMarkedText` drops the preedit. Move focus away,
   disable IME, and replace or clear the client mid-composition; each finalization path must call
   `unmarkText` on the outgoing client and keep its marked text.
4. Verify observed compatibility cases: a Korean IME that commits after END and a Japanese IME
   that supplies result + new preedit in one composition message.
5. Move the caret with the mouse / arrows and scroll, calling `notifySelectionChanged` /
   `notifyLayoutChanged`, and confirm the candidate window follows.
6. Switch keyboard layout (Alt+Shift) and confirm `InputLanguageChanged` fires with the HKL and
   locale name, including between two non-IME layouts.
7. Move the field to a monitor at a different DPI and confirm the IME windows stay on the caret.
8. Close the window during composition and confirm teardown completes without invoking a freed
   callback stub.

---

## 10. References

Microsoft Learn:

- Input Method Manager overview — https://learn.microsoft.com/windows/win32/intl/about-input-method-manager
- `WM_IME_STARTCOMPOSITION` / `WM_IME_COMPOSITION` / `WM_IME_ENDCOMPOSITION` —
  https://learn.microsoft.com/windows/win32/intl/wm-ime-composition
- `WM_IME_SETCONTEXT` (clearing `ISC_SHOWUICOMPOSITIONWINDOW`) —
  https://learn.microsoft.com/windows/win32/intl/wm-ime-setcontext
- Composition string values (`GCS_*`, `ATTR_*`) —
  https://learn.microsoft.com/windows/win32/intl/ime-composition-string-values
- `ImmGetCompositionStringW` — https://learn.microsoft.com/windows/win32/api/imm/nf-imm-immgetcompositionstringw
- `COMPOSITIONFORM` / `CANDIDATEFORM` —
  https://learn.microsoft.com/windows/win32/api/imm/ns-imm-compositionform ·
  https://learn.microsoft.com/windows/win32/api/imm/ns-imm-candidateform
- `ImmAssociateContextEx` — https://learn.microsoft.com/windows/win32/api/imm/nf-imm-immassociatecontextex
- `ImmNotifyIME` (`CPS_COMPLETE` / `CPS_CANCEL`) — https://learn.microsoft.com/windows/win32/api/imm/nf-imm-immnotifyime
- Input context (association persists across activation) —
  https://learn.microsoft.com/windows/win32/intl/input-context
- `WM_INPUTLANGCHANGE` — https://learn.microsoft.com/windows/win32/winmsg/wm-inputlangchange
- `LCIDToLocaleName` (LANGID → BCP-47) — https://learn.microsoft.com/windows/win32/api/winnls/nf-winnls-lcidtolocalename
- `ImmGetVirtualKey` (`VK_PROCESSKEY` / `TranslateMessage` ordering) —
  https://learn.microsoft.com/windows/win32/api/imm/nf-imm-immgetvirtualkey
- "Using an Input Method Editor in a Game" (canonical self-drawing IMM32 sample) —
  https://learn.microsoft.com/windows/win32/dxtecharts/using-an-input-method-editor-in-a-game

FFI pattern to copy (a callback-table down-call; the win32 client is an independent type):

- macOS — `native/desktop-macos/src/macos/text_input_client.rs` +
  [`TextInputClient.kt`](../../../../kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/macos/TextInputClient.kt).

Open-source references (all IMM32, app-side, own their HWND):

- Zed / GPUI — `crates/gpui_windows/src/{events,platform,window}.rs`
  (self-drawn preedit, `ImmSetCompositionWindow`/`ImmSetCandidateWindow`, `ImmAssociateContextEx`).
- winit — `winit-win32/src/{ime,event_loop,keyboard,keyboard_layout}.rs`.
- SDL3 — `src/video/windows/SDL_windowskeyboard.c`, `SDL_windowsevents.c`.
- Chromium — `ui/base/ime/win/imm32_manager.cc` (system-caret shim; per-language positioning) at
  tag `120.0.6099.5` (not present on `main`).
- Firefox — `widget/windows/IMMHandler.cpp`, `WinIMEHandler.cpp` (native-caret shim; commit path).
