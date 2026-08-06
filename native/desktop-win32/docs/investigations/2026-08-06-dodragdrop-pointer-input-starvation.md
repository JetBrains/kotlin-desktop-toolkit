# Handover: `DoDragDrop` wedges the UI thread — pointer-input starvation hypothesis

**Branch:** `win-dnd-pointer-input-starvation` · **Written:** 2026-08-06 · **Target:** a native Windows
development machine, taking over from a VM where the decisive experiment could not be run.

This document is the investigation's memory. It separates what is **documented**, what is **observed**,
and what is **assumed**, because the investigation has already lost time to conflating the three — one
conclusion in it had to be retracted within the hour (§5.1).

---

## 0. How to use this branch

**Reproduce red before changing behaviour.** Everything added on this branch is inert unless an
environment variable is set, on purpose: with no variables set, this branch behaves exactly like `main`.
The only edit to a stock code path is that `start_drag_drop` now binds `DoDragDrop`'s `HRESULT` to a
local before applying `?`, which is semantically identical.

The intended order is:

1. get a **red** repro on the machine (§7, E1) — with no fix flags set,
2. **measure** it (§6 probes) — still no fix flags,
3. only then flip a fix flag (§7, E3/E4) and see whether red turns green.

Skipping to step 3 costs the ability to tell a fix from a coincidence: the wedge is a *rate*, not a
certainty (§1), so a single green run after a change proves nothing.

---

## 1. The symptom — two signatures, and only one of them is the target

| | **CI signature** (the actual bug) | **Local signature** (seen on the VM) |
|---|---|---|
| Where | `Fleet Tests [integration-tests] [Win] [x64]`, e.g. #7372, #7381 | Apple-Silicon VM running the same tests |
| OLE callbacks during the drag | **none at all** | healthy: `onQueryContinueDrag` at +22 ms, `onDragEnter` +38 ms, `onDragOver` +55 ms, then every ~64 ms out to +310 s |
| UI-thread CPU | 93–98% of a core — spinning, not parked | 20–98% |
| Messages retrieved on the UI thread | **zero**, up to 4.57 s into every wedge | zero |
| `Dispatchers.Main` | starved for the whole drag | starved (measured 37.9 s in one case) |
| Ends | `RETURNED effect=0`, always after `REVIVED BY escape` | never ends on its own |
| Rate | 6 of 7 file-tree drags wedged; 0 of 11 dock-tab drags in the same processes | intermittent |

**The CI signature has never been reproduced locally.** That is the single most important fact in this
document. The local hang is starved in the same way but keeps receiving callbacks, so it differs in the
one respect that would discriminate between the candidate mechanisms. Any local experiment must first
establish which signature it is looking at.

The wedge is **re-decided per drag**: a wedged drag is followed by healthy chatty drags in the same
process. So `0 of 11` is a rate, not immunity.

There is also a **circular wait on the test side**, observed and not hypothetical: the robot drives the
gesture from `Dispatchers.Main`, which is the thread `DoDragDrop` blocks, so the `mouseRelease` that
would end the drag can never be produced. In all 6 CI wedges the robot delivered 0 mouse moves during
the drag and never released the button; in all 12 healthy drags it delivered 9–262 and did release. The
button was down when `DoDragDrop` was entered in **18/18** drags, so "the gesture had already finished"
is not the explanation. This is a real test defect worth fixing on its own, but a user's mouse does not
depend on our dispatcher, so fixing it will change the rate, not settle the product question.

---

## 2. The leading hypothesis, link by link

> `ole32` is waiting for legacy mouse input that our window suppresses, so the drag can neither advance
> nor terminate.

Each link, with its status stated honestly:

**L1 — KDT puts the whole process into mouse-in-pointer mode.**
`event_loop.rs` calls `EnableMouseInPointer(true)` unconditionally in `EventLoop::new`. *Proven by
source.* The documentation states this can be called only once in the context of a process lifetime, so
it cannot be undone later in the run; `IsMouseInPointerEnabled` is the documented way to read the state
back. This crate's `docs/AGENTS.md` already lists the irreversibility as surprise #7.

**L2 — A pointer message the application reports as handled is never promoted to the legacy mouse
stream.** In `window_proc`, `handle_event` returns `Some(LRESULT(0))` when the Kotlin handler returns
`Stop`, and the tail is `match handled { Some(result) => result, None => DefWindowProcW(...) }` —
so a handled pointer message skips `DefWindowProcW` entirely. *Proven by source for the KDT half.* That
`DefWindowProc` is what performs the promotion is **documented behaviour** for the pointer-message
family, and it is the mechanism this crate's own caption-button spec relies on
(`docs/specs/2026-04-30-win32-caption-buttons-design.md`).

**L3 — `DoDragDrop` consumes the legacy mouse stream and terminates on `WM_LBUTTONUP`.** *Documented*,
verbatim, in the Remarks of `ConvertPrimaryPointerToMouseDrag`:

> "Apps that do handle pointer messages do not receive mouse messages, which can cause problems when
> calling APIs that expect mouse input, such as **DoDragDrop**."
>
> "**DoDragDrop** expects to be called while mouse input is active — typically in response to a
> **WM_LBUTTONDOWN** or **WM_MOUSEMOVE** while the button is held. It captures mouse input and processes
> drag-and-drop, exiting when it receives **WM_LBUTTONUP**."

Microsoft's remedy is that same function: call it *immediately before* `DoDragDrop` to switch the
in-flight primary contact to mouse input for the rest of the interaction.

**L4 — The Fleet client consumes pointer events during a drag; this repository's own sample does not.**
*Proven by source on both sides.* Fleet's `InputStateTracker.updateStateAndSendEvents` maps
`PointerUpdated` to `Stop` when `processResult.anyChangeConsumed`, and a recognised Compose drag gesture
consumes its move changes. `sample/.../win32/SkikoWindowWin32.kt` returns `EventHandlerResult.Continue`
from `PointerUpdated` unconditionally — and it starts its drag from exactly the same place, a
`PointerUpdated` with the left button held. This asymmetry is the best available explanation for why the
sample drags fine by hand while Fleet wedges, and it is what §7 E1 tests.

**L5 — Therefore `ole32` blocks waiting for input that never arrives.** **Not proven. This is the
experiment.**

### 2.1 The counter-evidence L5 has to survive

Do not present L1–L4 as a solved case; one observation cuts against the simple version of L5.

A **stationary manual drag** in the Fleet client keeps receiving `onQueryContinueDrag` callbacks 63–64 ms
apart, and the local wedge did the same for 310 s. So `DoDragDrop`'s loop advances on its own cadence
without any new mouse input — starvation of *movement* alone does not stop it, and "no callbacks" is
therefore not a straightforward consequence of "no mouse messages".

What the hypothesis still has to explain is the **CI-only absence of callbacks**. Two readings survive,
and the probes in §6 are built to separate them:

- the promotion state differs between CI and the VM (e.g. no legacy `WM_LBUTTONDOWN` ever reached the
  system, so `DoDragDrop` blocks *before* its loop begins rather than inside it), or
- something else entirely blocks on CI and the pointer question is a second, independent defect.

An earlier note in the ultimate-side handoff marked "consumed pointer messages starve `ole32`" as
*refuted* on the strength of that stationary-drag observation. **That was too strong** and has been
withdrawn: the observation shows `DoDragDrop` needs no movement to advance; it does not show the API is
receiving the input it documents itself as requiring.

---

## 3. Proven platform behaviour

Documented contract, and observations kept separate from it.

| Claim | Status | Source |
|---|---|---|
| `EnableMouseInPointer` can be called only once per process lifetime; later calls fail if the state differs | documented | `EnableMouseInPointer` reference |
| `IsMouseInPointerEnabled` reports the current state | documented | ditto |
| Apps handling pointer messages do not receive mouse messages, which breaks APIs expecting mouse input, `DoDragDrop` named explicitly | documented | `ConvertPrimaryPointerToMouseDrag` Remarks |
| `DoDragDrop` expects active mouse input, captures it, and exits on `WM_LBUTTONUP` | documented | ditto |
| `ConvertPrimaryPointerToMouseDrag` promotes the in-flight primary contact to mouse input; call after `WM_POINTERDOWN`, before `WM_POINTERUP`, with exactly one primary contact, immediately before `DoDragDrop` | documented | ditto |
| That function has **no SDK header and no import library**; Windows 11+; exported from `User32.dll` at ordinal 2811 | documented | ditto — and note the page carries a pre-release banner |
| `WH_CALLWNDPROC` is documented in terms of `SendMessage`; visibility of posted-message dispatch is **not** guaranteed | documented | `SetWindowsHookEx` reference |
| Thread messages (null `hwnd`) are discarded by `DispatchMessage` and eaten by every modal loop | documented / well-established | Win32 message reference; Chen |
| A running `DoDragDrop` modal loop coexists with a 37.9 s `Dispatchers.Main` starvation and **zero** message retrievals on that thread | **observed**, local, one wedge | ultimate-side hook counters |
| `ole32`'s loop advances on a ~64 ms cadence with no new input | **observed**, local and manual | callback timestamps |
| With `dragImage = null`, `start_drag_drop` is a `Box` allocation plus `DoDragDrop` and nothing else | **observed** — read off this crate's source; all four setup calls are inside one `if let Some(…)` block | `drag_drop.rs` |
| The data object is built on the Kotlin side and passed in as a raw COM pointer, so it is complete before the downcall; `IDataObject` is implemented in Rust over pre-pushed bytes, so OLE's format enumeration cannot re-enter the JVM | **observed** from source | `drag_drop_api.rs`, `data_object.rs` |
| The UI thread is a proper OLE STA (`OleInitialize(None)`) | **observed** from source | `application.rs` |

### 3.1 Measurement traps that already cost time

- **The legacy mouse band is empty by construction** under `EnableMouseInPointer(true)`. A control built
  on "did we see `WM_MOUSEFIRST..WM_MOUSELAST`?" measures a guaranteed zero and proves nothing. It is
  still the right thing to *count during a drag* (that is the payload, §6), just never as a liveness
  control.
- **Near-zero UI CPU plus a late first starvation report means the whole process was paused** (VM
  suspend), not this bug. A real wedge spins at 20–98% of a core.
- **Wall-clock timestamps in the VM logs are unusable.** One wedge is stamped `+310017 ms` at a wall time
  102 minutes after its `ENTER`. Both instruments use `System.nanoTime()` (QPC), which does not advance
  across host suspend, so the *durations* are genuine and the wall clock jumped. On a native machine this
  confound disappears — which is much of the point of moving.
- **Do not attach a debugger to the wedged JVM locally to get a stack.** Attaching freezes the JVM for
  ~45 s and manufactures the same symptom. Use a sampling profiler or ETW instead (§8).

---

## 4. What is *not* proven

1. **L5 itself** — that the missing legacy input is what blocks `ole32`.
2. **Which call blocks, and in which module.** Needs a native stack at module granularity (`ole32` vs
   `rpcrt4` vs `shell32`) from a genuinely wedged thread. Never obtained.
3. **Whether the CI and local signatures share a mechanism at all.**
4. **`pointerRetrievedTotal = 0` across an entire run.** The UI thread retrieved *no* `WM_POINTER*`
   messages in a whole test run, which is hard to square with a pointer-driven application whose robot
   drives real OS-level input (the button state was genuinely down: `keys=Left` for 310 s). Either the
   hook does not see what we think it sees, or the input path is not what we think it is. **Unexplained,
   and worth resolving early** — a wrong model of the input path invalidates several other readings.
5. **Whether a `WH_CALLWNDPROC` hook observes `DispatchMessage` at all** in this process. The control for
   this came back empty, so the instrument's capability is still unknown (and per §3 the documentation
   only promises `SendMessage`).

### 4.1 One retraction, recorded on purpose

Earlier in the investigation I reported, as established, that **the wedge is inside `DoDragDrop`'s
pre-loop initialisation**. That rested on zero message retrievals, zero callbacks, and 368/368 stack
samples inside the downcall. It is **withdrawn**: the local wedge `sfss9bdk6ed20ohh4p14` fired
`onQueryContinueDrag` at +22 ms and kept firing every ~64 ms for 310 s while the watchdog reported
`STARVED 37874 ms` with `pumpedDuringStall=0`. Zero retrievals and 368/368 downcall samples are both
fully compatible with a *running* modal loop. Only the CI-only absence of callbacks would still force the
pre-loop conclusion, and that leg has never been reproduced anywhere it can be inspected.

If you find the ultimate-side notes or a YouTrack draft asserting the pre-loop conclusion, they are stale.

---

## 5. Where the code involved lives

| Thing | Location |
|---|---|
| The blocking call | `native/desktop-win32/src/win32/drag_drop.rs` → `start_drag_drop` |
| FFI wrapper (adds nothing blocking) | `native/desktop-win32/src/win32/drag_drop_api.rs` |
| Process-wide pointer mode | `native/desktop-win32/src/win32/event_loop.rs` → `EventLoop::new` |
| Handled-message → skip `DefWindowProc` | same file, tail of `EventLoop::window_proc` |
| Drag started from a pointer update, in this repo | `sample/.../win32/SkikoWindowWin32.kt`, `Event.PointerUpdated` branch |
| Prior art on the promotion mechanism | `native/desktop-win32/docs/specs/2026-04-30-win32-caption-buttons-design.md` |
| Client-side consumption decision | ultimate: `fleet/noria/ui/srcJvmMain/androidx/compose/ui/desktop/windows/InputStateTracker.kt` |
| Client-side drag start | ultimate: `.../windows/WindowsWindow.kt` |
| Full VM-side evidence log | ultimate branch `air-dnd-hang-diagnostics`, `.investigation/win-dnd-hang-handoff.md` |

---

## 6. Probes added on this branch

All off by default. `native/desktop-win32/src/win32/drag_drop_probe.rs` holds the Rust side.

| Variable | What it does | Why it exists |
|---|---|---|
| `KDT_WIN32_DND_PROBE=1` | logs input state before and after `DoDragDrop` (`IsMouseInPointerEnabled`, `GetKeyState(VK_LBUTTON)`, `GetCapture`, `GetQueueStatus`, thread id), logs every mouse-band and pointer-band message the window receives **while a drag is in flight**, and on return logs the `HRESULT`, the effect, and both band counts | measurement. `mouseBandDuringDrag` is the number the hypothesis lives or dies by |
| `KDT_WIN32_DND_CONVERT_POINTER_TO_MOUSE=1` | calls `ConvertPrimaryPointerToMouseDrag` immediately before `DoDragDrop`, resolving it from `user32.dll` by name and then by ordinal 2811, logging which route worked and what it returned | candidate fix, exactly as documented |
| `KDT_WIN32_NO_MOUSE_IN_POINTER=1` | skips `EnableMouseInPointer(true)` in `EventLoop::new` | isolation only — see the caveat below |
| `KDT_SAMPLE_CONSUME_POINTER_EVENTS=1` | makes the win32 sample report every pointer event as handled, the way a Compose scene does | **the repro lever** (§7 E1) |

Grep the logs for `DND_PROBE`.

**Caveat on `KDT_WIN32_NO_MOUSE_IN_POINTER`.** On its own it makes the window input-blind: the wndproc
only handles the `WM_POINTER*` band, so with the flag set nothing reacts to the mouse and no drag can be
started by hand. It is useful for confirming the once-per-process semantics and for watching the raw
message stream, but as a *fix* it is only half a change — the other half is wiring the legacy `WM_MOUSE*`
band into the event pipeline, which is a feature-sized piece of work and deliberately not attempted here.
`ConvertPrimaryPointerToMouseDrag` tests the same hypothesis without any input rework, which is why it is
ordered first.

**Caveat on the `ConvertPrimaryPointerToMouseDrag` binding.** It compiles and is wired, but it has
**never been executed** — no drag has run on this branch yet. Its documentation page carries a
pre-release banner, lists no header and no import library, and requires Windows 11. If neither
resolution route finds it, the probe logs a warning and does nothing. First run should confirm which
route resolved and what the call returned before any conclusion is drawn from it.

---

## 7. Experiments, in order

### E0 — Baseline, no flags
Run the win32 sample and drag by hand:

```
./gradlew :sample:runSkikoSampleWin32
```

Log: `sample/build/sample-logs/skiko_sample.log`. Drag from the client area with the left button held —
the sample starts a drag from `PointerUpdated` whenever the left button is down outside the caption and
the toy buttons. Expected: drags work. This establishes that the machine, the build and the tooling are
sound before anything interesting is attempted.

### E1 — Reproduce red in the sample (the important one)
```
set KDT_SAMPLE_CONSUME_POINTER_EVENTS=1
set KDT_WIN32_DND_PROBE=1
./gradlew :sample:runSkikoSampleWin32
```
The sample now differs from stock only in reporting pointer events as handled — L4's asymmetry, and
nothing else. Then drag.

- **Wedges** → the bug is reproduced with no Fleet, no Compose, no JVM test framework, no robot. That is
  the goal of this handover, and everything afterwards becomes cheap. Expect
  `mouseBandDuringDrag=0` in the `DND_PROBE` line.
- **Does not wedge** → L4 is not sufficient, and the next question is what else differs. Check
  `mouseBandDuringDrag` anyway: if it is `0` and the drag still worked, L3's practical force is weaker
  than its wording suggests and L5 is in trouble. Record the number either way.

Vary: with and without a drag image; `DragDropEffect.Copy` versus MOVE-only (a CI wedge occurred on a tab
drag with `allowedEffects=2` and no `CF_HDROP`, so neither the effect mask nor file formats are necessary
conditions); stationary versus moving; releasing the button versus not.

### E2 — Measure, before fixing
With `KDT_WIN32_DND_PROBE=1` on a red run, answer:

1. `mouseBandDuringDrag` — zero or not?
2. `pointerBandDuringDrag` — is `DoDragDrop`'s modal loop dispatching anything to us at all?
3. `lButtonState` immediately before `DoDragDrop` — did the legacy stream ever see the button go down?
4. Do OLE callbacks fire? This is what tells you whether you have the CI signature or the local one.

Cross-check the message stream with **Spy++** on the sample's window (§8): it needs no code and it is the
most direct answer to "does any `WM_MOUSE*` arrive during the drag".

### E3 — The documented fix
```
set KDT_WIN32_DND_CONVERT_POINTER_TO_MOUSE=1
```
on the E1 configuration. If red turns green and `mouseBandDuringDrag` goes from `0` to non-zero, the
hypothesis is confirmed and the fix is the one Microsoft prescribes. Note the Windows 11 floor: the
production fix needs a fallback path for Windows 10, and "what should happen on Windows 10" then becomes
a real design question rather than an afterthought.

### E4 — Only if E3 is inconclusive
`KDT_WIN32_NO_MOUSE_IN_POINTER=1`, understanding that it makes the window input-blind (§6). Its honest
use is to observe the unmodified legacy stream with Spy++ / ETW, not to demonstrate a fix.

### E5 — In parallel, cheap and independent
Resolve the `pointerRetrievedTotal = 0` anomaly (§4.4) on this machine. It costs one instrumented run and
it either restores or destroys confidence in a whole class of earlier measurements.

---

## 8. Machine setup

**Build prerequisites** — a Rust toolchain (this branch was checked with cargo/rustc 1.96.0), a JDK 25
toolchain for the sample module, and MSVC build tools. `cargo check -p desktop-win32`,
`cargo clippy -p desktop-win32 --all-targets` and `./gradlew :sample:compileKotlin` all pass on this
branch as committed.

**Instruments worth having installed before starting**, roughly in order of value here:

- **Spy++** (ships with Visual Studio) — message log on the target window and thread. This is the direct
  observation the whole hypothesis wants, and it requires no code changes.
- **Process Explorer** — per-thread CPU. Distinguishes "spinning" (the real wedge, 20–98% of a core) from
  "parked" and from "the whole process was suspended".
- **WinDbg** — native stacks at module granularity, for §4.2. Attach to the *sample*, not to a wedged
  Fleet JVM, for the reason in §3.1.
- **ETW / WPR** — if a non-invasive stack of the wedged thread is needed while the process keeps running.

**Getting Fleet's side of it**, if the sample cannot be made to reproduce: the ultimate branch
`air-dnd-hang-diagnostics` carries the client-side tracing (`DND_TRACE`-prefixed, env-gated) plus
`.investigation/win-dnd-hang-handoff.md`. All of that instrumentation must be removed before any merge —
it is diagnostic scaffolding, including changes to the test framework core.

---

## 9. Closed — do not re-test

- **Drag-image setup.** The hang reproduces with `dragImage = null`, and this crate's source shows that
  exonerates the *whole* setup block, not just part of it.
- **Data-object construction and OLE format enumeration.** Built Kotlin-side and passed in as a raw COM
  pointer; `IDataObject` is Rust-side over pre-pushed bytes, so it cannot re-enter the JVM.
- **`CO_E_NOTINITIALIZED` / wrong apartment.** The thread is a proper STA.
- **`CF_HDROP` / the effect mask as necessary conditions.** A wedge occurred on a tab drag with
  `allowedEffects=2` and custom items only.
- **Moving `DoDragDrop` to a dedicated thread.** Chromium shipped that and deleted it again after it
  produced new hangs. Do not propose it.
- **AIR-5786 (Rework Dispatcher in KDT Windows)** as the explanation. Worth doing, but the drag is entered
  from the *window* wndproc, so `pollCallbacks` is not on the blocked stack and no callback batch is being
  held. Related, not the same.

---

## 10. If E1 reproduces, what to write down

The point of a red sample is that it makes the remaining questions cheap. In that order:

1. the exact `DND_PROBE` lines for a wedged and a healthy drag, side by side;
2. a WinDbg stack of the wedged thread, at module granularity;
3. whether E3 fixes it, and what it does on Windows 10;
4. then, and only then, a design proposal for the crate — and per `docs/AGENTS.md`, outline it before
   writing it.
