# Win32 IME via TSF (non-transitory, pull model) — Spec & Implementation Plan

Status: proposed · Crate: `native/desktop-win32` · API: Text Services Framework (TSF), non-transitory `ITextStoreACP`

> **DRAFT — NOT IMPLEMENTATION-READY.** This spec has unresolved design holes (see §0). Do not hand it to an implementer as a build plan yet.

This document explains how TSF text input works, the exact windows-rs 0.62.2 surface (verified
against the resolved crate), the design, and an ordered implementation plan. It is **not yet a build
plan** — see §0 for the unresolved design holes that must be closed first.

This specifies a **TSF-based** implementation. Windows offers two application-side IME APIs: the
classic IMM32 (`Imm*` / `WM_IME_*`) and the modern Text Services Framework (TSF, a COM
text-services framework). TSF is the heavier of the two but reaches features IMM32 cannot expose
app-side — notably Korean reconversion of committed text and rich per-range composition attributes.
This document is one complete way to build Win32 IME for `desktop-win32`; it stands alone.

---

## 0. Known gaps (must be resolved before implementation)

This spec is not implementation-ready: the following are real design holes, not wording, and must
be resolved before it becomes a build plan.

- **Synchronization model undefined.** The store keeps a text mirror but the client has no
  full-text or document-length query and no app→TSF change downcalls, so the mirror cannot be
  seeded or kept in sync. Decide: full snapshot/change protocol, or drop the mirror and pull
  synchronously from a richer client while holding the TSF lock.
- **Lock protocol violates the documented contract.** The design queues arbitrary requests in a
  `VecDeque`; Microsoft says coalesce pending requests (write supersedes read), not queue many.
  `reconcile_after_lock()` sends `OnTextChange`/`OnSelectionChange` before `RequestLock` returns;
  Microsoft forbids notifying from within `RequestLock`. Any Chromium-style deviation must be
  stated and justified, not presented as "the documented protocol."
- **`reconcile_after_lock()` is undefined.** It hides cache refresh, selection reconciliation,
  composition start/update/end, replacement, and notification suppression — the core of the
  backend — yet appears as one unexplained call.
- **Phase slicing is wrong.** Composition tracking (the composition sink) must land in the first
  functional phase; without it the TIP's preedit `SetText` writes are indistinguishable from
  commits and get sent as `insertText`. And a non-transitory store has no TIP-drawn composition
  window (the app draws preedit) — the Phase 1 "TIP's composition window appears" claim and the
  scope note contradict each other.
- **Content methods underspecified.** `ITextStoreACP` methods need full contracts: `acpEnd == -1`,
  `TF_E_INVALIDPOS` / range validation, partial `GetText` + `pacpNext`, `TS_RUNINFO`, null-pointer
  rules, `TS_IAS_QUERYONLY`/`TS_IAS_NOQUERY`, selection affinity, `TS_TEXTCHANGE` values, ACP
  overflow past `i32::MAX`, and `AdviseSink` identity/duplicate/mask/unadvise.
- **`ITfInputScope` needs five methods** (`GetInputScopes`, `GetPhrase`, `GetRegularExpression`,
  `GetSRGS`, `GetXML`); the spec supplies one, so it will not compile.
- **Field identity is contradictory.** Per-field document manager vs one client per window; pick a
  per-window store with app-side document reset, or a native per-field handle.
- **COM callbacks must not panic.** `extern "system"` entry points need `catch_unwind`/an
  `ffi_boundary` equivalent; the pseudocode's `unwrap()` and unchecked raw-pointer writes are UB
  on unwind.
- **Key-path assumption unverified.** The claim that no `ITfKeystrokeMgr` is needed is disputed
  (WPF wraps `TestKeyDown`/`KeyDown`/`TestKeyUp`/`KeyUp`); prototype the key path against this
  toolkit's opt-in `translate()` model before prescribing. Define ordinary/dead/system `WM_CHAR`
  behavior while a store is focused.
- **Missing pieces:** an app→native change-notification surface, `OnLayoutChange`/`GetScreenExt`/
  `GetACPFromPoint`/viewport for candidate tracking, a reconversion edit-session mechanism
  (`RequestEditSession` + an `ITfEditSession` object), a password/security policy (a non-transitory
  store exposes the whole buffer to the TIP), a `scale` update path, and a language-change event.

---

## 1. Purpose & scope

Let users type Chinese, Japanese, Korean (and other composed) text into a window whose text is
rendered entirely by the Kotlin application — no native edit control. The application draws its own
text and its own in-progress composition ("preedit"); the installed text service (TIP) draws its own
candidate window.

This design is:

- **TSF, non-transitory.** The application is a COM *text store* (`ITextStoreACP`) that keeps a real,
  addressable text buffer. It does **not** set `TS_SS_TRANSITORY`, so modern TSF features — Korean
  reconversion of committed text, rich per-range display attributes, text intelligence — remain
  available. (A transitory store routes through the IMM32-emulation layer and loses those.)
- **Pull model.** TSF calls *into* the store to read text, read the selection, read caret rectangles,
  and to write text. The store forwards those to a win32 `TextInputClient` that the application
  implements. Its shape resembles the macOS `NSTextInputClient` (a useful FFI template) and is an
  independent win32 type. TSF is inherently pull, so this client fits it directly.

Out of scope: authoring a TIP (this is the *application* side); app-drawn candidate lists (UILess
mode); an IMM32-based implementation (a lighter alternative approach, not covered here).

### 1.1 When to choose TSF over IMM32

IMM32 (the classic `Imm*` / `WM_IME_*` API) is the simpler, lighter alternative. Choose a TSF
implementation like this one only when you need a feature IMM32 cannot reach app-side: **Korean
reconversion of committed text**, **per-range display attributes / text intelligence**, or
**pen/speech-as-text**. Absent one of those, IMM32 is the better choice.

Honest comparison (verified against MS docs + Chromium/Firefox/Windows Terminal/WPF):

| | IMM32 (pull) | TSF non-transitory (this doc) |
|---|---|---|
| Model | `WM_IME_*` messages, synchronous, no COM | COM text store + async document-lock state machine |
| Rust precedent | winit / SDL / Zed / Flutter / GLFW | **none** — trailblazing |
| Size | moderate | ~1,000–1,800 LOC of new COM (Chromium 1,787, WPF 5,236) |
| Testable at build+lint | partial | **no** — correctness is TIP-driven runtime behavior |
| Per-IME quirks | low | high (`TS_E_NOLAYOUT` per-TIP surface; Firefox ships both stacks) |
| Korean reconversion, per-range attrs | ✗ / limited | ✅ |

**Non-transitory is the point.** A *transitory* TSF store (`TS_SS_TRANSITORY`) is not worth building.
Either it is a full `ITextStoreACP` with the flag set — barely simpler than this design (~one line in
`GetStatus`) and it forfeits Korean reconversion, because the flag routes through CUAS (IMM32
emulation); or it is the Windows Terminal *context-owner* model (~half the code) which abandons the
addressable text buffer, does not expose the pull-client surface, and also runs on CUAS. Either way a
transitory store adds no feature over IMM32, so IMM32 wins for "simplest." This design is
non-transitory precisely because that is the only variant that pays for TSF's cost.

---

## 2. Background: how TSF text input works

Read this once; the rest of the document assumes it.

### 2.1 The cast

TSF is a COM framework. Two kinds of object matter:

- **The manager** (`msctf.dll`, in-process). You create it and call into it. Interfaces:
  `ITfThreadMgr2` (one per UI thread), `ITfDocumentMgr` (one per editable field), `ITfContext`
  (the edit context on a field).
- **Your text store** (you implement it in Rust). It is one COM object exposing `ITextStoreACP` (the
  document) and `ITfContextOwnerCompositionSink` (composition lifecycle). TSF drives your store to
  read and edit "the document."

"ACP" = *application character position*: every offset TSF passes you is a **UTF-16 code-unit index**
into your document. Kotlin `String` is UTF-16, so these indices line up end to end with no conversion.

### 2.2 The flow

1. On the UI thread you create an `ITfThreadMgr2`, `ActivateEx` it (get a `TfClientId`), and — per
   focusable field — create an `ITfDocumentMgr`, create an `ITfContext` passing **your store** as the
   document, and `Push` the context. When the field gains focus you call `SetFocus(docMgr)`.
2. The user presses keys. `msctf` intercepts them during ordinary message processing (see §2.5) and
   drives the active TIP. Composed text does **not** arrive as `WM_CHAR`; it arrives through your store.
3. To read or change the document the TIP (via the manager) takes a **document lock**: it calls your
   `ITextStoreACP::RequestLock`; you grant it by synchronously calling back
   `ITextStoreACPSink::OnLockGranted`. During that one call the TIP reads (`GetText`, `GetSelection`)
   and writes (`SetText`, `SetSelection`, `InsertTextAtSelection`). The lock is valid only for the
   duration of `OnLockGranted`.
4. Composition is bracketed by `ITfContextOwnerCompositionSink::OnStartComposition` /
   `OnEndComposition`. Between them, the text the TIP writes into the composing range is *preedit*;
   when composition ends, that text is *committed*.
5. The TIP positions its own candidate window by asking your store for the caret rectangle in **screen
   coordinates** via `ITextStoreACP::GetTextExt`.

### 2.3 The store mirrors the app's document

Your store must answer `GetText`/`GetSelection` synchronously under a lock. So the store keeps a
**UTF-16 mirror** of the application's text plus the current selection. Two directions keep it in sync:

- **TIP → app.** During a write lock the TIP calls `SetText`/`InsertTextAtSelection`; you update the
  mirror. When the edit finishes you translate what changed into calls on the Kotlin
  `TextInputClient` (`setMarkedText` while composing, `insertText` on commit).
- **App → TIP.** When the application changes its own text or selection (user clicks, arrow keys,
  programmatic edit), you update the mirror and notify TSF via
  `OnTextChange`/`OnSelectionChange`/`OnLayoutChange` so the TIP re-reads.

This mirror-plus-notify pattern is exactly what Chromium's `TSFTextStore` does.

### 2.4 Non-transitory status

There is no `TS_SS_NONTRANSITORY` flag — you get a non-transitory store simply by **not** setting
`TS_SS_TRANSITORY` in `GetStatus`. That single omission keeps the store on modern TSF. For a
plain-text field, report `dwStaticFlags = TS_SS_NOHIDDENTEXT` and `dwDynamicFlags = TS_SD_READONLY`
only when the field is read-only.

### 2.5 The message pump

A plain `GetMessageW` → `TranslateMessage` → `DispatchMessageW` loop is sufficient. Once the thread
manager is active and a store-backed document manager has focus, `msctf` intercepts keystrokes during
message processing on its own. `ITfMessagePump` and `ITfKeystrokeMgr` are timing *optimizations*, not
requirements — Chromium and WPF use neither. Keep `TranslateMessage` for ordinary (non-IME) typing;
composed text comes through the store, so do not also treat `WM_CHAR` as composition input while a
store-backed field has focus.

### 2.6 Candidates and reconversion

The TIP draws its own candidate window (activate with `dwFlags = 0`; the app draws nothing). Because
the store is non-transitory and can return arbitrary committed text, the app can also **launch
reconversion** of a committed selection through `ITfFnReconversion` (§10).

---

## 3. Current state of `desktop-win32`

Files an implementer will touch, and what they do today:

- **`src/win32/application.rs`** — calls `OleInitialize(None)` on the UI thread (an STA
  initialization). TSF is created on this same thread; **no extra `CoInitializeEx` is needed or wanted**.
- **`src/win32/event_loop.rs`** — `EventLoop::run` is the pump (`GetMessageW` → `DispatchMessageW`;
  it translates `VK_PROCESSKEY` key messages so the active IME composes).
  `EventLoop::window_proc` is the `WM_*` dispatch table; `on_keyevent` drops `VK_PROCESSKEY` and
  `on_keyevent`/`on_char` build key/char events. No TSF handling exists.
- **`src/win32/window.rs`** — the `Window` struct; `HWND` in an `AtomicPtr` read via `Window::hwnd()`;
  per-window mutable state uses `Cell`/`RefCell`.
- **`src/win32/window_api.rs`** — per-window downcalls shaped `window_<verb>(window_ptr: WindowPtr, …)`
  wrapped in `ffi_boundary("name", || { … })`.
- **`src/win32/data_object.rs`** — an existing `#[implement(IDataObject)]` COM object: the reference
  pattern for implementing a COM interface in Rust in this crate.
- **`src/win32/events_api.rs`** — contains a `windows_core::link!("user32.dll" …)` shim
  (`TranslateMessageEx`): the pattern to hand-bind any msctf export the crate does not expose.
- **`native/desktop-macos/src/macos/text_input_client.rs`** + Kotlin
  [`TextInputClient.kt`](../../../../kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/macos/TextInputClient.kt)
  — the callback-table FFI pattern to copy (a code template; the win32 client is a separate type).
- **`Cargo.toml`** — the `windows` feature list has `Win32_System_Com`, `Win32_System_Ole`,
  `Win32_Graphics_Gdi`, `Win32_UI_WindowsAndMessaging`, `Win32_Foundation`. It is **missing**
  `Win32_UI_TextServices` and `Win32_System_Variant` — both must be added (§12).

---

## 4. The pull `TextInputClient` API

The store forwards everything to one Kotlin interface. The application owns its document and answers
queries; the backend calls in to read and to mutate.

```kotlin
public interface TextInputClient {
    // Queries — the backend asks; the app answers from its own buffer.
    public fun hasMarkedText(): Boolean
    public fun markedRange(): TextRange?
    public fun selectedRange(): TextRange?
    public fun textForRange(range: TextRange): String?
    public fun caretRectForRange(range: TextRange): RectAndRange   // screen coords, logical px
    public fun characterIndexForPoint(point: LogicalPoint): Long?

    // Mutations — the backend applies TIP edits to the app document.
    public fun insertText(text: String, replacementRange: TextRange?)
    public fun setMarkedText(
        text: String,
        selectedRange: TextRange?,
        replacementRange: TextRange?,
        underlines: List<UnderlineSegment>,
    )
    public fun unmarkText()

    public object Noop : TextInputClient { /* null / false / no-ops */ }
}

public data class TextRange(val location: Long, val length: Long)          // UTF-16 code units
public data class RectAndRange(val rect: LogicalRect, val actualRange: TextRange?)
public data class UnderlineSegment(val range: TextRange, val style: UnderlineStyle, val targetClause: Boolean)
public enum class UnderlineStyle { Solid, Dotted, Thick }
```

- Register with `window.setTextInputClient(client)` on focus; `window.clearTextInputClient()` on blur;
  `window.setImeEnabled(enabled)` for numeric/password fields (§9).
- `TextInputClient.Noop` lets a consumer adopt incrementally.
- The mapping from `ITextStoreACP` to these methods:

| `ITextStoreACP` / sink | `TextInputClient` |
|---|---|
| `GetText` | `textForRange` |
| `GetSelection` | `selectedRange` |
| `GetTextExt` | `caretRectForRange` |
| `GetACPFromPoint` | `characterIndexForPoint` |
| `SetText` / `InsertTextAtSelection` inside a composition | `setMarkedText` |
| `SetText` / `InsertTextAtSelection` outside a composition; `OnEndComposition` | `insertText` |
| composition marked range | `markedRange` / `hasMarkedText` |
| `OnEndComposition` | `unmarkText` |

**Attribute divergence:** macOS carries preedit styling in an attributed string; TSF carries it as
per-range display attributes. `setMarkedText` takes an explicit `underlines: List<UnderlineSegment>`
so both backends map into one signature. The FFI passes the struct-of-callbacks down exactly like
`native/desktop-macos/src/macos/text_input_client.rs` (`NativeTextInputClient`), registered via
`window_set_text_input_client(window_ptr, client_ptr)`.

---

## 5. Architecture

New module `src/win32/tsf/` with:

- **`mod.rs`** — the `ImeThread` (per-thread manager) and `ImeField` (per-field document manager),
  plus the two window downcalls.
- **`text_store.rs`** — the `#[implement(ITextStoreACP, ITfContextOwnerCompositionSink, ITfInputScope)]`
  object: the mirror buffer, the lock state machine, and all content/layout methods.
- **`composition.rs`** — the `#[implement(ITfTextEditSink)]` object (or the same object) that reads the
  composing range and display attributes on `OnEndEdit` and calls the pull client.

Threading: everything lives on the UI thread (the OLE STA from `application.rs`). The `ITfThreadMgr2`
is thread-affine — never send it across threads.

Interior mutability: every implemented COM method takes `&self`, so mutable store state lives behind
`RefCell`/`Cell`. **Because the sink calls re-enter the store on the same thread** (the TIP reads you
during `OnLockGranted`; TSF calls `RequestLock` back during your change notifications), you must drop
every `RefCell` borrow before calling out to a sink or to Kotlin. Read scalars into locals first.

The store object:

```rust
#[implement(ITextStoreACP, ITfContextOwnerCompositionSink, ITfInputScope)]
struct TextStore {
    client: TextInputClientPtr,      // the Kotlin pull client (callback table)
    hwnd: HWND,
    scale: Cell<f32>,                // DPI scale for screen-rect conversion
    doc: RefCell<Vec<u16>>,          // UTF-16 mirror of the app document
    sel: Cell<(i32, i32)>,           // selection (acpStart, acpEnd)
    lock: RefCell<LockState>,        // lock state machine (see §6)
    sink: RefCell<Option<ITextStoreACPSink>>, // handed to us by AdviseSink
    sink_mask: Cell<u32>,            // TS_AS_* mask from AdviseSink
    composing: Cell<Option<(i32, i32)>>, // active composing range, or None
    category_mgr: ITfCategoryMgr,        // for display-attribute GUID resolution
    display_attr_mgr: ITfDisplayAttributeMgr,
    read_only: Cell<bool>,
}
```

---

## 6. The text store: document lock + content methods

This is the core. The full `ITextStoreACP_Impl` trait is 26 methods; you implement the content/layout
ones with real bodies and give the embedded/attribute-query ones minimal correct stubs.

### 6.1 The exact windows-rs 0.62.2 signatures

The `#[implement]` macro (re-exported from `windows_core`, backed by `windows-implement`) generates a
`ITextStoreACP_Impl` trait you implement for the macro-made `TextStore_Impl` type. Key signatures
(verbatim from the resolved crate):

```rust
fn RequestLock(&self, dwlockflags: u32) -> windows_core::Result<windows_core::HRESULT>;
fn GetStatus(&self) -> windows_core::Result<TS_STATUS>;
fn QueryInsert(&self, acpteststart: i32, acptestend: i32, cch: u32,
               pacpresultstart: *mut i32, pacpresultend: *mut i32) -> Result<()>;
fn GetSelection(&self, ulindex: u32, ulcount: u32,
                pselection: *mut TS_SELECTION_ACP, pcfetched: *mut u32) -> Result<()>;
fn SetSelection(&self, ulcount: u32, pselection: *const TS_SELECTION_ACP) -> Result<()>;
fn GetText(&self, acpstart: i32, acpend: i32, pchplain: PWSTR, cchplainreq: u32,
           pcchplainret: *mut u32, prgruninfo: *mut TS_RUNINFO, cruninforeq: u32,
           pcruninforet: *mut u32, pacpnext: *mut i32) -> Result<()>;
fn SetText(&self, dwflags: u32, acpstart: i32, acpend: i32, pchtext: &PCWSTR, cch: u32)
           -> Result<TS_TEXTCHANGE>;
fn InsertTextAtSelection(&self, dwflags: u32, pchtext: &PCWSTR, cch: u32,
                         pacpstart: *mut i32, pacpend: *mut i32, pchange: *mut TS_TEXTCHANGE) -> Result<()>;
fn GetEndACP(&self) -> Result<i32>;
fn GetActiveView(&self) -> Result<u32>;                 // TsViewCookie is a bare u32
fn GetACPFromPoint(&self, vcview: u32, ptscreen: *const POINT, dwflags: u32) -> Result<i32>;
fn GetTextExt(&self, vcview: u32, acpstart: i32, acpend: i32,
              prc: *mut RECT, pfclipped: *mut BOOL) -> Result<()>;
fn GetScreenExt(&self, vcview: u32) -> Result<RECT>;
fn GetWnd(&self, vcview: u32) -> Result<HWND>;
fn AdviseSink(&self, riid: *const GUID, punk: Ref<IUnknown>, dwmask: u32) -> Result<()>;
fn UnadviseSink(&self, punk: Ref<IUnknown>) -> Result<()>;
// GetFormattedText, GetEmbedded, QueryInsertEmbedded, InsertEmbedded, InsertEmbeddedAtSelection,
// RequestSupportedAttrs, RequestAttrsAtPosition, RequestAttrsTransitioningAtPosition,
// FindNextAttrTransition, RetrieveRequestedAttrs — return Ok(()) / E_NOTIMPL as noted in §6.5.
```

Note two crate conventions that will trip you up if unnoticed:

- **`RequestLock` returns `Result<HRESULT>`, not a `*mut phrSession` out-param.** The generated vtable
  shim writes your `Ok(hr)` into `*phrSession` and returns `S_OK` from the method; an `Err(e)` sets the
  method's `HRESULT` and leaves `*phrSession` unwritten. So a granted lock is `Ok(session_hr)`, a
  synchronous rejection is `Ok(TS_E_SYNCHRONOUS)`, and a hard failure is `Err(E_FAIL)`.
- `AdviseSink` here is the one **you implement** — `msctf` calls it to hand you the
  `ITextStoreACPSink` you notify on. (Do not confuse it with `ITfSource::AdviseSink`, which *you* call
  to observe `ITfTextEditSink`; that one returns a cookie — §7.)

Constants (verified values): `TS_LF_READ = 2`, `TS_LF_READWRITE = 6` (has the READ bit),
`TS_LF_SYNC = 1`; `TF_E_NOLOCK == TS_E_NOLOCK == 0x80040201`; `TS_E_SYNCHRONOUS = 0x80040208`;
`TS_S_ASYNC = 0x00040300` (a **success** code); `TS_E_NOLAYOUT = 0x80040206`;
`TS_AS_TEXT_CHANGE = 1`, `TS_AS_SEL_CHANGE = 2`, `TS_AS_LAYOUT_CHANGE = 4`; `TS_SS_NOHIDDENTEXT = 8`;
`TS_SD_READONLY = 1`; `TS_DEFAULT_SELECTION = 0xFFFFFFFF`.

### 6.2 The lock state machine

`RequestLock` is called on one thread but re-entrantly. The logic (matching Chromium exactly):

```rust
struct LockState {
    current: u32,             // 0 = unlocked; else (flags & TS_LF_READWRITE): 2 = read, 6 = readwrite
    queue: VecDeque<u32>,     // queued async grants
    notifying: bool,          // guard: change notifications re-enter RequestLock
}

fn RequestLock(&self, dwlockflags: u32) -> Result<HRESULT> {
    let sink = self.sink.borrow().clone().ok_or_else(|| Error::from(E_FAIL))?;

    // Already locked -> this is a reentrant request from inside OnLockGranted.
    {
        let mut st = self.lock.borrow_mut();
        if st.current != 0 {
            if dwlockflags & TS_LF_SYNC != 0 {
                return Ok(TS_E_SYNCHRONOUS);                 // sync denied; method still S_OK
            }
            st.queue.push_back(dwlockflags & TS_LF_READWRITE.0);
            return Ok(TS_S_ASYNC);          // covers the read->readwrite upgrade with no special case
        }
        st.current = dwlockflags & TS_LF_READWRITE.0;         // strips the SYNC bit
    } // drop borrow before calling the sink

    let cur = self.lock.borrow().current;
    let session_hr = match unsafe { sink.OnLockGranted(TEXT_STORE_LOCK_FLAGS(cur)) } {
        Ok(()) => S_OK,
        Err(e) => e.code(),
    };
    self.lock.borrow_mut().current = 0;

    // Drain queued async grants (the reentrant readwrite upgrade lands here).
    loop {
        let next = { let mut st = self.lock.borrow_mut();
                     match st.queue.pop_front() { Some(f) => { st.current = f; f } None => break } };
        let _ = unsafe { sink.OnLockGranted(TEXT_STORE_LOCK_FLAGS(next)) };
        self.lock.borrow_mut().current = 0;
    }

    self.reconcile_after_lock();      // if the mirror diverged from the client, diff + notify (§7.3)
    Ok(session_hr)
}
```

The reentrant read→readwrite upgrade is **not** a special path: the reentrant call hits the
`current != 0 && async` branch, queues, returns `TS_S_ASYNC`, and the drain loop re-grants it.

### 6.3 Lock enforcement on content methods

Every accessor checks the lock first:

```rust
fn has_read_lock(&self) -> bool  { (self.lock.borrow().current & TS_LF_READ.0) == TS_LF_READ.0 }        // true under readwrite too
fn has_write_lock(&self) -> bool { (self.lock.borrow().current & TS_LF_READWRITE.0) == TS_LF_READWRITE.0 }
```

- `GetText`, `GetSelection`, `GetTextExt`, `GetEndACP`, `InsertTextAtSelection(TS_IAS_QUERYONLY)` require a read lock; return `Err(TF_E_NOLOCK)` if absent.
- `SetText`, `SetSelection`, writing `InsertTextAtSelection` require a write lock.

`GetText` copies `self.doc[acpstart..acpend]` (clamped to `GetEndACP`) into `pchplain`, fills the
plain-run info, and sets `*pacpnext`. `GetSelection` writes `self.sel` into `pselection[0]` and
`*pcfetched = 1`. `SetText` is implemented as `SetSelection(acpstart..acpend)` then
`InsertTextAtSelection(text)` (Chromium does this) — it splices the mirror and returns the
`TS_TEXTCHANGE { acpStart, acpOldEnd, acpNewEnd }`. `QueryInsert` returns the clamped insert range.
`GetEndACP` returns `self.doc.borrow().len() as i32`. `GetActiveView` returns a fixed cookie (e.g. `0`).

### 6.4 `GetStatus` (non-transitory)

```rust
fn GetStatus(&self) -> Result<TS_STATUS> {
    Ok(TS_STATUS {
        dwStaticFlags: TS_SS_NOHIDDENTEXT,                                  // NO TS_SS_TRANSITORY
        dwDynamicFlags: if self.read_only.get() { TS_SD_READONLY } else { 0 },
    })
}
```

`GetStatus` runs outside any lock and takes no cookie, so the read-only bit must come from synchronous
app state. Omitting `TS_SS_TRANSITORY` is the whole point — it keeps reconversion and per-range
attributes alive.

### 6.5 The stub methods

The embedded-object and attribute-query methods are not needed for plain-text CJK input. Give them
minimal correct returns so the vtable is complete: `GetFormattedText`/`GetEmbedded`/`InsertEmbedded*`
return `Err(E_NOTIMPL)`; `QueryInsertEmbedded` returns `Ok(FALSE)`; the `RequestAttrs*` /
`FindNextAttrTransition` / `RetrieveRequestedAttrs` group returns `Ok(())` with `*pcfetched = 0`.

---

## 7. Composition, commit & display attributes

### 7.1 The sinks

`ITfContextOwnerCompositionSink` gives composition edges; `ITfTextEditSink::OnEndEdit` gives the one
place with a valid read cookie for reading the composing range and its attributes.

```rust
fn OnStartComposition(&self, view: Ref<ITfCompositionView>) -> Result<BOOL> {
    self.cache_view(view.ok()?.clone());
    self.composing.set(Some((0, 0)));     // real extent read in OnEndEdit
    Ok(BOOL(1))                            // pfOk = TRUE allows the composition (FALSE rejects it)
}
fn OnUpdateComposition(&self, view: Ref<ITfCompositionView>, _new: Ref<ITfRange>) -> Result<()> {
    self.cache_view(view.ok()?.clone());
    Ok(())
}
fn OnEndComposition(&self, _view: Ref<ITfCompositionView>) -> Result<()> {
    let (text, marked) = self.marked_snapshot();
    self.client.insert_text(&text, marked);   // residual preedit is now committed
    self.client.unmark_text();
    self.composing.set(None);
    Ok(())
}
```

Do not try to read the preedit in `OnStartComposition` — the range is 0-length there. The TIP writes
the preedit via `SetText` during a write lock; the authoritative read happens in `OnEndEdit`.

### 7.2 Reading the composing range and attributes in `OnEndEdit`

```rust
fn OnEndEdit(&self, pic: Ref<ITfContext>, ec: u32, _rec: Ref<ITfEditRecord>) -> Result<()> {
    let ctx = pic.ok()?;
    let Some(view) = self.cached_view() else { return Ok(()) };

    // 1. composing span as ACP (UTF-16) offsets
    let range: ITfRange = unsafe { view.GetRange()? };
    let range_acp: ITfRangeACP = range.cast()?;
    let (mut anchor, mut cch) = (0i32, 0i32);
    unsafe { range_acp.GetExtent(&mut anchor, &mut cch)? };
    if cch <= 0 { return Ok(()); }
    self.composing.set(Some((anchor, anchor + cch)));

    // 2. preedit text + caret from the mirror
    let text = self.text_utf16(anchor, cch);
    let sel  = self.selection_within(anchor, cch);

    // 3. per-range display attributes over the composing range
    let underlines = unsafe { self.read_underlines(&ctx, ec, &range)? };

    // 4. push to the pull client
    self.client.set_marked_text(&text, sel, self.replacement_range(), &underlines);
    Ok(())
}
```

`ITfCompositionView::GetRange` returns an `ITfRange`; `.cast::<ITfRangeACP>()` then `GetExtent` gives
the UTF-16 `(anchor, cch)`.

### 7.3 The display-attribute read

```rust
unsafe fn read_underlines(&self, ctx: &ITfContext, ec: u32, composing: &ITfRange)
    -> Result<Vec<UnderlineSegment>>
{
    let prop: ITfProperty = ctx.GetProperty(&GUID_PROP_ATTRIBUTE)?;   // is-a ITfReadOnlyProperty
    let mut enum_ranges: Option<IEnumTfRanges> = None;
    prop.EnumRanges(ec, &mut enum_ranges, composing)?;               // restrict scan to the preedit
    let enum_ranges = enum_ranges.unwrap();

    let mut out = Vec::new();
    let mut slot: [Option<ITfRange>; 1] = [None];
    let mut fetched = 0u32;
    while enum_ranges.Next(&mut slot, &mut fetched).is_ok() && fetched == 1 {
        let run = slot[0].take().unwrap();
        let v: VARIANT = prop.GetValue(ec, &run)?;

        // EnumRanges over a target range yields empty VARIANTs for gaps — guard VT_I4.
        if v.Anonymous.Anonymous.vt != VARENUM(VT_I4.0) { continue; }
        // NOTE: THREE `.Anonymous` to reach the leaf `lVal` in windows 0.62.2.
        let atom = v.Anonymous.Anonymous.Anonymous.lVal as u32;      // TfGuidAtom

        let guid = self.category_mgr.GetGUID(atom)?;
        let mut info: Option<ITfDisplayAttributeInfo> = None;
        self.display_attr_mgr.GetDisplayAttributeInfo(&guid, &mut info, core::ptr::null_mut())?;
        let mut da = TF_DISPLAYATTRIBUTE::default();
        info.unwrap().GetAttributeInfo(&mut da)?;

        let acp: ITfRangeACP = run.cast()?;
        let (mut a, mut c) = (0i32, 0i32);
        acp.GetExtent(&mut a, &mut c)?;

        let target = da.bAttr == TF_ATTR_TARGET_CONVERTED || da.bAttr == TF_ATTR_TARGET_NOTCONVERTED;
        let style = if da.fBoldLine.as_bool() || da.bAttr == TF_ATTR_TARGET_CONVERTED {
            UnderlineStyle::Thick
        } else if matches!(da.lsStyle, TF_LS_DOT | TF_LS_DASH | TF_LS_SQUIGGLE) {
            UnderlineStyle::Dotted
        } else {
            UnderlineStyle::Solid
        };
        out.push(UnderlineSegment { range: TextRange::new(a, c), style, target_clause: target });
    }
    Ok(out)
}
```

`category_mgr` / `display_attr_mgr` are created once via
`CoCreateInstance(&CLSID_TF_CategoryMgr, …)` / `CoCreateInstance(&CLSID_TF_DisplayAttributeMgr, …)`.
`TF_DISPLAYATTRIBUTE` fields: `lsStyle` (`TF_LS_*`), `fBoldLine`, `bAttr` (`TF_ATTR_TARGET_CONVERTED`
is the active clause the user is converting).

### 7.4 Commit vs preedit

- Text the TIP writes **inside** the composing range → `setMarkedText` (§7.2).
- Text written **outside** any composition (`self.composing == None`) → direct `insertText`.
- `OnEndComposition` → commit the residual preedit with `insertText`, then `unmarkText`.

### 7.5 App-side change notifications

When the application changes its own text/selection for a reason other than a TIP edit, update the
mirror and notify — never from inside an `ITextStoreACP` method:

```rust
fn notify(&self, change: TS_TEXTCHANGE, text_changed: bool, sel_changed: bool) {
    if self.lock.borrow().notifying { return; }
    let Some(sink) = self.sink.borrow().clone() else { return };
    let mask = self.sink_mask.get();
    self.lock.borrow_mut().notifying = true;               // set, drop borrow, then call sink
    unsafe {
        if text_changed && mask & TS_AS_TEXT_CHANGE != 0 { let _ = sink.OnTextChange(TEXT_STORE_TEXT_CHANGE_FLAGS(0), &change); }
        if sel_changed  && mask & TS_AS_SEL_CHANGE  != 0 { let _ = sink.OnSelectionChange(); }
    }
    self.lock.borrow_mut().notifying = false;
}
```

Update the mirror **before** notifying — TSF may call straight back into `GetText`/`GetSelection`
under a fresh lock during the notification. Gate each notification on the `AdviseSink` mask.

---

## 8. Positioning the candidate window

The TIP positions its own window from `GetTextExt`. Return the caret/range rect in **screen pixels**:

```rust
fn GetTextExt(&self, _vc: u32, acp_start: i32, acp_end: i32,
              prc: *mut RECT, pf_clipped: *mut BOOL) -> Result<()> {
    if !self.has_read_lock() { return Err(TF_E_NOLOCK.into()); }
    let Some(screen_rect) = self.client.caret_rect_for_range(acp_start, acp_end) else {
        return Err(TS_E_NOLAYOUT.into());    // only when layout is genuinely unavailable
    };
    let screen = self.logical_rect_to_physical(screen_rect, self.scale.get()); // DPI scale only — already screen coords
    unsafe { *prc = screen; *pf_clipped = BOOL(0); }
    Ok(())
}
fn GetScreenExt(&self, _vc: u32) -> Result<RECT> { /* whole control, screen px */ }
fn GetWnd(&self, _vc: u32) -> Result<HWND> { Ok(self.hwnd) }
```

The crate is `PER_MONITOR_AWARE_V2`, so scale the caret rect — already screen coordinates — by the
window's current DPI. Return `TS_E_NOLAYOUT` **only** when layout is not yet computed; if the
window is minimized/off-screen, return `Ok` with `prc = {0,0,0,0}`. Returning `TS_E_NOLAYOUT`
gratuitously makes some TIPs suppress their candidate window — answer with a best-effort rect instead.

---

## 9. Focus, enable/disable, input scopes

- **Focus / enable:** `thread_mgr.SetFocus(&field.doc_mgr)` when a text field gains focus.
- **Disable:** keep one **empty** `ITfDocumentMgr` (created with no context pushed — WPF's `_dimEmpty`)
  and `thread_mgr.SetFocus(&dim_empty)` for a non-text/disabled field; use `SetFocus(None)` on window
  blur/shutdown.
- **Per-field hints:** implement `ITfInputScope` on the store. `GetInputScopes` returns a
  `CoTaskMemAlloc`'d `InputScope[]` (the TIP frees it) — e.g. `[IS_NUMBER]`, `[IS_DIGITS]`, `[IS_URL]`,
  `[IS_PASSWORD]`, or `[IS_DEFAULT]` (`= 0`, meaning unrestricted — not "off"). The `msctf`
  `SetInputScopes` free function is not bound in windows 0.62.2; the interface route needs no shim.

```rust
fn GetInputScopes(&self, out: *mut *mut InputScope, count: *mut u32) -> Result<()> {
    unsafe { *out = cotaskmem_dup(&[IS_NUMBER]); *count = 1; }   // CoTaskMemAlloc a copy
    Ok(())
}
```

---

## 10. Reconversion (committed text)

Because the store is non-transitory, the app can launch reconversion of a committed selection:

```rust
fn reconvert_selection(t: &ImeThread, sel_range: &ITfRange) -> Result<()> {
    let fp: ITfFunctionProvider = unsafe { t.thread_mgr.GetFunctionProvider(&GUID_SYSTEM_FUNCTIONPROVIDER)? };
    let unk = unsafe { fp.GetFunction(&GUID::zeroed() /* GUID_NULL */, &ITfFnReconversion::IID)? };
    let reconv: ITfFnReconversion = unk.cast()?;
    let (mut new_range, mut convertable) = (None, BOOL(0));
    unsafe { reconv.QueryRange(sel_range, &mut new_range, &mut convertable)? };
    if convertable.as_bool() {
        if let Some(r) = new_range.as_ref() { unsafe { reconv.Reconvert(r)? }; }
    }
    Ok(())
}
```

Expose this as a window command (e.g. a context-menu "Reconvert"). It works only because `GetText`
answers over the whole committed buffer — do not enable the transitory extension.

---

## 11. Lifecycle & message pump

### 11.1 Startup / teardown

```rust
struct ImeThread { thread_mgr: ITfThreadMgr2, client_id: u32, dim_empty: ITfDocumentMgr }

impl ImeThread {
    fn new() -> Result<Self> {
        // Thread is already an OLE STA (application.rs OleInitialize) — no CoInitializeEx here.
        let thread_mgr: ITfThreadMgr2 =
            unsafe { CoCreateInstance(&CLSID_TF_ThreadMgr, None, CLSCTX_INPROC_SERVER)? };
        let mut client_id = 0u32;
        unsafe { thread_mgr.ActivateEx(&mut client_id, 0)? };   // 0 => the TIP draws its own candidates
        let dim_empty = unsafe { thread_mgr.CreateDocumentMgr()? };
        Ok(Self { thread_mgr, client_id, dim_empty })
    }
}
impl Drop for ImeThread {
    fn drop(&mut self) {
        unsafe { let _ = self.thread_mgr.SetFocus(None); let _ = self.thread_mgr.Deactivate(); }
    }
}

struct ImeField { doc_mgr: ITfDocumentMgr, context: ITfContext, sink_cookie: u32 }

impl ImeField {
    fn new(t: &ImeThread, store: ITextStoreACP, edit_sink: IUnknown) -> Result<Self> {
        let doc_mgr = unsafe { t.thread_mgr.CreateDocumentMgr()? };
        let (mut context, mut edit_cookie) = (None, 0u32);
        unsafe { doc_mgr.CreateContext(t.client_id, 0, &store.cast::<IUnknown>()?, &mut context, &mut edit_cookie)? };
        let context = context.unwrap();
        unsafe { doc_mgr.Push(&context)? };
        let source: ITfSource = context.cast()?;
        // ITfSource::AdviseSink takes (riid, punk) and RETURNS the cookie — keep it for UnadviseSink.
        let sink_cookie = unsafe { source.AdviseSink(&ITfTextEditSink::IID, &edit_sink)? };
        Ok(Self { doc_mgr, context, sink_cookie })
    }
}
impl Drop for ImeField {
    fn drop(&mut self) {
        unsafe {
            if let Ok(source) = self.context.cast::<ITfSource>() { let _ = source.UnadviseSink(self.sink_cookie); }
            let _ = self.doc_mgr.Pop(TF_POPF_ALL);
        }
    }
}
```

`OleUninitialize` stays at application scope — per-field teardown must not `CoUninitialize`.

### 11.2 Message pump

Leave `EventLoop::run` as it is: `GetMessageW` → `DispatchMessageW`, with `TranslateMessage` called on
`VK_PROCESSKEY` key messages. That translation is harmless for TSF —
`msctf` intercepts keystrokes itself once the thread manager is active and a store-backed document
manager has focus, so composed text arrives through the store, not `WM_CHAR`. Do **not** add
`ITfMessagePump`/`ITfKeystrokeMgr`, and keep `on_keyevent`'s `VK_PROCESSKEY` drop as-is.

---

## 12. Cargo features & FFI wiring

Add to the `windows` dependency in `native/desktop-win32/Cargo.toml`:

- `"Win32_UI_TextServices"` — the whole TSF module (interfaces, `TS_*`/`TF_*` constants, structs).
- `"Win32_System_Variant"` — `VARIANT` (used reading display attributes; also required to compile the
  `ITextStoreACP` vtable, whose signatures reference `IDataObject`/`FORMATETC`).

`Win32_System_Com`, `Win32_System_Ole`, `Win32_Foundation` are already present. The `#[implement]`
macro comes from `windows_core` (backed by `windows-implement` 0.60.2, already resolved) and needs no
extra feature.

FFI: define `NativeTextInputClient` (struct of fn pointers) and `window_set_text_input_client` /
`window_clear_text_input_client` / `window_set_ime_enabled` downcalls, modelled on
`native/desktop-macos/src/macos/text_input_client.rs`. `AutoDropArray` carries `UnderlineSegment[]`.
Regenerate the C header (cbindgen) and Java bindings (JExtract) via the build; author the Kotlin
`TextInputClient` interface + `Window.kt` wrappers. The `ffi-sync-checker` covers the Rust → header →
Java → Kotlin chain.

---

## 13. Implementation plan

Ordered; each task is independently buildable.

### Phase 0 — scaffolding
1. **Cargo features** — add `Win32_UI_TextServices` + `Win32_System_Variant`.
2. **Module skeleton** — create `src/win32/tsf/{mod,text_store,composition}.rs`; `mod tsf;`.
3. **`ImeThread`** — `CoCreateInstance` `ITfThreadMgr2`, `ActivateEx(_, 0)`, create `dim_empty`; create
   `category_mgr` / `display_attr_mgr`. Build + lint (activates and tears down cleanly).

### Phase 1 — text store + committed input
4. **`TextStore` object** — `#[implement(ITextStoreACP)]` with the mirror buffer + `LockState`.
5. **Lock protocol** — `RequestLock` + `has_read_lock`/`has_write_lock` (§6.2–6.3).
6. **Content methods** — `GetText`, `GetSelection`, `SetSelection`, `SetText`,
   `InsertTextAtSelection`, `QueryInsert`, `GetEndACP`, `GetActiveView`, `GetStatus` (non-transitory,
   §6.4), `AdviseSink`/`UnadviseSink`; stub the embedded/attr-query group (§6.5).
7. **`ImeField` + focus** — `CreateContext(store)` + `Push`; `SetFocus(doc_mgr)` / `SetFocus(dim_empty)`.
8. **Pull client FFI** — `NativeTextInputClient` + `window_set_text_input_client` /
   `window_clear_text_input_client` / `window_set_ime_enabled`; wire content methods to the client.
9. **Positioning** — `GetTextExt` (screen px + DPI), `GetScreenExt`, `GetWnd` (§8).
10. **Kotlin** — `TextInputClient` interface + `Window.kt` registration. Build + lint. (CJK commits
    through the store; TIP draws its own composition + candidates.)

### Phase 2 — self-drawn preedit + attributes
11. **Composition sinks** — `ITfContextOwnerCompositionSink` + `ITfTextEditSink`; advise the edit sink
    via `ITfSource::AdviseSink` (§7.1–7.2).
12. **Display attributes** — `read_underlines` (§7.3) with the three-`.Anonymous` VARIANT read and the
    `VT_I4` guard.
13. **Commit/preedit routing** — `setMarkedText` / `insertText` / `unmarkText` (§7.4); app-side
    `notify` (§7.5).
14. **Input scopes** — `ITfInputScope::GetInputScopes` (§9). Build + lint.

### Phase 3 — reconversion
15. **Reconversion command** — `ITfFnReconversion` via `GUID_SYSTEM_FUNCTIONPROVIDER` (§10) + a window
    downcall to trigger it. Build + lint.

---

## 14. Verification

**Required bar:** `./gradlew lint` passes (pre-push) and the crate builds.

**Recommended manual check** (TSF behavior is driven by the installed TIP and is not scriptable; a
real IME must be installed). On a focused text field:

1. Install a Japanese or Chinese IME. Register a `TextInputClient`, `setImeEnabled(true)`, and answer
   `caretRectForRange` with the field's caret rect.
2. Type. Phase 1: the TIP's composition and candidate windows appear at the caret; committing inserts
   through `insertText`. Phase 2: the preedit renders inline via `setMarkedText` with clause
   underlines from the display attributes; committing calls `insertText` + `unmarkText`.
3. Korean IME: type, commit, then trigger **Reconvert** on the committed text and confirm the
   candidate list reopens (the non-transitory payoff).
4. Focus a numeric field and confirm the TIP restricts input (input scope).
5. Move the window to a monitor at a different DPI and confirm the candidate window stays on the caret
   (screen-rect DPI conversion in `GetTextExt`).

**Known-hard parts to watch** (not defensive padding — these are where TSF work actually goes):
- `RefCell` borrows across a sink call panic. Drop every borrow before calling out.
- `GetTextExt` / `TS_E_NOLAYOUT` is per-TIP sensitive; return a best-effort rect rather than the error.
- No Rust project implements a TSF text store today — the reference implementations are C++/C#
  (Chromium, WPF). Cross-check behavior against them when a TIP misbehaves.

---

## 15. References

Verified against Microsoft Learn, Chromium `ui/base/ime/win/tsf_text_store.cc` / `tsf_bridge.cc`,
WPF `TextStore.cs` / `TextServicesContext.cs`, and the resolved `windows` 0.62.2 crate
(`…/UI/TextServices/mod.rs`).

- Text stores overview — https://learn.microsoft.com/windows/win32/tsf/text-stores
- `ITextStoreACP` — https://learn.microsoft.com/windows/win32/api/textstor/nn-textstor-itextstoreacp
- Document locks (grant protocol, reentrant upgrade, `TF_E_NOLOCK`) — https://learn.microsoft.com/windows/win32/tsf/document-locks
- `ITextStoreACP::GetTextExt` (screen coords; `TS_E_NOLAYOUT`) — https://learn.microsoft.com/windows/win32/api/textstor/nf-textstor-itextstoreacp-gettextext
- `ITfContextOwnerCompositionSink` — https://learn.microsoft.com/windows/win32/api/msctf/nn-msctf-itfcontextownercompositionsink
- `ITfTextEditSink::OnEndEdit` (read cookie) — https://learn.microsoft.com/windows/win32/api/msctf/nf-msctf-itftexteditsink-onendedit
- `ITfCompositionView::GetRange` / `ITfRangeACP::GetExtent` — https://learn.microsoft.com/windows/win32/api/msctf/nf-msctf-itfcompositionview-getrange · https://learn.microsoft.com/windows/win32/api/msctf/nf-msctf-itfrangeacp-getextent
- Using display attributes — https://learn.microsoft.com/windows/win32/tsf/using-display-attributes
- `ITfReadOnlyProperty::EnumRanges` / `GetValue` — https://learn.microsoft.com/windows/win32/api/msctf/nf-msctf-itfreadonlyproperty-enumranges
- `ITfDisplayAttributeMgr::GetDisplayAttributeInfo` / `TF_DISPLAYATTRIBUTE` — https://learn.microsoft.com/windows/win32/api/msctf/ns-msctf-tf_displayattribute
- `ITfThreadMgr2::ActivateEx` (flag meanings) — https://learn.microsoft.com/windows/win32/api/msctf/nf-msctf-itfthreadmgr2-activateex
- `ITfMessagePump` (wrapper, not required) — https://learn.microsoft.com/windows/win32/api/msctf/nn-msctf-itfmessagepump
- `ITfFnReconversion` — https://learn.microsoft.com/windows/win32/api/ctffunc/nn-ctffunc-itffnreconversion
- `ITfInputScope` — https://learn.microsoft.com/windows/win32/api/inputscope/nn-inputscope-itfinputscope
- Chromium text store — https://chromium.googlesource.com/chromium/src/+/main/ui/base/ime/win/tsf_text_store.cc · bridge — https://chromium.googlesource.com/chromium/src/+/main/ui/base/ime/win/tsf_bridge.cc
- WPF `TextStore.cs` — https://github.com/dotnet/wpf/blob/main/src/Microsoft.DotNet.Wpf/src/PresentationFramework/System/Windows/Documents/TextStore.cs · `TextServicesContext.cs` — https://github.com/dotnet/wpf/blob/main/src/Microsoft.DotNet.Wpf/src/PresentationCore/System/Windows/Input/TextServicesContext.cs
- macOS pull client (FFI pattern reference; separate type, not a unification target) — [`TextInputClient.kt`](../../../../kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/macos/TextInputClient.kt)
