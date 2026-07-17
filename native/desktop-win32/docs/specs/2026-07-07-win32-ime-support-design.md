# Win32 IME (Input Method Editor) Support — Design & Implementation Plan

Status: implemented · Crate: `native/desktop-win32` · Backend: IMM32 · Kotlin API: pull `TextInputClient`

This document is self-contained. It explains how Windows IME input works, what the
`desktop-win32` backend looks like today, the design, and an ordered implementation plan.
An implementer with no prior context can read it top to bottom and build the feature.

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

The work ships in two phases:

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
| `GCS_COMPATTR`   | Per-character clause attributes (underlining)  | one `ATTR_*` byte per UTF-16 unit |
| `GCS_COMPCLAUSE` | Clause segment boundaries                      | array of UTF-16 offsets (`u32`) |

Units matter: `ImmGetCompositionStringW` returns **byte** lengths for the string buffers (divide
by 2 for the `u16` count), but `GCS_CURSORPOS`, the `GCS_COMPATTR` indices, and the
`GCS_COMPCLAUSE` offsets are all **UTF-16 code-unit** indices. The call returns a signed count and
can fail with `IMM_ERROR_NODATA` (`-1`) or `IMM_ERROR_GENERAL` (`-2`); §7.3 handles those.

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

## 3. Current state of `desktop-win32`

Files an implementer will touch, and what they do today:

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

The pump already drives the IME (it translates `VK_PROCESSKEY`), so CJK composition with the IME's
own default UI and `WM_CHAR` commit works today. What is missing is the toolkit's own integration:
no `WM_IME_*` handlers, no pull `TextInputClient`, no caret positioning, no per-field enable, no
self-drawn preedit.

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
   `setMarkedText` and every underline range are preedit-relative.

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

### 5.1 New module `src/win32/ime.rs`

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
- The `GCS_*` readers, the attribute→underline conversion, and the composition-apply logic (§7.3).

### 5.2 Per-window state (`window.rs`)

Add one field to `Window`. `ImeState` is `Copy`, so use `Cell` rather than `RefCell`: state
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
        underlines: List<UnderlineSegment>, // preedit-relative
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
            underlines: List<UnderlineSegment>,
        ) = Unit
        override fun unmarkText() = Unit
        override fun discardMarkedText() = Unit
    }
}

public data class TextRange(val location: Long, val length: Long)          // UTF-16 code units
public data class UnderlineSegment(val range: TextRange, val style: UnderlineStyle, val targetClause: Boolean)
public enum class UnderlineStyle { Solid, Dotted, Thick }
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
    pub underlines: BorrowedArray<'a, UnderlineSegment>, // preedit-relative; borrowed for the call
}

#[repr(C)]
pub struct CaretRectArgs {
    pub range_in: TextRange,
    pub rect_out: LogicalRect,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnderlineSegment {
    pub range: TextRange,      // preedit-relative UTF-16
    pub style: UnderlineStyle,
    pub target_clause: bool,   // the clause the IME is currently converting
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum UnderlineStyle { Solid, Dotted, Thick }

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
`&str` bytes through length-bearing `BorrowedUtf8`; the `underlines` slice is likewise borrowed
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
    pub(crate) fn set_marked_text(self, text: &str, selected_range: TextRange, underlines: &[UnderlineSegment]) {
        (self.set_marked_text)(SetMarkedTextArgs {
            text: BorrowedUtf8::new(text),
            selected_range,
            underlines: BorrowedArray::from_slice(underlines),
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

Add `"Win32_UI_Input_Ime"` to the `windows` dependency feature list in
`native/desktop-win32/Cargo.toml`. This single feature enables the whole `Windows::Win32::UI::Input::Ime`
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

Add the dispatch arm and handler. `WM_INPUTLANGCHANGE`'s `lParam` is the new `HKL`; the event carries
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

Goal: the application renders the in-progress composition inline (with clause underlines and a
composition caret); the backend reads composition state and calls `setMarkedText` / `insertText` /
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
Tasks 11–12 give the exact apply function, injectable sink, and tests.

### 7.3 `GCS_*` readers and underline conversion (`ime.rs`)

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
    pub(crate) underlines: Vec<UnderlineSegment>,
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
            let underlines = match (
                source.bytes(GCS_COMPATTR),
                source.bytes(GCS_COMPCLAUSE).and_then(|bytes| decode_u32_bytes(&bytes)),
            ) {
                (Ok(attrs), Ok(clauses)) => underlines_from_parts(&attrs, &clauses, length),
                (Err(err), _) | (_, Err(err)) => {
                    log::warn!("reading IME underline data failed: {err:#}");
                    fallback_underlines(length)
                }
            };
            Some(PreeditSnapshot { text, selected, underlines })
        } else {
            None
        };
        Ok(Self { result, preedit, cancelled: gcs & GCS_ANY == 0 })
    }
}
```

`underlines_from_parts` converts `GCS_COMPATTR` (one `ATTR_*` byte per preedit UTF-16 unit) and
the decoded `GCS_COMPCLAUSE` offsets (ascending clause boundaries, `u32`) into one
`UnderlineSegment` per clause. **All emitted ranges are preedit-relative.** A valid clause array
starts at zero, ends at the preedit UTF-16 length, stays in bounds, and is non-descending.
Malformed or missing clause data falls back to one dotted segment spanning the whole preedit:

```rust
fn decode_u32_bytes(bytes: &[u8]) -> anyhow::Result<Vec<u32>> {
    anyhow::ensure!(bytes.len().is_multiple_of(size_of::<u32>()), "unaligned u32 byte count: {}", bytes.len());
    Ok(bytes
        .chunks_exact(size_of::<u32>())
        .map(|part| u32::from_ne_bytes([part[0], part[1], part[2], part[3]]))
        .collect())
}

fn underlines_from_parts(attrs: &[u8], clauses: &[u32], preedit_len: usize) -> Vec<UnderlineSegment> {
    let bounds = clauses.iter().map(|value| usize::try_from(*value)).collect::<Result<Vec<_>, _>>();
    let Ok(bounds) = bounds else {
        return fallback_underlines(preedit_len);
    };
    if bounds.len() < 2
        || bounds.first() != Some(&0)
        || bounds.last() != Some(&preedit_len)
        || bounds.iter().any(|value| *value > preedit_len)
        || bounds.windows(2).any(|pair| pair[0] > pair[1])
    {
        return fallback_underlines(preedit_len);
    }

    bounds
        .windows(2)
        .filter_map(|pair| {
            let (start, end) = (pair[0], pair[1]);
            if start >= end {
                return None;
            }
            let attribute = attrs.get(start).copied().map_or(ATTR_INPUT, u32::from);
            let (style, target_clause) = match attribute {
                ATTR_TARGET_CONVERTED => (UnderlineStyle::Thick, true),
                ATTR_TARGET_NOTCONVERTED => (UnderlineStyle::Dotted, true),
                ATTR_CONVERTED | ATTR_FIXEDCONVERTED => (UnderlineStyle::Solid, false),
                _ => (UnderlineStyle::Dotted, false), // ATTR_INPUT / ATTR_INPUT_ERROR
            };
            Some(UnderlineSegment {
                range: TextRange { location: start, length: end - start },
                style,
                target_clause,
            })
        })
        .collect()
}

fn fallback_underlines(preedit_len: usize) -> Vec<UnderlineSegment> {
    (preedit_len != 0)
        .then_some(UnderlineSegment {
            range: TextRange { location: 0, length: preedit_len },
            style: UnderlineStyle::Dotted,
            target_clause: false,
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
existing `TextInputClient`. The application renders `text` at the caret, underlines clause segments
from `underlines`, places its composition caret at `selectedRange`, treats `insertText` during
composition as the commit, `unmarkText` as "keep what you are rendering", and `discardMarkedText` as
"drop what you are rendering".

---

## 8. FFI & Kotlin wiring

The generated layers regenerate from Rust; only the Rust definitions and the hand-written Kotlin
wrappers are authored.

1. Define the Rust callback table (`TextInputClient`) and its POD companions (`TextRange`,
   `InsertTextArgs`, `SetMarkedTextArgs`, `CaretRectArgs`, `UnderlineSegment`, `UnderlineStyle`), the
   `InputLanguageChangedEvent` variant (`events.rs`), and the downcalls (`ime_api.rs`).
2. Run the header + binding generation (`./gradlew build`; `cbindgen` regenerates the C header,
   JExtract regenerates the Java bindings). `BorrowedUtf8`, `BorrowedArray<T>`, and
   `AutoDropStrPtr` already have cbindgen mappings and Kotlin readers; the generic
   `BorrowedArray<UnderlineSegment>` monomorphizes to a `Native…` layout class like the other
   generic instantiations.
3. Author the Kotlin wrappers: the `TextInputClient` interface + a holder that binds each method to
   the native callback table via a shared `Arena` (mirror `macos/TextInputClient.kt` — the stubs
   carry the client identity, so there is no handle field); the `InputLanguageChanged` variant +
   `fromNative()` case (`Event.kt`); `setTextInputClient` / `clearTextInputClient` / `setImeEnabled`
   / `notifySelectionChanged` / `notifyLayoutChanged` (`Window.kt`).
4. Keep the layers aligned across the Rust → header → Java → Kotlin chain by running the generated
   binding and full-build commands in §10. The repository has no separate `ffi-sync-checker` or
   `win32-doc-sync` task.

---

## 9. Implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: use
> `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to
> implement this plan task-by-task. Track progress by changing each `- [ ]` checkbox to `- [x]`.

**Goal:** Add an IMM32-backed Win32 `TextInputClient` that preserves today's default enabled and
no-client behavior, supports a system-drawn Phase-1 vertical slice, then atomically enables
self-drawn Phase-2 preedit.

**Architecture:** `text_input_client.rs` is the pure FFI callback ABI. `ime.rs` owns the IMM32
transport, per-window IME state, pure decoders, and the composition-apply logic. `Window` owns the
state cell and the HWND-bound lifecycle; `event_loop.rs` remains a thin message dispatcher. Kotlin
owns all document text through one stable, window-owned callback holder and Arena.

**Tech stack:** Rust 2024, `windows` 0.62.2 (`Win32_UI_Input_Ime`), cbindgen, JExtract/Panama,
Kotlin/JVM, JUnit 5, Skiko/Skia, PowerShell/Gradle.

### Global constraints

- Work in the current worktree and current branch; do not create another worktree or branch.
- Keep one combined Phase 1 + Phase 2 plan. Phase 1 must build and remain usable before Phase 2.
- Never commit `WM_IME_SETCONTEXT` suppression without working self-drawn composition in the same
  commit.
- Preserve the automatically associated default HIMC. `ImeState::new().enabled` is `true`; only
  `setImeEnabled(false)` detaches it.
- Client registration is orthogonal to HIMC association. Clearing a client finalizes and removes
  callbacks/caret but does not detach the context.
- Active means `focused && enabled && client.is_some()`. Message ownership uses the weaker
  `enabled && client.is_some()` gate.
- `caretRect` is a client-relative `LogicalRect`; Rust scales both corners directly to client
  physical pixels. Never introduce mixed-DPI screen-logical coordinates.
- Ranges are UTF-16 code units. Document selection is document-relative; marked selection and
  underline ranges are preedit-relative. `usize::MAX` is the only null-range sentinel.
- Queries use out-parameters. Keep `markedRange`, all replacement-range parameters,
  `textForRange`, and point-to-index APIs absent.
- The holder type lives in `win32/TextInputClient.kt`, while each `Window` owns exactly one holder
  and shared Arena until `Window.close()`. Clear native callbacks before closing the Arena.
- Every new Rust `unsafe` block has a minimal scope and an immediately preceding `// SAFETY:`
  justification. Recoverable failures return `Result`, never panic or `unwrap` in production.
- HIMC association and finalization failures propagate through `anyhow::Result` / `with_window`.
  Positioning and hidden-caret failures are logged and never roll back valid lifecycle state.
- A Phase-2 core read failure keeps message ownership and state, logs once for that message, and
  recovers on a later update or END.
- Refresh preedit for `GCS_COMPSTR`, `GCS_COMPATTR`, `GCS_COMPCLAUSE`, `GCS_CURSORPOS`, or
  `GCS_DELTASTART`. Cancellation means none of the complete `GCS_*` bits, not `lParam == 0`.
- Preserve normal, control, dead-character, and system-character paths; only printable `WM_CHAR`
  reaches `insertText` while an active client exists.
- Every task ends with the stated focused test/check and its own commit. Do not combine commits.

### File responsibility map

| File | Responsibility after implementation |
|---|---|
| `native/desktop-win32/src/win32/text_input_client.rs` | Pure FFI callback ABI: `TextRange`, args structs, `UnderlineSegment`/`UnderlineStyle`, callback aliases, the `TextInputClient` table and its safe wrappers. No imports from `ime.rs`/`window.rs`. |
| `native/desktop-win32/src/win32/ime.rs` | `ImmContext` guard as sole `HIMC` owner (transport, positioning, notify), `ImeState` + `ClientCallbackGuard`, `GCS_*` readers/decoders, `CompositionSnapshot`, `CompositionSink` trait, composition-apply functions, unit tests. |
| `native/desktop-win32/src/win32/ime_api.rs` | Five exported window downcalls only. |
| `native/desktop-win32/src/win32/window.rs` | The `ime` state cell and its `Window` helpers, HWND-bound lifecycle (client/enable/focus/teardown, caret), `CompositionSink` impl, caret-rect scaling. |
| `native/desktop-win32/src/win32/event_loop.rs` | Message arms (`WM_IME_*` handlers, `WM_INPUTLANGCHANGE`), existing character fallback, DPI/focus dispatch. |
| `native/desktop-win32/src/win32/events.rs` | `InputLanguageChanged` push event ABI (tag values regenerate in lockstep with all bindings; no append-only ordering). |
| `kotlin-desktop-toolkit/.../win32/TextInputClient.kt` | Public Win32 client model, borrowed-value decoders, stable holder/upcall stubs. |
| `kotlin-desktop-toolkit/.../win32/Window.kt` | Holder ownership, five public methods, ordered idempotent close. |
| `kotlin-desktop-toolkit/.../win32/Event.kt` | Managed language-event variant and decoder. |
| `sample/.../win32/ToyTextInputWin32.kt` | Editable reference client and inline-preedit renderer. |

---

### Task 1: Native IME module and callback ABI

**Files:**
- Modify: `native/desktop-win32/Cargo.toml`
- Modify: `native/desktop-win32/src/win32/mod.rs`
- Create: `native/desktop-win32/src/win32/text_input_client.rs`
- Create: `native/desktop-win32/src/win32/ime.rs`

**Interfaces:**
- Consumes: `desktop_common::ffi_utils::{BorrowedArray, BorrowedUtf8}` and
  `geometry::{LogicalPoint, LogicalRect, LogicalSize}`.
- Produces: `TextRange`, `UnderlineSegment`, `UnderlineStyle`, `InsertTextArgs`,
  `SetMarkedTextArgs`, `CaretRectArgs`, `TextInputClient`, `ImmContext::get(HWND)`, and safe
  callback wrapper methods used by Tasks 2–13.

- [ ] **Step 1: Add the Windows feature and module declarations**

Add `"Win32_UI_Input_Ime",` immediately before `"Win32_UI_Input_KeyboardAndMouse",` in
`Cargo.toml`. Add `pub mod ime;` between `geometry` and `global_data` and
`pub mod text_input_client;` between `system_menu` and `utils` in `mod.rs`. Do not declare
`ime_api` yet; its exports consume methods that do not exist until Task 7.

- [ ] **Step 2: Write the sentinel test first**

Create `text_input_client.rs` with the imports, `TextRange`, and this test. Leave
`TextRange::none` and `into_option` absent for the red run. This module is the pure callback ABI:
it never imports from `ime.rs` or `window.rs` and touches no Win32 API:

```rust
use desktop_common::ffi_utils::{BorrowedArray, BorrowedUtf8};

use super::geometry::{LogicalPoint, LogicalRect, LogicalSize};

const NOT_FOUND: usize = usize::MAX;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    pub location: usize,
    pub length: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_range_round_trips() {
        assert_eq!(TextRange::none().into_option(), None);
        let range = TextRange { location: 4, length: 2 };
        assert_eq!(range.into_option(), Some(range));
    }
}
```

Run:

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::text_input_client::tests::none_range_round_trips
```

Expected: compilation fails because `none` and `into_option` are not defined.

- [ ] **Step 3: Add the complete ABI foundation**

Insert the following above the test module in `text_input_client.rs`. Use aliases for every
function pointer so cbindgen and JExtract generate stable callback helper names. The full struct
and wrapper bodies are in §5.3; `set_marked_text` takes the sentinel-carrying `TextRange`
directly (`none` = the IME shows no composition cursor), and only the query direction maps the
sentinel to `Option`:

```rust
impl TextRange {
    pub(crate) const fn none() -> Self {
        Self { location: NOT_FOUND, length: 0 }
    }

    const fn into_option(self) -> Option<Self> {
        if self.location == NOT_FOUND { None } else { Some(self) }
    }
}

#[repr(C)]
pub struct InsertTextArgs<'a> { ... }

#[repr(C)]
pub struct SetMarkedTextArgs<'a> {
    pub text: BorrowedUtf8<'a>,
    pub selected_range: TextRange,
    pub underlines: BorrowedArray<'a, UnderlineSegment>,
}

#[repr(C)]
pub struct CaretRectArgs { ... }

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnderlineSegment { ... }

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderlineStyle { Solid, Dotted, Thick }

pub type SelectedRangeCallback = extern "C" fn(range_out: &mut TextRange);
pub type CaretRectCallback = extern "C" fn(args: &mut CaretRectArgs);
pub type InsertTextCallback = extern "C" fn(args: InsertTextArgs);
pub type SetMarkedTextCallback = extern "C" fn(args: SetMarkedTextArgs);
pub type UnmarkTextCallback = extern "C" fn();
pub type DiscardMarkedTextCallback = extern "C" fn();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TextInputClient { ... }

impl TextInputClient {
    pub(crate) fn selected_range(self) -> Option<TextRange> { ... }
    pub(crate) fn caret_rect(self, range: TextRange) -> LogicalRect { ... }
    pub(crate) fn insert_text(self, text: &str) { ... }
    /// A `none` `selected_range` means the IME shows no composition cursor.
    pub(crate) fn set_marked_text(self, text: &str, selected_range: TextRange, underlines: &[UnderlineSegment]) { ... }
    pub(crate) fn unmark_text(self) { ... }
    pub(crate) fn discard_marked_text(self) { ... }
}
```

Create `ime.rs` with the RAII context guard from §5.1. The raw `HIMC` stays private to the guard:

```rust
use windows::Win32::{
    Foundation::HWND,
    UI::Input::Ime::{HIMC, ImmGetContext, ImmReleaseContext},
};

pub(crate) struct ImmContext {
    hwnd: HWND,
    himc: HIMC,
}

impl ImmContext {
    pub(crate) fn get(hwnd: HWND) -> Option<Self> {
        // SAFETY: callers pass the live HWND owned by `Window`; the guard releases the acquired
        // context against the same handle.
        let himc = unsafe { ImmGetContext(hwnd) };
        (!himc.is_invalid()).then_some(Self { hwnd, himc })
    }
}

impl Drop for ImmContext {
    fn drop(&mut self) {
        // SAFETY: this guard owns exactly one successful `ImmGetContext` result for `self.hwnd`.
        if !unsafe { ImmReleaseContext(self.hwnd, self.himc) }.as_bool() {
            log::warn!("ImmReleaseContext failed");
        }
    }
}
```

- [ ] **Step 4: Run focused and crate checks**

Run:

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::text_input_client::tests::none_range_round_trips
cargo check --manifest-path native/Cargo.toml -p desktop-win32
```

Expected: the focused test passes and `cargo check` ends with `Finished`.

- [ ] **Step 5: Commit**

```powershell
git add native/desktop-win32/Cargo.toml native/desktop-win32/src/win32/mod.rs native/desktop-win32/src/win32/text_input_client.rs native/desktop-win32/src/win32/ime.rs
git commit -m "feat(win32): add IME callback ABI"
```

---

### Task 2: Per-window IME state and callback guard

**Files:**
- Modify: `native/desktop-win32/src/win32/ime.rs`
- Modify: `native/desktop-win32/src/win32/window.rs`

**Interfaces:**
- Consumes: `text_input_client::TextInputClient` from Task 1.
- Produces: `ImeState` (private fields, accessor + transition methods) and `ClientCallbackGuard`
  in `ime.rs`; `Window::{enabled_client, active_client, with_enabled_client, with_active_client,
  ime_start, ime_end, ime_is_finalizing, ime_revision, ime_set_app_marked,
  clear_composition_state}` for later tasks.

- [ ] **Step 1: Add red state tests**

At the end of `ime.rs`, add a `#[cfg(test)] mod ime_state_tests` containing:

```rust
#[cfg(test)]
mod ime_state_tests {
    use super::*;

    #[test]
    fn ime_starts_enabled_without_a_client_or_focus() {
        let state = ImeState::new();
        assert!(state.enabled);
        assert!(!state.focused);
        assert!(state.client.is_none());
        assert!(!state.is_active());
    }

    #[test]
    fn callback_guard_nests_and_restores_depth() {
        let state = Cell::new(ImeState::new());
        {
            let _outer = ClientCallbackGuard::enter(&state);
            assert_eq!(state.get().callback_depth, 1);
            {
                let _inner = ClientCallbackGuard::enter(&state);
                assert_eq!(state.get().callback_depth, 2);
            }
            assert_eq!(state.get().callback_depth, 1);
        }
        assert_eq!(state.get().callback_depth, 0);
    }
}
```

Run:

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::ime::ime_state_tests::
```

Expected: compilation fails because `ImeState` and `ClientCallbackGuard` do not exist.

- [ ] **Step 2: Add the state and field**

Add the `ImeState` record, accessors, and transition methods from §5.2 plus the guard to `ime.rs`.
In `window.rs`, import `ime::{ClientCallbackGuard, ImeState}`, add `ime: Cell<ImeState>` as the
final mutable-state field before `event_loop`, and initialize it with `Cell::new(ImeState::new())`:

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
```

- [ ] **Step 3: Add the narrow `Window` helpers**

Add these methods inside `impl Window`. Each is a thin get/transition/set wrapper; the `ImeState`
transitions themselves own the revision invariant:

```rust
pub(crate) const fn enabled_client(&self) -> Option<TextInputClient> {
    self.ime.get().enabled_client()
}

pub(crate) const fn active_client(&self) -> Option<TextInputClient> {
    self.ime.get().active_client()
}

pub(crate) fn with_enabled_client<R>(&self, f: impl FnOnce(TextInputClient) -> R) -> Option<R> {
    let client = self.enabled_client()?;
    let _guard = ClientCallbackGuard::enter(&self.ime);
    Some(f(client))
}

pub(crate) fn with_active_client<R>(&self, f: impl FnOnce(TextInputClient) -> R) -> Option<R> {
    let client = self.active_client()?;
    let _guard = ClientCallbackGuard::enter(&self.ime);
    Some(f(client))
}

pub(crate) const fn ime_revision(&self) -> u64 {
    self.ime.get().revision()
}

pub(crate) fn ime_start(&self) {
    let mut ime = self.ime.get();
    ime.start_composition();
    self.ime.set(ime);
}

pub(crate) fn ime_end(&self) -> bool {
    let had_marked_text = self.ime.get().app_has_marked_text();
    self.clear_composition_state();
    had_marked_text
}

pub(crate) const fn ime_is_finalizing(&self) -> bool {
    self.ime.get().is_finalizing()
}

pub(crate) const fn ime_app_has_marked_text(&self) -> bool {
    self.ime.get().app_has_marked_text()
}

fn ime_set_app_marked(&self, value: bool) -> u64 {
    let mut ime = self.ime.get();
    let revision = ime.set_app_marked(value);
    self.ime.set(ime);
    revision
}

pub(crate) fn clear_composition_state(&self) -> u64 {
    let mut ime = self.ime.get();
    let revision = ime.clear_composition_state();
    self.ime.set(ime);
    revision
}
```

- [ ] **Step 4: Run focused and crate checks**

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::ime::ime_state_tests::
cargo check --manifest-path native/Cargo.toml -p desktop-win32
```

Expected: two focused tests pass and `cargo check` ends with `Finished`.

- [ ] **Step 5: Commit**

```powershell
git add native/desktop-win32/src/win32/ime.rs native/desktop-win32/src/win32/window.rs
git commit -m "feat(win32): track per-window IME state"
```

---

### Task 3: Phase-1 character routing and surrogate joining

**Files:**
- Modify: `native/desktop-win32/src/win32/window.rs`
- Modify: `native/desktop-win32/src/win32/event_loop.rs`

**Interfaces:**
- Consumes: `Window::{active_client, with_active_client}`.
- Produces: `ImeState::join_surrogate`,
  `Window::{join_surrogate, clear_pending_surrogate}`, and a refactored `on_char` that owns only
  printable `WM_CHAR` for an active client.

- [ ] **Step 1: Add the red surrogate tests**

Append these tests to `ime_state_tests`:

```rust
#[test]
fn surrogate_joiner_handles_bmp_pair_and_lone_low() {
    let mut ime = ImeState::new();
    assert_eq!(ime.join_surrogate('A' as u16), Some("A".to_owned()));
    assert_eq!(ime.join_surrogate(0xD83D), None);
    assert_eq!(ime.join_surrogate(0xDE00), Some("😀".to_owned()));
    assert_eq!(ime.join_surrogate(0xDE00), None);
}

#[test]
fn surrogate_joiner_drops_an_interrupted_high_unit() {
    let mut ime = ImeState::new();
    assert_eq!(ime.join_surrogate(0xD83D), None);
    ime.reset_pending_surrogate();
    assert_eq!(ime.join_surrogate(0xDE00), None);
}

#[test]
fn pending_surrogate_reset_is_idempotent() {
    let mut ime = ImeState::new();
    ime.pending_high_surrogate = Some(0xD83D);
    ime.reset_pending_surrogate();
    assert_eq!(ime.pending_high_surrogate, None);
    ime.reset_pending_surrogate();
    assert_eq!(ime.pending_high_surrogate, None);
}
```

Run:

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::ime::ime_state_tests::surrogate_joiner
```

Expected: compilation fails because `ImeState::join_surrogate` does not exist.

- [ ] **Step 2: Implement the pure joiner and `Window` wrapper**

Add to `impl ImeState` (the §6.2 joiner):

```rust
pub(crate) fn join_surrogate(&mut self, unit: u16) -> Option<String> {
    let pending = self.pending_high_surrogate.take();
    if (0xD800..=0xDBFF).contains(&unit) {
        self.pending_high_surrogate = Some(unit);
        return None;
    }
    if (0xDC00..=0xDFFF).contains(&unit) {
        return pending.map(|high| String::from_utf16_lossy(&[high, unit]));
    }
    Some(String::from_utf16_lossy(&[unit]))
}
```

Add to `impl Window`:

```rust
pub(crate) fn join_surrogate(&self, unit: u16) -> Option<String> {
    let mut ime = self.ime.get();
    let result = ime.join_surrogate(unit);
    self.ime.set(ime);
    result
}


pub(crate) fn clear_pending_surrogate(&self) {
    let mut ime = self.ime.get();
    ime.reset_pending_surrogate();
    self.ime.set(ime);
}
```

Use `reset_pending_surrogate()` rather than assigning the field in the later client-switch,
disable, focus-loss, and composition-clear paths. Task 5 routes client replacement, focus changes,
enable changes, and composition clearing through separately tested pure `ImeState` transitions,
each of which owns its surrogate reset.

- [ ] **Step 3: Preserve the old path under a new helper, then route text**

Rename the current `on_char` body to `character_received` without changing it. Replace `on_char`
with:

```rust
fn on_char(event_loop: &EventLoop, window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if window.active_client().is_none() {
        return character_received(event_loop, window, msg, wparam, lparam);
    }
    if msg != WM_CHAR {
        window.clear_pending_surrogate();
        return character_received(event_loop, window, msg, wparam, lparam);
    }

    let unit = LOWORD!(wparam.0);
    // Control units (Enter, Tab, Backspace, ...) must still reach the app as CharacterReceived,
    // never as an insertText edit.
    if matches!(unit, 0x00..=0x1F | 0x7F..=0x9F) {
        window.clear_pending_surrogate();
        return character_received(event_loop, window, msg, wparam, lparam);
    }

    if let Some(text) = window.join_surrogate(unit) {
        let _ = window.with_active_client(|client| client.insert_text(&text));
    }
    Some(LRESULT(0))
}

fn character_received(
    event_loop: &EventLoop,
    window: &Window,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<LRESULT> {
    let character = LOWORD!(wparam.0);
    let event = CharacterReceivedEvent {
        character,
        key_status: PhysicalKeyStatus::from(lparam),
        is_dead_char: matches!(msg, WM_DEADCHAR | WM_SYSDEADCHAR),
        is_system_key: matches!(msg, WM_SYSCHAR | WM_SYSDEADCHAR),
    };
    event_loop.handle_event(window, event)
}
```

Do not change the pump or `on_keyevent`: its existing `VK_PROCESSKEY` translation/drop is required.

- [ ] **Step 4: Run focused and crate checks**

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::ime::ime_state_tests::surrogate_joiner
cargo check --manifest-path native/Cargo.toml -p desktop-win32
```

Expected: two surrogate tests pass and `cargo check` ends with `Finished`.

- [ ] **Step 5: Commit**

```powershell
git add native/desktop-win32/src/win32/ime.rs native/desktop-win32/src/win32/window.rs native/desktop-win32/src/win32/event_loop.rs
git commit -m "feat(win32): route text through IME client"
```

---

### Task 4: Candidate positioning and compatibility caret

**Files:**
- Modify: `native/desktop-win32/src/win32/ime.rs`
- Modify: `native/desktop-win32/src/win32/window.rs`
- Modify: `native/desktop-win32/src/win32/event_loop.rs`

**Interfaces:**
- Consumes: `Window::{active_client, with_active_client, ime_revision, hwnd, get_scale}` and
  `ImmContext::get`.
- Produces: `ImmContext::{set_composition_window, set_candidate_window}`,
  `client_logical_to_physical_rect(LogicalRect, f32) -> RECT`, and
  `Window::{update_ime_windows, create_caret, destroy_caret}`.

- [ ] **Step 1: Write the failing corner-scaling test**

Add a `#[cfg(test)] mod tests` at the end of `window.rs`:

```rust
#[test]
fn logical_caret_rect_scales_both_corners() {
    let rect = LogicalRect {
        origin: LogicalPoint::new(10.25, 5.25),
        size: LogicalSize::new(3.5, 4.5),
    };
    let physical = client_logical_to_physical_rect(rect, 1.5);
    assert_eq!((physical.left, physical.top, physical.right, physical.bottom), (15, 8, 21, 15));
}
```

Run `cargo test --manifest-path native/Cargo.toml -p desktop-win32
win32::window::tests::logical_caret_rect_scales_both_corners`.

Expected: compilation fails because the converter is missing.

- [ ] **Step 2: Implement positioning**

Add the `CANDIDATEFORM`/`COMPOSITIONFORM` imports to `ime.rs` and
`WindowsAndMessaging::{CreateCaret, DestroyCaret, SetCaretPos}` to `window.rs`, without replacing
unrelated imports. Add the `ImmContext::{set_composition_window, set_candidate_window}` inherent
methods from §5.1 to `ime.rs` — the raw `HIMC` never crosses the module boundary.

Add next to its sole caller in `window.rs`:

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

Add the §6.3 `update_ime_windows` to `impl Window`: one `with_active_client` cycle pulls
`selected_range` and `caret_rect` together, the revision check after the cycle catches reentrant
composition/focus transitions, and positioning goes through the `ImmContext` methods:

```rust
pub(crate) fn update_ime_windows(&self) {
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
    context.set_composition_window(origin);
    context.set_candidate_window(origin, caret);

    // SAFETY: active-client gating requires focus; lifecycle code attempted to create this caret.
    if let Err(err) = unsafe { SetCaretPos(origin.x, origin.y) } {
        log::warn!("SetCaretPos failed: {err}");
    }
}

fn create_caret(&self) -> windows_core::Result<()> {
    // SAFETY: callers use this only for a live, focused HWND with an active client.
    unsafe { CreateCaret(self.hwnd(), None, 1, 1) }
}

fn destroy_caret(&self) -> windows_core::Result<()> {
    // SAFETY: callers use this only while this window owns the GUI thread's caret.
    unsafe { DestroyCaret() }
}
```

Keep all caret failures best effort at lifecycle call sites; they must never roll back valid
client/HIMC state.

- [ ] **Step 3: Refresh after DPI changes**

Add `window.update_ime_windows();` after the existing scale event and custom-titlebar update in
`on_dpichanged`, immediately before returning its current `result`.

- [ ] **Step 4: Verify and commit**

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::window::tests::logical_caret_rect_scales_both_corners
cargo check --manifest-path native/Cargo.toml -p desktop-win32
git add native/desktop-win32/src/win32/ime.rs native/desktop-win32/src/win32/window.rs native/desktop-win32/src/win32/event_loop.rs
git commit -m "feat(win32): position IME UI at caret"
```

Expected: focused test passes; `cargo check` ends with `Finished`.

---

### Task 5: HIMC, focus, client, and teardown lifecycle

**Files:**
- Modify: `native/desktop-win32/src/win32/ime.rs`
- Modify: `native/desktop-win32/src/win32/window.rs`
- Modify: `native/desktop-win32/src/win32/window_api.rs`
- Modify: `native/desktop-win32/src/win32/event_loop.rs`

**Interfaces:**
- Consumes: Tasks 2–4 state, callbacks, positioning, and caret helpers.
- Produces: `ImeState::{replace_client, set_enabled, set_focused, start_composition,
  begin_finalizing, clear_composition_state}`; `ImmContext::notify_composition`;
  `Window::{set_text_input_client, set_ime_enabled, ime_focus_gained, ime_focus_lost,
  finalize_composition, ime_teardown}`; guarded `Window::destroy`; Phase-1 START/END forwarding.

- [ ] **Step 1: Add failing lifecycle-state tests**

Append to `ime_state_tests`:

```rust
#[test]
fn callback_depth_rejects_lifetime_mutation() {
    let mut ime = ImeState::new();
    assert!(ime.ensure_mutation_allowed("client change").is_ok());
    ime.callback_depth = 1;
    assert_eq!(
        ime.ensure_mutation_allowed("client change").unwrap_err().to_string(),
        "client change is not allowed during a text input callback",
    );
}

#[test]
fn client_replacement_clears_pending_surrogate() {
    let mut ime = ImeState::new();
    ime.pending_high_surrogate = Some(0xD83D);
    ime.replace_client(None);
    assert_eq!(ime.pending_high_surrogate, None);
}

#[test]
fn focus_loss_clears_pending_surrogate() {
    let mut ime = ImeState::new();
    ime.focused = true;
    ime.pending_high_surrogate = Some(0xD83D);
    let revision = ime.composition_revision;
    ime.set_focused(false);
    assert!(!ime.focused);
    assert_eq!(ime.pending_high_surrogate, None);
    assert_eq!(ime.composition_revision, revision + 1);
}

#[test]
fn composition_start_marks_active_and_advances_revision() {
    let mut ime = ImeState::new();
    ime.app_has_marked_text = true;
    ime.pending_high_surrogate = Some(0xD83D);
    let revision = ime.composition_revision;
    ime.start_composition();
    assert!(ime.composition_active);
    assert!(!ime.app_has_marked_text);
    assert_eq!(ime.pending_high_surrogate, None);
    assert_eq!(ime.composition_revision, revision + 1);
}

#[test]
fn composition_end_clears_pending_surrogate() {
    let mut ime = ImeState::new();
    ime.composition_active = true;
    ime.app_has_marked_text = true;
    ime.finalizing = true;
    ime.pending_high_surrogate = Some(0xD83D);
    let revision = ime.composition_revision;
    assert_eq!(ime.clear_composition_state(), revision + 1);
    assert!(!ime.composition_active);
    assert!(!ime.app_has_marked_text);
    assert!(!ime.finalizing);
    assert_eq!(ime.pending_high_surrogate, None);
}
```

Run:

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::ime::ime_state_tests::
```

Expected: compilation fails because `ensure_mutation_allowed`, `replace_client`, `set_focused`, and
the `ImeState` form of `clear_composition_state` are missing.

- [ ] **Step 2: Implement the pure lifecycle transitions and mutation gate**

Add the §5.2 transition methods (`replace_client`, `set_enabled`, `set_focused`,
`start_composition`, `begin_finalizing`, `clear_composition_state`) to `impl ImeState`, plus the
mutation gate:

```rust
pub(crate) fn ensure_mutation_allowed(self, operation: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        self.callback_depth == 0,
        "{operation} is not allowed during a text input callback"
    );
    Ok(())
}
```

Then replace Task 2's `Window::clear_composition_state` body so every composition-end path uses the
tested pure transition:

```rust
pub(crate) fn clear_composition_state(&self) -> u64 {
    let mut ime = self.ime.get();
    let revision = ime.clear_composition_state();
    self.ime.set(ime);
    revision
}
```

- [ ] **Step 3: Implement the complete lifecycle**

Import the IMM symbols used by the lifecycle:

```rust
use anyhow::Context as _;
use windows::Win32::UI::Input::Ime::{
    CPS_CANCEL, CPS_COMPLETE, HIMC, IACE_DEFAULT, ImmAssociateContextEx, ImmNotifyIME,
    NI_COMPOSITIONSTR,
};
use windows::Win32::UI::WindowsAndMessaging::GetFocus;
```

Add these methods to `impl Window`:

```rust
pub(crate) fn set_text_input_client(&self, client: Option<TextInputClient>) -> anyhow::Result<()> {
    let current = self.ime.get();
    current.ensure_mutation_allowed("text input client change")?;
    if current.is_composition_active() {
        self.finalize_composition()?;
    }

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
        anyhow::ensure!(
            // SAFETY: `hwnd` is live; null HIMC plus `IACE_DEFAULT` restores the thread default.
            unsafe { ImmAssociateContextEx(hwnd, HIMC::default(), IACE_DEFAULT) }.as_bool(),
            "ImmAssociateContextEx(IACE_DEFAULT) failed"
        );
        let mut ime = self.ime.get();
        ime.set_enabled(true);
        self.ime.set(ime);
        if ime.is_active() {
            if let Err(err) = self.create_caret() {
                log::warn!("CreateCaret failed after enabling IME: {err}");
            }
            self.update_ime_windows();
        }
    } else {
        self.finalize_composition()?;
        anyhow::ensure!(
            // Null HIMC + flags 0 is the de-facto detach idiom (winit, GLFW); Learn documents
            // only the IACE_* flags.
            // SAFETY: `hwnd` is live.
            unsafe { ImmAssociateContextEx(hwnd, HIMC::default(), 0) }.as_bool(),
            "ImmAssociateContextEx(detach) failed"
        );
        let mut ime = self.ime.get();
        let destroy_caret = ime.is_active();
        ime.set_enabled(false);
        self.ime.set(ime);
        if destroy_caret && let Err(err) = self.destroy_caret() {
            log::warn!("DestroyCaret failed after disabling IME: {err}");
        }
    }
    Ok(())
}

fn finalize_composition(&self) -> anyhow::Result<()> {
    let current = self.ime.get();
    if current.is_finalizing() || !current.is_composition_active() {
        return Ok(());
    }
    let context = ImmContext::get(self.hwnd()).context("window has no input context")?;
    let mut ime = current;
    ime.begin_finalizing();
    self.ime.set(ime);
    // CPS_CANCEL: the app renders the preedit — accept it app-side, then discard the IME's copy.
    // CPS_COMPLETE synchronously reenters WM_IME_COMPOSITION with GCS_RESULTSTR; the finalizing
    // arm of on_ime_composition routes it to insert_text.
    let (action, action_name) = if current.app_has_marked_text() {
        let _ = self.with_enabled_client(TextInputClient::unmark_text);
        (CPS_CANCEL, "CPS_CANCEL")
    } else {
        (CPS_COMPLETE, "CPS_COMPLETE")
    };
    let notified = context.notify_composition(action);
    self.clear_composition_state();
    anyhow::ensure!(notified, "ImmNotifyIME({action_name}) failed");
    Ok(())
}

pub(crate) fn ime_focus_gained(&self) {
    let mut ime = self.ime.get();
    ime.set_focused(true);
    self.ime.set(ime);
    if ime.is_active() {
        if let Err(err) = self.create_caret() {
            log::warn!("CreateCaret failed after focus gain: {err}");
        }
        self.update_ime_windows();
    }
}

pub(crate) fn ime_focus_lost(&self) -> anyhow::Result<()> {
    // Finalizing can synchronously reenter this window (`CPS_COMPLETE` delivers its result
    // through `WM_IME_COMPOSITION`) and can even regain focus, so query the final OS focus
    // owner before committing local state.
    let finalization = self.finalize_composition();
    // SAFETY: `GetFocus` has no pointer/lifetime preconditions and returns this GUI thread's owner.
    let focused = unsafe { GetFocus() } == self.hwnd();
    let mut ime = self.ime.get();
    let destroy_caret = ime.is_active() && !focused;
    ime.set_focused(focused);
    self.ime.set(ime);
    if destroy_caret && let Err(err) = self.destroy_caret() {
        log::warn!("DestroyCaret failed after focus loss: {err}");
    }
    finalization
}

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
        // SAFETY: teardown runs before `WM_NCDESTROY` releases the live HWND.
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

Do not call `ImmAssociateContextEx` from `Window::new`; `ImeState::new().enabled == true` describes
the context Windows already associated. Only explicit disable and terminal teardown detach it.

- [ ] **Step 4: Guard `DestroyWindow` and teardown before weak reclamation**

Replace `Window::destroy` and the `WM_NCDESTROY` branch with:

```rust
pub fn destroy(&self) -> anyhow::Result<()> {
    self.ime.get().ensure_mutation_allowed("window destruction")?;
    // SAFETY: this is the live HWND owned by `Window`; `WM_NCDESTROY` performs terminal teardown.
    unsafe { DestroyWindow(self.hwnd()) }?;
    Ok(())
}
```

```rust
if msg == WM_NCDESTROY {
    // SAFETY: this is the property installed by `on_nccreate`; it remains installed through IME
    // teardown so any synchronous finalization message can recover the same `Window`.
    let raw = unsafe { GetPropW(hwnd, WINDOW_PTR_PROP_NAME).0.cast::<Window>() };
    if !raw.is_null() {
        // SAFETY: `raw` is the leaked `Weak<Window>`; `ManuallyDrop` only borrows it here because
        // `RemovePropW` below performs the unique reclamation.
        let weak = ManuallyDrop::new(unsafe { Weak::from_raw(raw) });
        if let Some(window) = weak.upgrade() {
            window.ime_teardown();
        }
    }
    // SAFETY: teardown is complete and this terminal message removes the property exactly once.
    if let Ok(raw) = unsafe { RemovePropW(hwnd, WINDOW_PTR_PROP_NAME) } {
        // SAFETY: property removal transfers the single `Weak<Window>` leaked by `on_nccreate`
        // back for exactly one reconstruction and drop.
        drop(unsafe { Weak::from_raw(raw.0.cast::<Window>()) });
    }
    return LRESULT(0);
}
```

The existing `window_destroy` wrapper remains structurally unchanged; `window.destroy()?` now
propagates the callback-depth error.

- [ ] **Step 5: Wire focus and Phase-1 forwarding**

Add to `event_loop.rs`:

```rust
fn on_ime_startcomposition_phase1(window: &Window) -> Option<LRESULT> {
    if window.active_client().is_some() {
        window.ime_start();
        window.update_ime_windows();
    }
    None
}

fn on_ime_endcomposition_phase1(window: &Window) -> Option<LRESULT> {
    window.clear_composition_state();
    None
}
```

In `event_loop.rs`, merge `WM_IME_STARTCOMPOSITION` and `WM_IME_ENDCOMPOSITION` into the existing
`WindowsAndMessaging` import, and merge
`ime::{on_ime_endcomposition_phase1, on_ime_startcomposition_phase1}` into the existing
`use super::{...}` group. Then replace the current focus arms and add the Phase-1 composition arms:

```rust
WM_SETFOCUS => on_setfocus(self, window),
WM_KILLFOCUS => on_killfocus(self, window),
WM_IME_STARTCOMPOSITION => on_ime_startcomposition_phase1(window),
WM_IME_ENDCOMPOSITION => on_ime_endcomposition_phase1(window),
```

Add both helpers; they log lifecycle failure, then deliver the existing keyboard event:

```rust
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

- [ ] **Step 6: Verify and commit**

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::ime::ime_state_tests::
cargo check --manifest-path native/Cargo.toml -p desktop-win32
git add native/desktop-win32/src/win32/ime.rs native/desktop-win32/src/win32/window.rs native/desktop-win32/src/win32/window_api.rs native/desktop-win32/src/win32/event_loop.rs
git commit -m "feat(win32): manage IME lifecycle"
```

Expected: state tests pass; `cargo check` ends with `Finished`.

---

### Task 6: Input-language push event

**Files:**
- Modify: `native/desktop-win32/src/win32/events.rs`
- Modify: `native/desktop-win32/src/win32/event_loop.rs`

**Interfaces:**
- Consumes: `AutoDropStrPtr`, `RustAllocatedStrPtr`, and `EventLoop::handle_event`.
- Produces: the `Event::InputLanguageChanged` variant, locale resolution, and a
  `WM_INPUTLANGCHANGE` handler that always falls through.

- [ ] **Step 1: Add the ABI event**

Add at its alphabetical position — `Event` tag values are not ABI-stable (all bindings regenerate
in lockstep; no external header consumers), so no append-only ordering is required:

```rust
InputLanguageChanged(InputLanguageChangedEvent),
```

Add:

```rust
#[repr(C)]
#[derive(Debug)]
pub struct InputLanguageChangedEvent {
    pub hkl: usize,
    pub locale_name: AutoDropStrPtr,
}

impl From<InputLanguageChangedEvent> for Event {
    fn from(value: InputLanguageChangedEvent) -> Self {
        Self::InputLanguageChanged(value)
    }
}
```

- [ ] **Step 2: Add dispatch and resolver code**

Merge `desktop_common::ffi_utils::RustAllocatedStrPtr`,
`Globalization::LCIDToLocaleName`, and `System::SystemServices::LOCALE_NAME_MAX_LENGTH` into the
`event_loop.rs` imports, then add the §6.5 handler and resolver to `event_loop.rs`:

```rust
fn on_inputlangchange(event_loop: &EventLoop, window: &Window, lparam: LPARAM) -> Option<LRESULT> {
    let hkl = lparam.0.cast_unsigned();
    let langid = u32::from(LOWORD!(hkl));
    let locale_name = RustAllocatedStrPtr::allocate(resolve_locale_name(langid))
        .inspect_err(|err| log::error!("Failed to allocate the locale name: {err:?}"))
        .unwrap_or_else(|_| RustAllocatedStrPtr::null())
        .to_auto_drop();
    let _ = event_loop.handle_event(window, InputLanguageChangedEvent { hkl, locale_name });
    None
}

fn resolve_locale_name(langid: u32) -> String {
    let mut buffer = [0u16; LOCALE_NAME_MAX_LENGTH as usize];
    // SAFETY: the buffer is writable for `LOCALE_NAME_MAX_LENGTH` UTF-16 units; a LANGID is a
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

Merge `WM_INPUTLANGCHANGE` into the existing `WindowsAndMessaging` import, then add
`WM_INPUTLANGCHANGE => on_inputlangchange(self, window, lparam),` to the dispatch table.

- [ ] **Step 3: Verify and commit**

```powershell
cargo check --manifest-path native/Cargo.toml -p desktop-win32
git add native/desktop-win32/src/win32/events.rs native/desktop-win32/src/win32/event_loop.rs
git commit -m "feat(win32): report input language changes"
```

Expected: `cargo check` ends with `Finished`; the new event is the last enum tag.

---

### Task 7: Export IME window downcalls

**Files:**
- Create: `native/desktop-win32/src/win32/ime_api.rs`
- Modify: `native/desktop-win32/src/win32/mod.rs`

**Interfaces:**
- Consumes: `window_api::{with_window, WindowPtr}`, `text_input_client::TextInputClient`, and
  Task-5 methods.
- Produces: all five C exports consumed by Task 8.

- [ ] **Step 1: Create the complete API module**

Create the complete file:

```rust
use super::{
    text_input_client::TextInputClient,
    window_api::{WindowPtr, with_window},
};

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
    with_window(&window_ptr, "window_set_ime_enabled", |window| window.set_ime_enabled(enabled));
}

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

- [ ] **Step 2: Declare, verify, and commit**

Add `pub mod ime_api;` immediately after `pub mod ime;`, then run:

```powershell
cargo check --manifest-path native/Cargo.toml -p desktop-win32
git add native/desktop-win32/src/win32/ime_api.rs native/desktop-win32/src/win32/mod.rs
git commit -m "feat(win32): export IME window API"
```

Expected: `cargo check` ends with `Finished`; no export references a missing `Window` method.

---

### Task 8: Generated bindings and managed API

**Files:**
- Create: `kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/TextInputClient.kt`
- Modify: `kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/Converters.kt`
- Modify: `kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/Window.kt`
- Modify: `kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/Event.kt`
- Create: `kotlin-desktop-toolkit/src/test/kotlin/org/jetbrains/desktop/win32/tests/TextInputClientTests.kt`

**Interfaces:**
- Consumes: Task-7 exports and generated `NativeTextInputClient`, callback, argument, range,
  underline, and event layouts.
- Produces: the public Kotlin API in §5.3; a single stable `TextInputClientHolder` per `Window`;
  ordered replacement/clear/close; managed language-event decoding.

- [ ] **Step 1: Generate the Java layouts before writing wrappers**

```powershell
.\gradlew.bat :kotlin-desktop-toolkit:generateBindingsForWin32
```

Expected: task succeeds and the generated package contains `NativeTextInputClient`,
`NativeBorrowedUtf8`, `NativeBorrowedArray_UnderlineSegment`, the six named callback helpers, and
`NativeInputLanguageChangedEvent`. A missing/differently named class is a failed ABI-generation
gate; fix the Rust alias/type declaration before continuing.

- [ ] **Step 2: Write managed-boundary tests first**

Create `TextInputClientTests.kt` with these tests. The holder callbacks are `internal` specifically
so this same-module test can exercise the boundary without a live IME:

```kotlin
package org.jetbrains.desktop.win32.tests

import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.LogicalPoint
import org.jetbrains.desktop.win32.LogicalRect
import org.jetbrains.desktop.win32.LogicalSize
import org.jetbrains.desktop.win32.TextInputClient
import org.jetbrains.desktop.win32.TextInputClientHolder
import org.jetbrains.desktop.win32.TextRange
import org.jetbrains.desktop.win32.UnderlineSegment
import org.jetbrains.desktop.win32.UnderlineStyle
import org.jetbrains.desktop.win32.fromNative
import org.jetbrains.desktop.win32.generated.NativeBorrowedArray_UnderlineSegment
import org.jetbrains.desktop.win32.generated.NativeBorrowedUtf8
import org.jetbrains.desktop.win32.generated.NativeCaretRectArgs
import org.jetbrains.desktop.win32.generated.NativeEvent
import org.jetbrains.desktop.win32.generated.NativeInputLanguageChangedEvent
import org.jetbrains.desktop.win32.generated.NativeLogicalPoint
import org.jetbrains.desktop.win32.generated.NativeLogicalRect
import org.jetbrains.desktop.win32.generated.NativeLogicalSize
import org.jetbrains.desktop.win32.generated.NativeTextRange
import org.jetbrains.desktop.win32.generated.NativeUnderlineSegment
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import org.jetbrains.desktop.win32.readBorrowedUtf8
import org.jetbrains.desktop.win32.readUnderlines
import org.junit.jupiter.api.Test
import java.lang.foreign.Arena
import java.lang.foreign.ValueLayout
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertSame
import kotlin.test.assertTrue

class TextInputClientTests {
    private class RecordingClient : TextInputClient {
        var inserted = ""
        override fun selectedRange(): TextRange = TextRange(3, 0)
        override fun caretRect(range: TextRange): LogicalRect =
            LogicalRect(LogicalPoint(range.location.toFloat(), 7f), LogicalSize(1f, 18f))
        override fun insertText(text: String) { inserted += text }
        override fun setMarkedText(
            text: String,
            selectedRange: TextRange?,
            underlines: List<org.jetbrains.desktop.win32.UnderlineSegment>,
        ) = Unit
        override fun unmarkText() = Unit
        override fun discardMarkedText() = Unit
    }

    @Test
    fun `range sentinel uses location only`() = Arena.ofConfined().use { arena ->
        val native = NativeTextRange.allocate(arena)
        TextRange.notFound.toNative(native)
        assertEquals(null, TextRange.fromNative(native).nullIfNotFound())
        TextRange(5, 2).toNative(native)
        assertEquals(TextRange(5, 2), TextRange.fromNative(native).nullIfNotFound())
    }

    @Test
    fun `selected range callback writes caller storage`() = Arena.ofConfined().use { arena ->
        val holder = TextInputClientHolder()
        holder.textInputClient = RecordingClient()
        val native = NativeTextRange.allocate(arena)
        holder.selectedRangeCallback(native)
        assertEquals(TextRange(3, 0), TextRange.fromNative(native))
        holder.close()
    }

    @Test
    fun `caret callback reads range and writes inline rect`() = Arena.ofConfined().use { arena ->
        val holder = TextInputClientHolder()
        holder.textInputClient = RecordingClient()
        val args = NativeCaretRectArgs.allocate(arena)
        TextRange(9, 0).toNative(NativeCaretRectArgs.range_in(args))
        holder.caretRectCallback(args)
        val rect = NativeCaretRectArgs.rect_out(args)
        assertEquals(9f, NativeLogicalPoint.x(NativeLogicalRect.origin(rect)))
        assertEquals(7f, NativeLogicalPoint.y(NativeLogicalRect.origin(rect)))
        assertEquals(1f, NativeLogicalSize.width(NativeLogicalRect.size(rect)))
        assertEquals(18f, NativeLogicalSize.height(NativeLogicalRect.size(rect)))
        holder.close()
    }

    @Test
    fun `borrowed utf8 decoder preserves embedded nul`() = Arena.ofConfined().use { arena ->
        val bytes = byteArrayOf('A'.code.toByte(), 0, 'B'.code.toByte())
        val native = NativeBorrowedUtf8.allocate(arena)
        NativeBorrowedUtf8.ptr(native, arena.allocateFrom(ValueLayout.JAVA_BYTE, *bytes))
        NativeBorrowedUtf8.len(native, bytes.size.toLong())
        assertEquals("A\u0000B", readBorrowedUtf8(native))
    }

    @Test
    fun `underline decoder reads range style and target`() = Arena.ofConfined().use { arena ->
        val items = NativeUnderlineSegment.allocateArray(1L, arena)
        val item = NativeUnderlineSegment.asSlice(items, 0L)
        NativeTextRange.location(NativeUnderlineSegment.range(item), 2)
        NativeTextRange.length(NativeUnderlineSegment.range(item), 3)
        NativeUnderlineSegment.style(item, desktop_win32_h.NativeUnderlineStyle_Thick())
        NativeUnderlineSegment.target_clause(item, true)
        val borrowed = NativeBorrowedArray_UnderlineSegment.allocate(arena)
        NativeBorrowedArray_UnderlineSegment.ptr(borrowed, items)
        NativeBorrowedArray_UnderlineSegment.len(borrowed, 1)
        assertEquals(
            listOf(UnderlineSegment(TextRange(2, 3), UnderlineStyle.Thick, true)),
            readUnderlines(borrowed),
        )
    }

    @Test
    fun `holder starts with the noop recipient`() {
        val holder = TextInputClientHolder()
        assertSame(TextInputClient.Noop, holder.textInputClient)
        holder.close()
    }

    @Test
    fun `holder owns one table until close`() {
        val holder = TextInputClientHolder()
        val table = holder.native
        assertTrue(table.scope().isAlive)
        holder.close()
        assertFalse(table.scope().isAlive)
    }

    @Test
    fun `input language payload decodes hkl and locale`() = Arena.ofConfined().use { arena ->
        val eventStorage = NativeEvent.allocate(arena)
        NativeEvent.tag(eventStorage, desktop_win32_h.NativeEvent_InputLanguageChanged())
        val native = NativeEvent.input_language_changed(eventStorage)
        NativeInputLanguageChangedEvent.hkl(native, 0x0411L)
        NativeInputLanguageChangedEvent.locale_name(native, arena.allocateFrom("ja-JP"))
        val event = assertIs<Event.InputLanguageChanged>(Event.fromNative(eventStorage))
        assertEquals(0x0411L, event.hkl)
        assertEquals("ja-JP", event.localeName)
    }
}
```

Run:

```powershell
.\gradlew.bat :kotlin-desktop-toolkit:test --tests "org.jetbrains.desktop.win32.tests.TextInputClientTests"
```

Expected: Kotlin compilation fails because the Win32 client/holder do not exist.

- [ ] **Step 3: Add the inline logical-rectangle writer**

Append to `Converters.kt`:

```kotlin
internal fun LogicalRect.toNative(result: MemorySegment) {
    NativeLogicalPoint.x(NativeLogicalRect.origin(result), origin.x)
    NativeLogicalPoint.y(NativeLogicalRect.origin(result), origin.y)
    NativeLogicalSize.width(NativeLogicalRect.size(result), size.width)
    NativeLogicalSize.height(NativeLogicalRect.size(result), size.height)
}
```

This overload writes caller-provided storage; it must not allocate a temporary Arena.

- [ ] **Step 4: Create the public types and stable holder**

Create `TextInputClient.kt`. The imports are the generated layouts named in Step 1 plus
`Arena`, `MemorySegment`, and `ValueLayout`. Use this complete implementation:

```kotlin
package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeBorrowedArray_UnderlineSegment
import org.jetbrains.desktop.win32.generated.NativeBorrowedUtf8
import org.jetbrains.desktop.win32.generated.NativeCaretRectArgs
import org.jetbrains.desktop.win32.generated.NativeCaretRectCallback
import org.jetbrains.desktop.win32.generated.NativeDiscardMarkedTextCallback
import org.jetbrains.desktop.win32.generated.NativeInsertTextArgs
import org.jetbrains.desktop.win32.generated.NativeInsertTextCallback
import org.jetbrains.desktop.win32.generated.NativeSelectedRangeCallback
import org.jetbrains.desktop.win32.generated.NativeSetMarkedTextArgs
import org.jetbrains.desktop.win32.generated.NativeSetMarkedTextCallback
import org.jetbrains.desktop.win32.generated.NativeTextInputClient
import org.jetbrains.desktop.win32.generated.NativeTextRange
import org.jetbrains.desktop.win32.generated.NativeUnderlineSegment
import org.jetbrains.desktop.win32.generated.NativeUnmarkTextCallback
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.lang.foreign.ValueLayout

public interface TextInputClient {
    public fun selectedRange(): TextRange?
    public fun caretRect(range: TextRange): LogicalRect
    public fun insertText(text: String)
    public fun setMarkedText(text: String, selectedRange: TextRange?, underlines: List<UnderlineSegment>)
    public fun unmarkText()
    public fun discardMarkedText()

    public object Noop : TextInputClient {
        override fun selectedRange(): TextRange? = null
        override fun caretRect(range: TextRange): LogicalRect =
            LogicalRect(LogicalPoint.Zero, LogicalSize(0f, 0f))
        override fun insertText(text: String): Unit = Unit
        override fun setMarkedText(
            text: String,
            selectedRange: TextRange?,
            underlines: List<UnderlineSegment>,
        ): Unit = Unit
        override fun unmarkText(): Unit = Unit
        override fun discardMarkedText(): Unit = Unit
    }
}

public data class TextRange(
    public val location: Long,
    public val length: Long,
) {
    internal companion object {
        val notFound: TextRange = TextRange(-1L, 0L)
        fun fromNative(native: MemorySegment): TextRange =
            TextRange(NativeTextRange.location(native), NativeTextRange.length(native))
    }

    internal fun nullIfNotFound(): TextRange? = if (location == -1L) null else this

    internal fun toNative(result: MemorySegment) {
        NativeTextRange.location(result, location)
        NativeTextRange.length(result, length)
    }
}

public data class UnderlineSegment(
    public val range: TextRange,
    public val style: UnderlineStyle,
    public val targetClause: Boolean,
)

public enum class UnderlineStyle {
    Solid,
    Dotted,
    Thick;

    internal companion object {
        fun fromNative(value: Int): UnderlineStyle = when (value) {
            desktop_win32_h.NativeUnderlineStyle_Solid() -> Solid
            desktop_win32_h.NativeUnderlineStyle_Dotted() -> Dotted
            desktop_win32_h.NativeUnderlineStyle_Thick() -> Thick
            else -> error("Unexpected UnderlineStyle value: $value")
        }
    }
}

internal fun readBorrowedUtf8(native: MemorySegment): String {
    val pointer = NativeBorrowedUtf8.ptr(native)
    val length = NativeBorrowedUtf8.len(native)
    if (pointer == MemorySegment.NULL || length == 0L) return ""
    return pointer.asSlice(0, length).toArray(ValueLayout.JAVA_BYTE).decodeToString()
}

internal fun readUnderlines(native: MemorySegment): List<UnderlineSegment> {
    val pointer = NativeBorrowedArray_UnderlineSegment.ptr(native)
    val length = NativeBorrowedArray_UnderlineSegment.len(native)
    if (pointer == MemorySegment.NULL || length == 0L) return emptyList()
    return List(Math.toIntExact(length)) { index ->
        val item = NativeUnderlineSegment.asSlice(pointer, index.toLong())
        UnderlineSegment(
            range = TextRange.fromNative(NativeUnderlineSegment.range(item)),
            style = UnderlineStyle.fromNative(NativeUnderlineSegment.style(item)),
            targetClause = NativeUnderlineSegment.target_clause(item),
        )
    }
}

internal class TextInputClientHolder : AutoCloseable {
    private val arena: Arena = Arena.ofShared()
    internal var textInputClient: TextInputClient = TextInputClient.Noop

    internal val native: MemorySegment = NativeTextInputClient.allocate(arena).also { table ->
        NativeTextInputClient.selected_range(table, NativeSelectedRangeCallback.allocate(this::selectedRangeCallback, arena))
        NativeTextInputClient.caret_rect(table, NativeCaretRectCallback.allocate(this::caretRectCallback, arena))
        NativeTextInputClient.insert_text(table, NativeInsertTextCallback.allocate(this::insertTextCallback, arena))
        NativeTextInputClient.set_marked_text(table, NativeSetMarkedTextCallback.allocate(this::setMarkedTextCallback, arena))
        NativeTextInputClient.unmark_text(table, NativeUnmarkTextCallback.allocate(this::unmarkTextCallback, arena))
        NativeTextInputClient.discard_marked_text(
            table,
            NativeDiscardMarkedTextCallback.allocate(this::discardMarkedTextCallback, arena),
        )
    }

    internal fun selectedRangeCallback(rangeOut: MemorySegment) = ffiUpCall {
        (textInputClient.selectedRange() ?: TextRange.notFound).toNative(rangeOut)
    }

    internal fun caretRectCallback(args: MemorySegment) = ffiUpCall {
        val range = TextRange.fromNative(NativeCaretRectArgs.range_in(args))
        textInputClient.caretRect(range).toNative(NativeCaretRectArgs.rect_out(args))
    }

    internal fun insertTextCallback(args: MemorySegment) = ffiUpCall {
        textInputClient.insertText(readBorrowedUtf8(NativeInsertTextArgs.text(args)))
    }

    internal fun setMarkedTextCallback(args: MemorySegment) = ffiUpCall {
        textInputClient.setMarkedText(
            text = readBorrowedUtf8(NativeSetMarkedTextArgs.text(args)),
            selectedRange = TextRange.fromNative(NativeSetMarkedTextArgs.selected_range(args)).nullIfNotFound(),
            underlines = readUnderlines(NativeSetMarkedTextArgs.underlines(args)),
        )
    }

    internal fun unmarkTextCallback() = ffiUpCall { textInputClient.unmarkText() }
    internal fun discardMarkedTextCallback() = ffiUpCall { textInputClient.discardMarkedText() }

    override fun close() {
        arena.close()
    }
}
```

The holder is a dumb table: sequencing (native downcall first, recipient swap after) lives at the
call sites in `Window`, and `Window.closed` already guards double close.

- [ ] **Step 5: Make `Window` own the holder and expose the five operations**

Add `private val textInputClientHolder = TextInputClientHolder()` and
`private var closed = false` to `Window`. Add:

```kotlin
public fun setTextInputClient(client: TextInputClient) {
    // The downcall can finalize a live composition, which calls back into the previous
    // client; swap the recipient only after the native side has switched over.
    ffiDownCall { desktop_win32_h.window_set_text_input_client(ptr, textInputClientHolder.native) }
    textInputClientHolder.textInputClient = client
}

public fun clearTextInputClient() {
    ffiDownCall { desktop_win32_h.window_clear_text_input_client(ptr) }
    textInputClientHolder.textInputClient = TextInputClient.Noop
}

public fun setImeEnabled(enabled: Boolean) {
    ffiDownCall { desktop_win32_h.window_set_ime_enabled(ptr, enabled) }
}

public fun notifySelectionChanged() {
    ffiDownCall { desktop_win32_h.window_notify_selection_changed(ptr) }
}

public fun notifyLayoutChanged() {
    ffiDownCall { desktop_win32_h.window_notify_layout_changed(ptr) }
}
```

Replace `close` with this idempotent order. Do not close the holder from `clearTextInputClient`:

```kotlin
override fun close() {
    if (closed) return
    clearTextInputClient()
    ffiDownCall { desktop_win32_h.window_drop(ptr) }
    closed = true
    textInputClientHolder.close()
}
```

- [ ] **Step 6: Decode the language event**

Import `NativeInputLanguageChangedEvent`; append to the sealed class:

```kotlin
@ConsistentCopyVisibility
public data class InputLanguageChanged internal constructor(
    val hkl: Long,
    val localeName: String,
) : Event()
```

Append the tag branch and decoder:

```kotlin
desktop_win32_h.NativeEvent_InputLanguageChanged() -> inputLanguageChanged(s)
```

```kotlin
internal fun inputLanguageChanged(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.input_language_changed(s)
    return Event.InputLanguageChanged(
        hkl = NativeInputLanguageChangedEvent.hkl(nativeEvent),
        localeName = NativeInputLanguageChangedEvent.locale_name(nativeEvent).getString(0),
    )
}
```

- [ ] **Step 7: Run managed and integration checks**

```powershell
.\gradlew.bat :kotlin-desktop-toolkit:test --tests "org.jetbrains.desktop.win32.tests.TextInputClientTests"
.\gradlew.bat build
```

Expected: eight focused tests pass; full build regenerates and compiles Rust, C, Java, and Kotlin
bindings successfully.

- [ ] **Step 8: Commit**

```powershell
git add kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/TextInputClient.kt kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/Converters.kt kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/Window.kt kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/Event.kt kotlin-desktop-toolkit/src/test/kotlin/org/jetbrains/desktop/win32/tests/TextInputClientTests.kt
git commit -m "feat(win32): expose text input client"
```

---

### Task 9: Phase-1 editable sample and documentation gate

**Files:**
- Create: `sample/src/main/kotlin/org/jetbrains/desktop/sample/win32/ToyTextInputWin32.kt`
- Modify: `sample/src/main/kotlin/org/jetbrains/desktop/sample/win32/SkikoSampleWin32.kt`
- Modify: `native/desktop-win32/docs/ARCHITECTURE.md`
- Modify: `native/desktop-win32/docs/FFI_CONVENTIONS.md`
- Modify: `native/desktop-win32/docs/SUBSYSTEMS.md`
- Modify: `native/desktop-win32/docs/TODO.md`

**Interfaces:**
- Consumes: managed API from Task 8 and the sample's physical canvas plus scale.
- Produces: a committed, editable client that reports client-logical caret geometry and exercises
  system-drawn Phase 1 without requiring a separate test application.

- [ ] **Step 1: Add `ToyTextInputWin32`**

Create the file below. It uses Skia Paragraph with the platform font manager and explicit Windows,
Japanese, Simplified-Chinese, Korean, and generic families; raw `Font.drawString` is not acceptable
because it provides neither shaping nor fallback for the manual IME matrix. Keep the imports sorted
as shown so the lint gate is executable.

```kotlin
package org.jetbrains.desktop.sample.win32

import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.Keyboard
import org.jetbrains.desktop.win32.LogicalPoint
import org.jetbrains.desktop.win32.LogicalRect
import org.jetbrains.desktop.win32.LogicalSize
import org.jetbrains.desktop.win32.PointerButton
import org.jetbrains.desktop.win32.TextInputClient
import org.jetbrains.desktop.win32.TextRange
import org.jetbrains.desktop.win32.UnderlineSegment
import org.jetbrains.desktop.win32.VirtualKey
import org.jetbrains.desktop.win32.Window
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.FontMgr
import org.jetbrains.skia.Paint
import org.jetbrains.skia.PaintMode
import org.jetbrains.skia.Rect
import org.jetbrains.skia.paragraph.FontCollection
import org.jetbrains.skia.paragraph.Paragraph
import org.jetbrains.skia.paragraph.ParagraphBuilder
import org.jetbrains.skia.paragraph.ParagraphStyle
import org.jetbrains.skia.paragraph.TextStyle
import kotlin.math.abs

class ToyTextInputWin32(
    private val window: Window,
    var origin: LogicalPoint,
    var size: LogicalSize,
) : TextInputClient, AutoCloseable {
    private val fontCollection = FontCollection().apply { setDefaultFontManager(FontMgr.default) }
    private val fontSize = 18f
    private val padding = 8f
    private var horizontalOffset = 0f
    private val buffer = StringBuilder()
    private var cursor = 0
    private var anchor = 0
    private var marked: TextRange? = null
    private var underlines: List<UnderlineSegment> = emptyList()
    private var compositionBackup: CompositionBackup? = null

    private data class CompositionBackup(
        val replacedText: String,
        val range: TextRange,
        val cursor: Int,
        val anchor: Int,
    )

    override fun selectedRange(): TextRange {
        val start = minOf(cursor, anchor)
        return TextRange(start.toLong(), (maxOf(cursor, anchor) - start).toLong())
    }

    override fun caretRect(range: TextRange): LogicalRect {
        val index = range.location.toInt().coerceIn(0, buffer.length)
        return LogicalRect(
            LogicalPoint(origin.x + padding + measurePrefix(index) - horizontalOffset, origin.y + padding),
            LogicalSize(1f, lineHeight()),
        )
    }

    override fun insertText(text: String) {
        val target = marked ?: selectedRange()
        replace(target, text)
        cursor = target.location.toInt() + text.length
        anchor = cursor
        clearCompositionMetadata()
        changed()
    }

    override fun setMarkedText(
        text: String,
        selectedRange: TextRange?,
        underlines: List<UnderlineSegment>,
    ) {
        val target = marked ?: this.selectedRange().also { range ->
            val start = range.location.toInt()
            compositionBackup = CompositionBackup(
                replacedText = buffer.substring(start, start + range.length.toInt()),
                range = range,
                cursor = cursor,
                anchor = anchor,
            )
        }
        replace(target, text)
        marked = TextRange(target.location, text.length.toLong())
        this.underlines = underlines
        val local = selectedRange ?: TextRange(text.length.toLong(), 0)
        anchor = target.location.toInt() + local.location.toInt()
        cursor = anchor + local.length.toInt()
        changed()
    }

    override fun unmarkText() {
        clearCompositionMetadata()
        changed()
    }

    override fun discardMarkedText() {
        val mark = marked
        val backup = compositionBackup
        if (mark != null && backup != null) {
            replace(mark, backup.replacedText)
            cursor = backup.cursor
            anchor = backup.anchor
        }
        clearCompositionMetadata()
        changed()
    }

    private fun replace(range: TextRange, text: String) {
        val start = range.location.toInt()
        buffer.replace(start, start + range.length.toInt(), text)
    }

    private fun clearCompositionMetadata() {
        marked = null
        underlines = emptyList()
        compositionBackup = null
    }

    private fun changed() {
        revealCaret()
        window.notifySelectionChanged()
        window.requestRedraw()
    }

    fun reflow(newSize: LogicalSize) {
        size = newSize
        revealCaret()
        window.notifyLayoutChanged()
        window.requestRedraw()
    }

    fun handleEvent(event: Event): EventHandlerResult = when (event) {
        is Event.KeyDown -> handleKeyDown(event)
        is Event.PointerDown -> {
            if (event.button != PointerButton.Left || !hitTest(event.locationInWindow)) {
                EventHandlerResult.Continue
            } else if (marked != null) {
                // Keep the native composition and its rollback snapshot coherent. The sample
                // deliberately ignores app-side selection edits until IMM commits or cancels it.
                EventHandlerResult.Stop
            } else {
                cursor = findIndex(event.locationInWindow.x - origin.x - padding + horizontalOffset)
                anchor = cursor
                changed()
                EventHandlerResult.Stop
            }
        }
        is Event.ScrollWheelX -> handleScroll(event.scrollingDelta, event.locationInWindow)
        is Event.ScrollWheelY -> handleScroll(event.scrollingDelta, event.locationInWindow)
        else -> EventHandlerResult.Continue
    }

    private fun handleKeyDown(event: Event.KeyDown): EventHandlerResult {
        if (Keyboard.getKeyState(VirtualKey.Control).isDown ||
            Keyboard.getKeyState(VirtualKey.Menu).isDown
        ) {
            return EventHandlerResult.Continue
        }
        if (marked != null) {
            // Let the IME consume arrows/deletion while a preedit exists; do not mutate the app
            // selection or clear `compositionBackup` behind the native state machine.
            event.translate()
            return EventHandlerResult.Stop
        }
        when (event.virtualKey) {
            VirtualKey.Left -> move(-1)
            VirtualKey.Right -> move(1)
            VirtualKey.Home -> moveTo(0)
            VirtualKey.End -> moveTo(buffer.length)
            VirtualKey.Back -> deleteBackward()
            VirtualKey.Delete -> deleteForward()
            else -> {
                event.translate()
                return EventHandlerResult.Stop
            }
        }
        changed()
        return EventHandlerResult.Stop
    }

    private fun move(delta: Int) = moveTo((cursor + delta).coerceIn(0, buffer.length))

    private fun moveTo(index: Int) {
        cursor = index
        anchor = index
    }

    private fun deleteBackward() {
        if (cursor != anchor) {
            replace(selectedRange(), "")
            moveTo(minOf(cursor, anchor))
        } else if (cursor > 0) {
            buffer.deleteCharAt(cursor - 1)
            moveTo(cursor - 1)
        }
    }

    private fun deleteForward() {
        if (cursor != anchor) {
            replace(selectedRange(), "")
            moveTo(minOf(cursor, anchor))
        } else if (cursor < buffer.length) {
            buffer.deleteCharAt(cursor)
        }
    }

    fun draw(canvas: Canvas, scale: Float) {
        val x = origin.x * scale
        val y = origin.y * scale
        val width = size.width * scale
        val height = size.height * scale
        Paint().use { paint ->
            paint.color = 0xFF_20_20_20.toInt()
            canvas.drawRect(Rect.makeXYWH(x, y, width, height), paint)
            paint.mode = PaintMode.STROKE
            paint.strokeWidth = scale
            paint.color = 0xFF_70_70_70.toInt()
            canvas.drawRect(Rect.makeXYWH(x, y, width, height), paint)
        }
        buildParagraph(buffer.toString(), scale, 0xFF_F0_F0_F0.toInt()).use { paragraph ->
            val textX = x + (padding - horizontalOffset) * scale
            val textY = y + (height - paragraph.height) / 2f
            Paint().use { paint ->
                if (cursor != anchor) {
                    val start = minOf(cursor, anchor)
                    val end = maxOf(cursor, anchor)
                    paint.color = 0xFF_26_4F_78.toInt()
                    canvas.drawRect(
                        Rect.makeLTRB(
                            textX + measurePrefix(start, scale),
                            textY,
                            textX + measurePrefix(end, scale),
                            textY + paragraph.height,
                        ),
                        paint,
                    )
                }
                paragraph.paint(canvas, textX, textY)
                val caretX = textX + measurePrefix(cursor, scale)
                paint.color = 0xFF_F0_F0_F0.toInt()
                paint.strokeWidth = 1.5f * scale
                canvas.drawLine(caretX, textY, caretX, textY + paragraph.height, paint)
            }
        }
    }

    private fun buildParagraph(text: String, scale: Float, color: Int): Paragraph {
        val style = TextStyle().apply {
            setFontSize(fontSize * scale)
            setFontFamilies(
                arrayOf(
                    "Segoe UI",
                    "Yu Gothic UI",
                    "Microsoft YaHei UI",
                    "Malgun Gothic",
                    "Nirmala UI",
                    "sans-serif",
                ),
            )
            setColor(color)
        }
        val paragraph = ParagraphBuilder(ParagraphStyle(), fontCollection).use { builder ->
            builder.pushStyle(style)
            builder.addText(text.ifEmpty { " " })
            builder.build()
        }
        paragraph.layout(Float.MAX_VALUE)
        return paragraph
    }

    private fun measurePrefix(index: Int, scale: Float = 1f): Float {
        val end = index.coerceIn(0, buffer.length)
        if (end == 0) return 0f
        return buildParagraph(buffer.substring(0, end), scale, 0).use { it.maxIntrinsicWidth }
    }

    private fun lineHeight(): Float = buildParagraph(" ", 1f, 0).use { it.height }

    private fun revealCaret() {
        val contentWidth = (size.width - 2f * padding).coerceAtLeast(0f)
        val caretX = measurePrefix(cursor)
        horizontalOffset = when {
            caretX < horizontalOffset -> caretX
            caretX > horizontalOffset + contentWidth -> caretX - contentWidth
            else -> horizontalOffset
        }.coerceIn(0f, maxOf(0f, measurePrefix(buffer.length) - contentWidth))
    }

    private fun handleScroll(delta: Int, point: LogicalPoint): EventHandlerResult {
        if (!hitTest(point)) return EventHandlerResult.Continue
        val contentWidth = (size.width - 2f * padding).coerceAtLeast(0f)
        val maxOffset = maxOf(0f, measurePrefix(buffer.length) - contentWidth)
        horizontalOffset = (horizontalOffset - delta / 4f).coerceIn(0f, maxOffset)
        window.notifyLayoutChanged()
        window.requestRedraw()
        return EventHandlerResult.Stop
    }

    private fun hitTest(point: LogicalPoint): Boolean =
        point.x in origin.x..(origin.x + size.width) && point.y in origin.y..(origin.y + size.height)

    private fun findIndex(x: Float): Int = (0..buffer.length).minBy { index ->
        abs(measurePrefix(index) - x)
    }

    override fun close(): Unit = Unit
}
```

The draw method receives a physical canvas and scale, while `caretRect` and stored geometry remain
client-logical.

- [ ] **Step 2: Register, route, draw, and close in `SkottieWindow`**

Import `LogicalPoint`, then add to `SkottieWindow`:

```kotlin
private val textInput = ToyTextInputWin32(
    window = window,
    origin = LogicalPoint(40f, titleBarHeight + 24f),
    size = LogicalSize(560f, 52f),
)
private var imeEnabled = true
private var clientRegistered = true

init {
    window.setTextInputClient(textInput)
}

override fun handleEvent(event: Event): EventHandlerResult {
    when (event) {
        is Event.WindowResize -> {
            val logical = event.size.toLogical(event.scale)
            textInput.reflow(LogicalSize(maxOf(160f, logical.width - 80f), 52f))
        }
        is Event.WindowScaleChanged -> {
            val logical = event.size.toLogical(event.scale)
            textInput.reflow(LogicalSize(maxOf(160f, logical.width - 80f), 52f))
        }
        else -> Unit
    }
    if (event is Event.InputLanguageChanged) {
        Logger.info { "Input language: HKL=${event.hkl}, locale=${event.localeName}" }
    }
    if (event is Event.KeyDown &&
        event.virtualKey == VirtualKey.I &&
        Keyboard.getKeyState(VirtualKey.Control).isDown
    ) {
        imeEnabled = !imeEnabled
        window.setImeEnabled(imeEnabled)
        Logger.info { "IME enabled: $imeEnabled" }
        return EventHandlerResult.Stop
    }
    if (event is Event.KeyDown &&
        event.virtualKey == VirtualKey.T &&
        Keyboard.getKeyState(VirtualKey.Control).isDown
    ) {
        clientRegistered = !clientRegistered
        if (clientRegistered) window.setTextInputClient(textInput) else window.clearTextInputClient()
        Logger.info { "Text input client registered: $clientRegistered" }
        return EventHandlerResult.Stop
    }
    val textResult = textInput.handleEvent(event)
    return if (textResult == EventHandlerResult.Stop) textResult else super.handleEvent(event)
}
```

At the end of `Canvas.draw`, after rendering the animation, add:

```kotlin
textInput.draw(this, scale)
```

Replace the current `SkottieWindow.close` with:

```kotlin
override fun close() {
    timer.cancel()
    try {
        super.close()
    } finally {
        textInput.close()
    }
}
```

`super.close()` already calls `window.destroy()` before `window.close()`, so native callback
teardown precedes the sample client's resource release. Keep Ctrl+N routing in `ApplicationState`
unchanged.

- [ ] **Step 3: Update Phase-1 documentation exactly**

- Add `ime.rs` / `ime_api.rs` and `TextInputClient.kt` to the repository layout in
  `ARCHITECTURE.md`; add a Phase-1 data-flow paragraph (system preedit, `WM_CHAR` commit).
- Add a `Text input / IMM32` section to `SUBSYSTEMS.md` with files, active/enabled gates,
  client-logical coordinates, context association, and current Phase-1 limitation.
- Add an `FFI_CONVENTIONS.md` subsection documenting Rust-to-Kotlin `BorrowedUtf8` /
  `BorrowedArray`, query out-parameters, the `usize::MAX` sentinel, stable upcall Arena, and
  replacement/clear ordering.
- Replace the broad IME capability gap in `TODO.md` with one narrow Phase-2 item: system-drawn IME
  works; self-drawn `GCS_*` composition is still pending. Do not remove the entry until Task 13.

- [ ] **Step 4: Run the Phase-1 gate**

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32
cargo check --manifest-path native/Cargo.toml -p desktop-win32
.\gradlew.bat build
.\gradlew.bat :sample:runSkikoSampleWin32
```

Expected automated result: Rust tests/check and full build pass. Manual result: with no client,
behavior is unchanged; with the sample client, initial IME typing works without an enable call,
system composition/candidates follow the caret, commit calls `insertText`, and explicit disable /
enable detaches/restores the default context. Use Ctrl+T to clear/re-register the client and Ctrl+I
to disable/enable IME; both states are logged. Resize the window and wheel-scroll over the editor;
the field reflows/scrolls and calls `notifyLayoutChanged` so the candidate remains at the caret.

- [ ] **Step 5: Commit**

```powershell
git add sample/src/main/kotlin/org/jetbrains/desktop/sample/win32/ToyTextInputWin32.kt sample/src/main/kotlin/org/jetbrains/desktop/sample/win32/SkikoSampleWin32.kt native/desktop-win32/docs/ARCHITECTURE.md native/desktop-win32/docs/FFI_CONVENTIONS.md native/desktop-win32/docs/SUBSYSTEMS.md native/desktop-win32/docs/TODO.md
git commit -m "feat(win32): add editable IME sample"
```

---

### Task 10: Testable IMM readers and underline conversion

**Files:**
- Modify: `native/desktop-win32/src/win32/ime.rs`

**Interfaces:**
- Consumes: IMM constants, the private `ImmContext` transport, and Task-1 range/underline types.
- Produces: injectable `CompositionSource`, the `u32` clause decoder, `PreeditSnapshot`, and
  `CompositionSnapshot::read` for Tasks 11–12.

- [ ] **Step 1: Write failing decoder/source tests**

Append to `ime.rs::tests`:

```rust
fn utf16_units(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

fn u32_bytes(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|value| value.to_ne_bytes()).collect()
}

#[derive(Default)]
struct FakeSource {
    result: Vec<u16>,
    composition: Vec<u16>,
    attributes: Vec<u8>,
    clauses: Vec<u8>,
    cursor: Option<usize>,
    fail_on: Option<IME_COMPOSITION_STRING>,
}

impl FakeSource {
    fn ensure_available(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<()> {
        anyhow::ensure!(self.fail_on != Some(which), "injected {which:?} failure");
        Ok(())
    }
}

impl CompositionSource for FakeSource {
    fn bytes(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u8>> {
        self.ensure_available(which)?;
        Ok(match which {
            GCS_COMPATTR => self.attributes.clone(),
            GCS_COMPCLAUSE => self.clauses.clone(),
            _ => Vec::new(),
        })
    }

    fn utf16(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<u16>> {
        self.ensure_available(which)?;
        Ok(match which {
            GCS_RESULTSTR => self.result.clone(),
            GCS_COMPSTR => self.composition.clone(),
            _ => Vec::new(),
        })
    }

    fn cursor(&self) -> Option<usize> {
        self.cursor
    }
}

#[test]
fn u32_decoder_rejects_misaligned_data() {
    assert!(decode_u32_bytes(&[0, 0, 0]).is_err());
}

#[test]
fn underline_conversion_maps_clauses_and_targets() {
    let underlines = underlines_from_parts(
        &[
            u8::try_from(ATTR_INPUT).unwrap(),
            u8::try_from(ATTR_INPUT).unwrap(),
            u8::try_from(ATTR_TARGET_CONVERTED).unwrap(),
            u8::try_from(ATTR_TARGET_CONVERTED).unwrap(),
        ],
        &[0, 2, 4],
        4,
    );
    assert_eq!(
        underlines,
        vec![
            UnderlineSegment {
                range: TextRange { location: 0, length: 2 },
                style: UnderlineStyle::Dotted,
                target_clause: false,
            },
            UnderlineSegment {
                range: TextRange { location: 2, length: 2 },
                style: UnderlineStyle::Thick,
                target_clause: true,
            },
        ],
    );
}

#[test]
fn malformed_clauses_fall_back_to_whole_preedit() {
    assert_eq!(
        underlines_from_parts(&[], &[1, 3], 3),
        fallback_underlines(3),
    );
}

#[test]
fn partial_preedit_flags_refetch_complete_preedit() {
    let source = FakeSource {
        composition: utf16_units("かな"),
        attributes: vec![u8::try_from(ATTR_INPUT).unwrap(), u8::try_from(ATTR_INPUT).unwrap()],
        clauses: u32_bytes(&[0, 2]),
        cursor: Some(1),
        ..Default::default()
    };
    for flag in [GCS_COMPATTR, GCS_COMPCLAUSE, GCS_CURSORPOS, GCS_DELTASTART] {
        let snapshot = CompositionSnapshot::read(&source, flag.0).unwrap();
        let preedit = snapshot.preedit.unwrap();
        assert_eq!(preedit.text, "かな");
        assert_eq!(preedit.selected, TextRange { location: 1, length: 0 });
    }
}

#[test]
fn snapshot_reads_result_and_new_preedit_together() {
    let source = FakeSource {
        result: utf16_units("確定"),
        composition: utf16_units("つぎ"),
        cursor: Some(1),
        ..Default::default()
    };
    let snapshot = CompositionSnapshot::read(&source, GCS_RESULTSTR.0 | GCS_COMPSTR.0).unwrap();
    assert_eq!(snapshot.result.as_deref(), Some("確定"));
    assert_eq!(snapshot.preedit.unwrap().text, "つぎ");
}

#[test]
fn empty_preedit_is_present_and_cursor_is_clamped() {
    let empty = CompositionSnapshot::read(&FakeSource::default(), GCS_COMPSTR.0).unwrap();
    assert_eq!(empty.preedit.unwrap().text, "");
    assert!(!empty.cancelled);

    let source = FakeSource {
        composition: utf16_units("かな"),
        cursor: Some(99),
        ..Default::default()
    };
    let clamped = CompositionSnapshot::read(&source, GCS_COMPSTR.0).unwrap();
    assert_eq!(clamped.preedit.unwrap().selected, TextRange { location: 2, length: 0 });
}

#[test]
fn hidden_cursor_renders_preedit_without_selection() {
    let source = FakeSource {
        composition: utf16_units("かな"),
        cursor: None,
        ..Default::default()
    };
    let snapshot = CompositionSnapshot::read(&source, GCS_COMPSTR.0).unwrap();
    assert_eq!(snapshot.preedit.unwrap().selected, TextRange::none());
}

#[test]
fn status_only_mask_is_cancellation() {
    let snapshot = CompositionSnapshot::read(&FakeSource::default(), CS_INSERTCHAR | CS_NOMOVECARET).unwrap();
    assert!(snapshot.cancelled);
    assert!(snapshot.result.is_none());
    assert!(snapshot.preedit.is_none());
}

#[test]
fn optional_decoration_failure_uses_fallback() {
    let source = FakeSource {
        composition: utf16_units("かな"),
        fail_on: Some(GCS_COMPATTR),
        ..Default::default()
    };
    let snapshot = CompositionSnapshot::read(&source, GCS_COMPSTR.0).unwrap();
    assert_eq!(snapshot.preedit.unwrap().underlines, fallback_underlines(2));
}

#[test]
fn core_read_failure_is_returned_before_any_action() {
    let source = FakeSource { fail_on: Some(GCS_COMPSTR), ..Default::default() };
    assert!(CompositionSnapshot::read(&source, GCS_COMPSTR.0).is_err());
}
```

Run `cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::ime::tests::u32_decoder`.

Expected: compilation fails because the source, decoder, and snapshots are missing.

- [ ] **Step 2: Add the source abstraction and production transport**

Merge these symbols into the existing IMM import:

```rust
use windows::Win32::UI::Input::Ime::{
    ATTR_CONVERTED, ATTR_FIXEDCONVERTED, ATTR_INPUT, ATTR_TARGET_CONVERTED,
    ATTR_TARGET_NOTCONVERTED, GCS_COMPATTR, GCS_COMPCLAUSE, GCS_COMPREADATTR,
    GCS_COMPREADCLAUSE, GCS_COMPREADSTR, GCS_COMPSTR, GCS_CURSORPOS, GCS_DELTASTART,
    GCS_RESULTCLAUSE, GCS_RESULTREADCLAUSE, GCS_RESULTREADSTR, GCS_RESULTSTR,
    IME_COMPOSITION_STRING, ImmGetCompositionStringW,
};
```

Add the generic two-call transport as a private `ImmContext` method — the payload buffer is sized
in its natural element type, so UTF-16 strings are read directly into `Vec<u16>`:

```rust
impl ImmContext {
    /// Two-call `ImmGetCompositionStringW` transport: probe for the byte size, then fill.
    fn composition_payload<T: Copy + Default>(&self, which: IME_COMPOSITION_STRING) -> anyhow::Result<Vec<T>> {
        // SAFETY: this guard owns a valid HIMC; null buffer and zero size is the documented probe.
        let required = unsafe { ImmGetCompositionStringW(self.himc, which, None, 0) };
        anyhow::ensure!(required >= 0, "ImmGetCompositionStringW({which:?}) probe failed: {required}");
        let byte_count = usize::try_from(required)?;
        anyhow::ensure!(
            byte_count.is_multiple_of(size_of::<T>()),
            "ImmGetCompositionStringW({which:?}) returned a misaligned byte count: {byte_count}"
        );
        if byte_count == 0 {
            return Ok(Vec::new());
        }
        let mut payload = vec![T::default(); byte_count / size_of::<T>()];
        // SAFETY: `payload` is writable for exactly `byte_count` bytes and this guard owns the HIMC.
        let written = unsafe { ImmGetCompositionStringW(self.himc, which, Some(payload.as_mut_ptr().cast()), u32::try_from(byte_count)?) };
        anyhow::ensure!(written >= 0, "ImmGetCompositionStringW({which:?}) fill failed: {written}");
        let written = usize::try_from(written)?;
        anyhow::ensure!(
            written <= byte_count,
            "ImmGetCompositionStringW({which:?}) returned {written} > {byte_count}"
        );
        payload.truncate(written / size_of::<T>());
        Ok(payload)
    }
}
```

Add the `CompositionSource` trait and its `ImmContext` impl from §7.3 (`bytes`, `utf16`, and
`cursor`; the first two delegate to `composition_payload`).

- [ ] **Step 3: Add the clause decoder and underline mapping**

```rust
fn decode_u32_bytes(bytes: &[u8]) -> anyhow::Result<Vec<u32>> {
    anyhow::ensure!(bytes.len() % size_of::<u32>() == 0, "unaligned u32 byte count: {}", bytes.len());
    Ok(bytes
        .chunks_exact(size_of::<u32>())
        .map(|part| u32::from_ne_bytes([part[0], part[1], part[2], part[3]]))
        .collect())
}

fn underlines_from_parts(attrs: &[u8], clauses: &[u32], preedit_len: usize) -> Vec<UnderlineSegment> {
    let bounds = clauses
        .iter()
        .map(|value| usize::try_from(*value))
        .collect::<Result<Vec<_>, _>>();
    let Ok(bounds) = bounds else { return fallback_underlines(preedit_len) };
    if bounds.len() < 2
        || bounds.first() != Some(&0)
        || bounds.last() != Some(&preedit_len)
        || bounds.iter().any(|value| *value > preedit_len)
        || bounds.windows(2).any(|pair| pair[0] > pair[1])
    {
        return fallback_underlines(preedit_len);
    }

    bounds.windows(2).filter_map(|pair| {
        let (start, end) = (pair[0], pair[1]);
        if start >= end { return None; }
        let attribute = attrs.get(start).copied().map_or(ATTR_INPUT, u32::from);
        let (style, target_clause) = match attribute {
            ATTR_TARGET_CONVERTED => (UnderlineStyle::Thick, true),
            ATTR_TARGET_NOTCONVERTED => (UnderlineStyle::Dotted, true),
            ATTR_CONVERTED | ATTR_FIXEDCONVERTED => (UnderlineStyle::Solid, false),
            _ => (UnderlineStyle::Dotted, false),
        };
        Some(UnderlineSegment {
            range: TextRange { location: start, length: end - start },
            style,
            target_clause,
        })
    }).collect()
}

fn fallback_underlines(preedit_len: usize) -> Vec<UnderlineSegment> {
    (preedit_len != 0).then_some(UnderlineSegment {
        range: TextRange { location: 0, length: preedit_len },
        style: UnderlineStyle::Dotted,
        target_clause: false,
    }).into_iter().collect()
}
```

- [ ] **Step 4: Build one complete snapshot before callbacks**

Add the complete masks, then add the snapshot types:

```rust
const GCS_ANY: u32 = GCS_COMPREADSTR.0
    | GCS_COMPREADATTR.0
    | GCS_COMPREADCLAUSE.0
    | GCS_COMPSTR.0
    | GCS_COMPATTR.0
    | GCS_COMPCLAUSE.0
    | GCS_CURSORPOS.0
    | GCS_DELTASTART.0
    | GCS_RESULTREADSTR.0
    | GCS_RESULTREADCLAUSE.0
    | GCS_RESULTSTR.0
    | GCS_RESULTCLAUSE.0;

const GCS_PREEDIT_UPDATE: u32 = GCS_COMPSTR.0
    | GCS_COMPATTR.0
    | GCS_COMPCLAUSE.0
    | GCS_CURSORPOS.0
    | GCS_DELTASTART.0;
```

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreeditSnapshot {
    pub(crate) text: String,
    /// `TextRange::none()` when the IME shows no composition cursor.
    pub(crate) selected: TextRange,
    pub(crate) underlines: Vec<UnderlineSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
            let underlines = match (
                source.bytes(GCS_COMPATTR),
                source.bytes(GCS_COMPCLAUSE).and_then(|bytes| decode_u32_bytes(&bytes)),
            ) {
                (Ok(attrs), Ok(clauses)) => underlines_from_parts(&attrs, &clauses, length),
                (Err(err), _) | (_, Err(err)) => {
                    log::warn!("reading IME underline data failed: {err:#}");
                    fallback_underlines(length)
                }
            };
            Some(PreeditSnapshot { text, selected, underlines })
        } else {
            None
        };
        Ok(Self { result, preedit, cancelled: gcs & GCS_ANY == 0 })
    }
}
```

- [ ] **Step 5: Verify and commit**

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::ime::tests::
cargo check --manifest-path native/Cargo.toml -p desktop-win32
git add native/desktop-win32/src/win32/ime.rs
git commit -m "feat(win32): read IME composition state"
```

Expected: decoder/source tests pass; `cargo check` ends with `Finished`.

---

### Task 11: Revision-guarded composition apply

**Files:**
- Modify: `native/desktop-win32/src/win32/ime.rs`

**Interfaces:**
- Consumes: `CompositionSnapshot` from Task 10.
- Produces: injectable `CompositionSink` and `apply_composition` — one function that delivers a
  snapshot with direct, revision-checked calls in a fixed order — testable without Win32 or
  Kotlin callbacks.

- [ ] **Step 1: Write failing apply-order tests**

The `FakeSink` records every callback in order (Task 12 extends it with reentrancy injection):

```rust
#[test]
fn apply_commits_result_before_starting_next_preedit() {
    let sink = FakeSink::default();
    let source = FakeSource {
        result: utf16_units("確定"),
        composition: utf16_units("つぎ"),
        cursor: Some(1),
        ..Default::default()
    };
    apply_owned_composition(&sink, &source, GCS_RESULTSTR.0 | GCS_COMPSTR.0);
    assert_eq!(&*sink.callbacks.borrow(), &["insert", "set_marked", "update_windows"]);
    assert_eq!(&*sink.inserted.borrow(), &["確定"]);
    assert!(sink.app_marked.get());
}

#[test]
fn apply_distinguishes_empty_preedit_and_cancel() {
    let empty_preedit = FakeSink::default();
    empty_preedit.app_marked.set(true);
    apply_owned_composition(&empty_preedit, &FakeSource::default(), GCS_COMPSTR.0);
    assert_eq!(&*empty_preedit.callbacks.borrow(), &["discard", "update_windows"]);
    assert!(!empty_preedit.app_marked.get());

    let cancel = FakeSink::default();
    cancel.composition_active.set(true);
    cancel.app_marked.set(true);
    apply_owned_composition(&cancel, &FakeSource::default(), 0);
    assert_eq!(&*cancel.callbacks.borrow(), &["discard"]);
    assert!(!cancel.composition_active.get());
    assert!(!cancel.app_marked.get());
}

#[test]
fn owned_post_end_result_is_still_applied() {
    let sink = FakeSink::default();
    let source = FakeSource { result: utf16_units("한"), ..Default::default() };
    apply_owned_composition(&sink, &source, GCS_RESULTSTR.0);
    assert_eq!(&*sink.callbacks.borrow(), &["insert"]);
}
```

Run `cargo test --manifest-path native/Cargo.toml -p desktop-win32
win32::ime::tests::apply_commits_result_before_starting_next_preedit`.

Expected: compilation fails because the sink trait and apply functions are missing.

- [ ] **Step 2: Implement the sink and the apply function**

```rust
pub(crate) trait CompositionSink {
    fn revision(&self) -> u64;
    fn set_app_marked(&self, value: bool) -> u64;
    fn clear_composition(&self) -> u64;
    fn insert_text(&self, text: &str);
    fn set_marked_text(&self, preedit: &PreeditSnapshot);
    fn discard_marked_text(&self);
    fn update_windows(&self);
}

/// Deliver one composition snapshot to the sink. Every client callback can synchronously reenter
/// composition teardown (a nested END or focus loss); comparing the revision after each callback
/// against the last state transition this function made detects that and abandons the remaining,
/// now-stale steps.
fn apply_composition(sink: &impl CompositionSink, snapshot: CompositionSnapshot) {
    let mut expected_revision = sink.revision();
    if let Some(result) = snapshot.result.filter(|text| !text.is_empty()) {
        sink.insert_text(&result);
        if sink.revision() != expected_revision {
            return;
        }
        expected_revision = sink.set_app_marked(false);
    }
    if let Some(preedit) = snapshot.preedit {
        if preedit.text.is_empty() {
            sink.discard_marked_text();
            if sink.revision() != expected_revision {
                return;
            }
            sink.set_app_marked(false);
        } else {
            expected_revision = sink.set_app_marked(true);
            sink.set_marked_text(&preedit);
            if sink.revision() != expected_revision {
                return;
            }
        }
        sink.update_windows();
    } else if snapshot.cancelled {
        sink.clear_composition();
        sink.discard_marked_text();
    }
}

pub(crate) fn apply_owned_composition(sink: &impl CompositionSink, source: &impl CompositionSource, gcs: u32) {
    match CompositionSnapshot::read(source, gcs) {
        Ok(snapshot) => apply_composition(sink, snapshot),
        Err(err) => log::warn!("reading IME composition failed; keeping ownership until next update or END: {err:#}"),
    }
}
```

State transitions land *before* their corresponding client mutation (`set_app_marked(true)` before
`set_marked_text`), and commit / empty-preedit callbacks keep the previous marked state during the
callback so a nested focus loss still takes the Phase-2 cancel path. No revision check follows the
final call of a branch — there is nothing left to abort. `apply_owned_composition` returns nothing:
a failed core read logs, keeps message ownership, and recovers on the next update or END, and no
caller distinguishes that from the applied case.

- [ ] **Step 3: Verify and commit**

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::ime::tests::apply_
cargo check --manifest-path native/Cargo.toml -p desktop-win32
git add native/desktop-win32/src/win32/ime.rs
git commit -m "feat(win32): reduce IME composition updates"
```

Expected: all apply tests pass; `cargo check` ends with `Finished`.

---

### Task 12: Atomic Phase-2 message ownership

**Files:**
- Modify: `native/desktop-win32/src/win32/ime.rs`
- Modify: `native/desktop-win32/src/win32/event_loop.rs`

**Interfaces:**
- Consumes: Task-10 snapshot reader, the Task-11 apply function, and Task-2 enabled-client/state
  helpers.
- Produces: the `Window` sink impl, the finalizing delivery path, and gated
  SETCONTEXT/START/COMPOSITION/END ownership with inline preedit and owned read-error recovery.

- [ ] **Step 1: Write failing reentrancy and owned-read tests**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReenterOn {
    Insert,
    SetMarked,
}

#[derive(Default)]
struct FakeSink {
    revision: Cell<u64>,
    composition_active: Cell<bool>,
    app_marked: Cell<bool>,
    callbacks: RefCell<Vec<&'static str>>,
    inserted: RefCell<Vec<String>>,
    reenter_on: Option<ReenterOn>,
}

impl FakeSink {
    fn advance_revision(&self) -> u64 {
        let next = self.revision.get() + 1;
        self.revision.set(next);
        next
    }

    fn callback(&self, name: &'static str, kind: ReenterOn) {
        self.callbacks.borrow_mut().push(name);
        if self.reenter_on == Some(kind) {
            // Model a nested END/focus-loss finalization before returning to the outer callback.
            self.composition_active.set(false);
            self.app_marked.set(false);
            self.advance_revision();
        }
    }
}

impl CompositionSink for FakeSink {
    fn revision(&self) -> u64 { self.revision.get() }
    fn set_app_marked(&self, value: bool) -> u64 {
        self.app_marked.set(value);
        self.advance_revision()
    }
    fn clear_composition(&self) -> u64 {
        self.composition_active.set(false);
        self.app_marked.set(false);
        self.advance_revision()
    }
    fn insert_text(&self, text: &str) {
        self.inserted.borrow_mut().push(text.to_owned());
        self.callback("insert", ReenterOn::Insert);
    }
    fn set_marked_text(&self, _preedit: &PreeditSnapshot) { self.callback("set_marked", ReenterOn::SetMarked); }
    fn discard_marked_text(&self) { self.callbacks.borrow_mut().push("discard"); }
    fn update_windows(&self) { self.callbacks.borrow_mut().push("update_windows"); }
}

#[test]
fn apply_stops_after_reentrant_end_during_insert() {
    let sink = FakeSink { reenter_on: Some(ReenterOn::Insert), ..Default::default() };
    sink.app_marked.set(true);
    apply_composition(&sink, CompositionSnapshot {
        result: Some("commit".to_owned()),
        preedit: Some(PreeditSnapshot {
            text: "stale".to_owned(),
            selected: TextRange { location: 0, length: 0 },
            underlines: Vec::new(),
        }),
        cancelled: false,
    });
    assert_eq!(&*sink.callbacks.borrow(), &["insert"]);
    assert!(!sink.app_marked.get());
}

#[test]
fn apply_stops_before_positioning_after_reentrant_focus_loss() {
    let sink = FakeSink { reenter_on: Some(ReenterOn::SetMarked), ..Default::default() };
    apply_composition(&sink, CompositionSnapshot {
        result: None,
        preedit: Some(PreeditSnapshot {
            text: "preedit".to_owned(),
            selected: TextRange { location: 0, length: 0 },
            underlines: Vec::new(),
        }),
        cancelled: false,
    });
    assert_eq!(&*sink.callbacks.borrow(), &["set_marked"]);
    assert!(!sink.app_marked.get());
}

#[test]
fn owned_core_read_failure_preserves_sink_state() {
    let sink = FakeSink::default();
    sink.revision.set(7);
    sink.composition_active.set(true);
    sink.app_marked.set(true);
    let source = FakeSource { fail_on: Some(GCS_COMPSTR), ..Default::default() };
    apply_owned_composition(&sink, &source, GCS_COMPSTR.0);
    assert_eq!(sink.revision.get(), 7);
    assert!(sink.composition_active.get());
    assert!(sink.app_marked.get());
    assert!(sink.callbacks.borrow().is_empty());
}
```

Add `use std::cell::{Cell, RefCell};` inside `ime.rs::tests`, then run
`cargo test --manifest-path native/Cargo.toml -p desktop-win32
win32::ime::tests::apply_stops_after_reentrant_end_during_insert`.

Expected: the reentrancy tests fail until the `Window` sink impl and handlers below exist (the
apply function itself landed in Task 11).

- [ ] **Step 2: Give `Window` the sink impl and the finalizing delivery path**

Add to `window.rs` (this is where every callee lives):

```rust
impl CompositionSink for Window {
    fn revision(&self) -> u64 { self.ime_revision() }
    fn set_app_marked(&self, value: bool) -> u64 { self.ime_set_app_marked(value) }
    fn clear_composition(&self) -> u64 { self.clear_composition_state() }
    fn insert_text(&self, text: &str) {
        let _ = self.with_enabled_client(|client| client.insert_text(text));
    }
    fn set_marked_text(&self, preedit: &PreeditSnapshot) {
        let _ = self.with_enabled_client(|client| {
            client.set_marked_text(&preedit.text, preedit.selected, &preedit.underlines);
        });
    }
    fn discard_marked_text(&self) {
        let _ = self.with_enabled_client(TextInputClient::discard_marked_text);
    }
    fn update_windows(&self) { self.update_ime_windows(); }
}
```

Add to `ime.rs`:

```rust
/// Deliver the result of this window's own `CPS_COMPLETE` finalization. The reentrant
/// `WM_IME_COMPOSITION` arrives while composition state is being torn down, so only
/// `GCS_RESULTSTR` matters — preedit flags describe a composition that no longer exists.
pub(crate) fn apply_finalizing_composition(sink: &impl CompositionSink, source: &impl CompositionSource, gcs: u32) {
    if gcs & GCS_RESULTSTR.0 == 0 {
        return;
    }
    match source.utf16(GCS_RESULTSTR) {
        Ok(units) if !units.is_empty() => sink.insert_text(&String::from_utf16_lossy(&units)),
        Ok(_) => {}
        Err(err) => log::warn!("reading the finalized IME result failed: {err:#}"),
    }
}
```

- [ ] **Step 3: Replace Phase-1 handlers with the full owned handlers**

In `event_loop.rs`, merge `ISC_SHOWUICOMPOSITIONWINDOW` into the IMM import. `WPARAM`, `LPARAM`,
and `LRESULT` come from `Foundation`.

```rust
fn on_ime_setcontext(window: &Window, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    window.enabled_client()?;
    let adjusted = LPARAM(lparam.0 & !(ISC_SHOWUICOMPOSITIONWINDOW as isize));
    // SAFETY: arguments came from this live wndproc; only the documented composition-UI bit changed.
    Some(unsafe { DefWindowProcW(window.hwnd(), msg, wparam, adjusted) })
}

fn on_ime_startcomposition(window: &Window) -> Option<LRESULT> {
    window.enabled_client()?;
    window.ime_start();
    window.update_ime_windows();
    Some(LRESULT(0))
}

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
        // Owning the message keeps the IME from re-emitting it as WM_IME_CHAR/WM_CHAR later,
        // when the client may already be swapped, disabled, or unfocused.
        apply_finalizing_composition(window, &context, gcs);
        return Some(LRESULT(0));
    }
    apply_owned_composition(window, &context, gcs);
    Some(LRESULT(0))
}

fn on_ime_endcomposition(window: &Window) -> Option<LRESULT> {
    window.enabled_client()?;
    if window.ime_is_finalizing() {
        return Some(LRESULT(0));
    }
    if window.ime_end() {
        let _ = window.with_enabled_client(TextInputClient::discard_marked_text);
    }
    Some(LRESULT(0))
}
```

- [ ] **Step 4: Switch all four message arms in one edit**

In `event_loop.rs`, merge `WM_IME_SETCONTEXT` and `WM_IME_COMPOSITION` into the existing
`WindowsAndMessaging` import and `apply_finalizing_composition` / `apply_owned_composition` into
the `ime::{...}` import. Then replace the two Phase-1 arms and add the other two:

```rust
WM_IME_SETCONTEXT => on_ime_setcontext(window, msg, wparam, lparam),
WM_IME_STARTCOMPOSITION => on_ime_startcomposition(window),
WM_IME_COMPOSITION => on_ime_composition(window, lparam),
WM_IME_ENDCOMPOSITION => on_ime_endcomposition(window),
```

Delete `on_ime_startcomposition_phase1` and `on_ime_endcomposition_phase1` in this same commit.
With no enabled client, every handler returns `None`, preserving default system UI and `WM_CHAR`.

- [ ] **Step 5: Verify and commit atomically**

```powershell
cargo test --manifest-path native/Cargo.toml -p desktop-win32 win32::ime::tests::
cargo check --manifest-path native/Cargo.toml -p desktop-win32
git add native/desktop-win32/src/win32/ime.rs native/desktop-win32/src/win32/window.rs native/desktop-win32/src/win32/event_loop.rs
git commit -m "feat(win32): render IME preedit inline"
```

Expected: all reader, apply, and owned-read tests pass; `cargo check` ends with
`Finished`; no commit state exists where system preedit is suppressed without self-drawn callbacks.

---

### Task 13: Phase-2 sample, canonical docs, and final gate

**Files:**
- Modify: `sample/src/main/kotlin/org/jetbrains/desktop/sample/win32/ToyTextInputWin32.kt`
- Modify: `native/desktop-win32/docs/AGENTS.md`
- Modify: `native/desktop-win32/docs/ARCHITECTURE.md`
- Modify: `native/desktop-win32/docs/FFI_CONVENTIONS.md`
- Modify: `native/desktop-win32/docs/SUBSYSTEMS.md`
- Modify: `native/desktop-win32/docs/TODO.md`
- Modify: `native/desktop-win32/docs/specs/2026-07-07-win32-ime-support-design.md`

**Interfaces:**
- Consumes: complete Phase-2 callbacks and sample state from Task 9.
- Produces: visible clause styling/caret, final canonical documentation, and release evidence.

- [ ] **Step 1: Render preedit-relative underlines**

Add `import org.jetbrains.desktop.win32.UnderlineStyle` in sorted order. In the
`buildParagraph(...).use` block in `ToyTextInputWin32.draw`, after `paragraph.paint` and before
drawing the caret, add:

```kotlin
val mark = marked
if (mark != null) {
    for (segment in underlines) {
        val start = mark.location.toInt() + segment.range.location.toInt()
        val end = start + segment.range.length.toInt()
        val x1 = textX + measurePrefix(start, scale)
        val x2 = textX + measurePrefix(end, scale)
        Paint().use { paint ->
            paint.color = if (segment.targetClause) 0xFF_FF_CC_33.toInt() else 0xFF_E0_E0_E0.toInt()
            paint.strokeWidth = when (segment.style) {
                UnderlineStyle.Solid -> 1f * scale
                UnderlineStyle.Dotted -> 1f * scale
                UnderlineStyle.Thick -> 2f * scale
            }
            if (segment.style == UnderlineStyle.Dotted) {
                var x = x1
                while (x < x2) {
                    canvas.drawPoint(x, textY + paragraph.height - scale, paint)
                    x += 3f * scale
                }
            } else {
                val underlineY = textY + paragraph.height - scale
                canvas.drawLine(x1, underlineY, x2, underlineY, paint)
            }
        }
    }
}
```

The normal cursor already uses the document-global `cursor`, which Task 9 updates from the
preedit-relative selection, so no second composition-caret coordinate system is needed.

- [ ] **Step 2: Finalize canonical docs**

- Change the design status from `proposed` to `implemented` only after all automated/manual gates
  pass; keep checkboxes as the execution record.
- Update `ARCHITECTURE.md` and `SUBSYSTEMS.md` from Phase-1 wording to final self-drawn behavior,
  including the enabled-client ownership gate, snapshot-before-callback rule, and read-failure
  recovery.
- Keep the reverse-borrow/out-parameter/Arena contract in `FFI_CONVENTIONS.md`.
- Add IMM32 and callback-lifetime entries to `AGENTS.md`'s navigation/surprises only where they
  help future maintainers.
- Remove the resolved IMM32/TSF capability-gap entry from `TODO.md`; do not leave a stale partial
  item. Add no TSF follow-up unless a concrete unimplemented requirement remains.

- [ ] **Step 3: Run the complete automated gate**

```powershell
cargo fmt --manifest-path native/Cargo.toml --all -- --check
cargo test --manifest-path native/Cargo.toml -p desktop-win32
cargo check --manifest-path native/Cargo.toml -p desktop-win32
.\gradlew.bat :kotlin-desktop-toolkit:generateBindingsForWin32
.\gradlew.bat :kotlin-desktop-toolkit:test --tests "org.jetbrains.desktop.win32.tests.TextInputClientTests"
.\gradlew.bat build
.\gradlew.bat lint
```

Expected: every command exits zero; focused Kotlin tests pass; build completes the full
Rust → cbindgen → JExtract → Kotlin pipeline.

- [ ] **Step 4: Run the complete manual matrix**

Run `.\gradlew.bat :sample:runSkikoSampleWin32`, then record results for every §10 manual check:
no-client fallback; initial enabled state; detach/restore; Japanese or Chinese inline preedit and
candidates; Esc cancel; blur/disable/client-swap finalize; Japanese result+preedit; Korean
post-END result; caret moves/scroll/reflow notifications; non-IME language changes; mixed-DPI
movement; close during composition without a freed-stub callback.

- [ ] **Step 5: Commit**

```powershell
git add sample/src/main/kotlin/org/jetbrains/desktop/sample/win32/ToyTextInputWin32.kt native/desktop-win32/docs/AGENTS.md native/desktop-win32/docs/ARCHITECTURE.md native/desktop-win32/docs/FFI_CONVENTIONS.md native/desktop-win32/docs/SUBSYSTEMS.md native/desktop-win32/docs/TODO.md native/desktop-win32/docs/specs/2026-07-07-win32-ime-support-design.md
git commit -m "feat(win32): finish IMM32 integration"
```

---

## 10. Verification

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
  preedit-relative selection and underline ranges; the `NOT_FOUND` sentinel ↔ `null` round-trip.
- **Surrogate joining** — a BMP unit; a valid high+low pair; a lone high surrogate; a lone low
  surrogate; an interrupted pair; and clearing the pending unit on client switch / focus loss /
  composition end.
- **Underline mapping** — `ATTR_*` → `UnderlineStyle` / `targetClause`; multi-clause boundaries; and
  the malformed-boundary fallback to a single whole-preedit clause.
- **Coordinate conversion** — client-relative logical → client-relative physical, including the
  full-rectangle `CFS_EXCLUDE` corner conversion.
- **Composition state transitions** — start → compose → commit; start → compose → cancel (empty
  full-GCS mask); empty `GCS_COMPSTR`; observed both-flags-in-one-message and post-END-result
  compatibility cases; cursor-, attribute-, and clause-only preedit refresh; core read failure
  preserving ownership and client/native state; and reentrant finalize.

**Kotlin unit tests** cover only the managed boundary: `NOT_FOUND` ↔ `null`, callback argument
decoding, the holder's recipient and arena lifetime, and `InputLanguageChanged` decoding.
They do not duplicate private Rust algorithms. Run the focused class after it is added:

```powershell
.\gradlew.bat :kotlin-desktop-toolkit:generateBindingsForWin32
.\gradlew.bat :kotlin-desktop-toolkit:test --tests "org.jetbrains.desktop.win32.tests.TextInputClientTests"
```

**Manual check** (needs a real IME installed). Commit `ToyTextInputWin32.kt` and integrate its
editable `TextInputClient` surface into the Win32 Skiko sample (the current sample only logs
`CharacterReceived`), then:

1. Install a Japanese or Chinese IME (Windows Settings → Language). Register the client and answer
   `selectedRange` / `caretRect`; verify initial typing works without an enable call, then exercise
   explicit `setImeEnabled(false)` / `true` detach and restore.
2. Type romaji/pinyin. Phase 1: the IME's composition and candidate windows appear at the caret;
   selecting a candidate delivers the committed text as `insertText`. Phase 2: the preedit renders
   inline via `setMarkedText`, the candidate window tracks the caret, and committing calls
   `insertText` with the phrase.
3. Press Esc mid-composition and confirm `discardMarkedText` drops the preedit; move focus away
   mid-composition and confirm `unmarkText` keeps it.
4. Verify observed compatibility cases: a Korean IME that commits after END and a Japanese IME
   that supplies result + new preedit in one composition message.
5. Move the caret with the mouse / arrows and scroll, calling `notifySelectionChanged` /
   `notifyLayoutChanged`, and confirm the candidate window follows.
6. Switch keyboard layout (Alt+Shift) and confirm `InputLanguageChanged` fires with the HKL and
   locale name, including between two non-IME layouts.
7. Move the field to a monitor at a different DPI and confirm the IME windows stay on the caret.

---

## 11. References

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
