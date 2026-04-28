# Agent orientation: `desktop-win32`

You are reading this because you've been pointed at the `native/desktop-win32` crate (or its Kotlin counterpart at `kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/`) and need to navigate, modify, or debug it. Read this file first; it indexes the rest.

## What this crate is

Windows backend of the kotlin-desktop-toolkit. Rust crate (cdylib) exposing a flat C ABI via `cbindgen`, consumed from Kotlin via JExtract-generated bindings plus hand-written wrappers. UI runs on a single thread (`OleInitialize` STA) with a classic `GetMessage` pump in `event_loop.rs`. Composition uses **WinRT `Windows.UI.Composition`** via `ICompositorDesktopInterop` — *not* DirectComposition. Rendering is ANGLE-on-D3D11.

## Start here

- **`ARCHITECTURE.md`** — module layout, FFI pipeline, threading and ownership models, error channel, headline data flows.
- **`FFI_CONVENTIONS.md`** — the `*_api.rs` ↔ Kotlin contract; pointer/array/option type zoo; `ffiDownCall` scoping rule; COM lifecycle.
- **`SUBSYSTEMS.md`** — per-subsystem reference. Look up the area you're touching.
- **`TODO.md`** — confirmed bugs, capability gaps, smells, open design questions. Cross-reference before claiming something is "broken" or "missing".

## Top things that surprise

If you only read one section before touching code, read this one.

0. **This is the Win32-first backend.** Default to Win32 APIs (`CreateWindowExW`, `GetMessageW`, `RegisterDragDrop`, `IFileOpenDialog`, …). Use WinRT only when there's a documented reason — there are exactly four such subsystems today, each justified in `ARCHITECTURE.md` → Scope. **Never propose WinUI 3 or Windows App SDK (`Microsoft.UI.*`, `Microsoft.WindowsAppSDK`) APIs in this crate.** A WinUI 3 backend, if built, lives in a separate crate.
1. **Composition is `Windows.UI.Composition` (WinRT), not DirectComposition.** They're distinct APIs with different lifetimes and threading. See `ARCHITECTURE.md` → Composition section.
2. **Single UI thread.** `OleInitialize` STA, `DispatcherQueue` with `DQTYPE_THREAD_CURRENT`, the wndproc, and most state assume one thread. Cross-thread work goes through `application_dispatcher_invoke`. Several pieces are `thread_local!` (key-message stash, exception store).
3. **Error channel is `anyhow::Result` through `ffi_boundary`, not return codes.** Rust functions return `anyhow::Result<T>`; `ffi_boundary` logs any `Err`, appends the message to thread-local `LAST_EXCEPTION_MSGS`, and returns `R::default()`. Kotlin's `ffiDownCall` polls after every call and throws `NativeError`. Panic catching inside `ffi_boundary` is a safety net for unexpected unwinds, not a designed error path — if Rust code panics, treat it as a bug. **Background-thread errors are lost** (thread-local store).
4. **`ffiDownCall { ... }` must wrap only the native call.** Not `Arena.use`, not `withPointer`, not helpers (which wrap their own native calls). Wider scopes conflate exception attribution. See `FFI_CONVENTIONS.md`.
5. **Window starts at `1×1` and is then resized.** Intentional: managed code uses *logical* pixels but the DPI scale only exists once an `HWND` exists (`GetDpiForWindow`). Consequence: `WM_SIZE` fires twice on every window birth. Handlers must be idempotent.
6. **Coordinates are mostly logical, but with deliberate physical-pixel exceptions.** Pointer events' `locationOnScreen`, several Window events, drag-drop callbacks, and `screen_map_to_client` carry `PhysicalPoint` / `PhysicalSize`. See `SUBSYSTEMS.md` → Geometry → Exceptions.
7. **`EnableMouseInPointer(true)` is process-wide and irreversible** (event_loop.rs:55). Anything in the same process expecting raw `WM_MOUSE*` will silently break.
8. **The `borrow` pattern on `RustAllocatedRawPtr`** (ffi_utils.rs:105-112) reconstructs and immediately leaks a `Box` per call to produce a `&R`. Sound under the toolkit's single-thread-of-ownership assumption; soundness is by convention. Currently under deferred review.

## Watch out for in code reviews / edits

- `tryRead*` currently swallows all errors to `None` — should be format-not-found only. Don't add new `tryRead*` callers assuming the broad swallow semantics.
- COM impls have **no** `// SAFETY:` comments anywhere (and `desktop-common::ffi_utils` has a module-level `#![allow(clippy::missing_safety_doc)]`). Add one when you touch an `unsafe` block.
- Most clipboard / drag-drop work assumes the OLE STA. Calls from another thread will deadlock or fail silently. There is no thread-affinity assertion at the FFI boundary.

## Working with the human

The high-impact conventions for collaborating on this crate:

- **Outline before executing** anything multi-file or API-shaping. Wait for confirmation before reaching for `Edit` / `Write`.
- **Confirm interpretation when offered A vs B.** When asked to choose between two approaches, explicitly state which one you're adopting and why; don't silently compose hybrids.
- **`ffiDownCall` scoping**: wrap only the native call (not `Arena.use`, not `withPointer`, not helpers).
- **Throwing helper naming**: `require*` / `check*` / `ensure*` for throwing preconditions; reserve `let` / `if*` / `takeIf*` for genuine no-op semantics.
- **Distinguish documented contract from observed implementation behaviour — never state behaviour observed in source code or experiment as if it were a documented guarantee.** Prefix inferred-from-behaviour claims with "implementation-defined" / "in practice" / "per the X implementation". If asked where a claim is documented and you can't cite the source, admit it was inferred — bluffing about the docs erodes trust and leads to design decisions built on false premises.
- **Use authoritative-docs lookup skills when your agent platform exposes them.** For Win32 / WinRT questions, Claude Code provides `microsoft-docs:microsoft-docs` (concept lookup) and `microsoft-docs:microsoft-code-reference` (API / SDK code verification). Prefer these over speculating from internal knowledge — especially when distinguishing documented contract from observed behaviour (above).
- **Don't add redundant explanatory clauses** on top of content that already conveys the same point. Trust the prose to do its job.
- **A wrong claim about current state may still be a valid design suggestion.** When you (or a reviewer) correct a factual error of yours, check whether the *wrong* version was nonetheless what the code *should* be. If so, voice both halves: "I was wrong that X is Y, but X should be Y, and here's why." Don't silently retract — your hallucination may have surfaced a real bug.

## When to update this doc set

- Subsystem refactor (e.g. another `_reader` cousin, a new `_api.rs`) → update `SUBSYSTEMS.md` and probably `ARCHITECTURE.md`.
- New FFI primitive in `desktop-common::ffi_utils` → update `FFI_CONVENTIONS.md`.
- Shipped fix for a `TODO.md` entry → remove the entry (don't leave stale ones).
- Found a new bug or capability gap → add to `TODO.md`.
- The "top surprises" above stop being accurate → update this file.

Don't let any of these docs claim something the code doesn't do. Before recommending an action that names a specific file, line, function, or flag, verify it still exists and behaves as described — docs decay even when carefully maintained.
