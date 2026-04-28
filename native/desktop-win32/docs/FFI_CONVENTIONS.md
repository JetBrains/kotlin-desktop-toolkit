# FFI conventions

Reference for the patterns that bridge Rust ↔ Kotlin in this crate. Read alongside `ARCHITECTURE.md` (which covers the why and the data flows) and `SUBSYSTEMS.md` (which applies these patterns subsystem-by-subsystem).

## File-pair convention

| File | Role | Visibility |
|---|---|---|
| `xxx.rs` | Implementation. Plain Rust types, helpers, business logic. | Items used only inside the crate stay `pub(crate)` (or unmarked); items consumed by the matching `_api.rs` are `pub`. |
| `xxx_api.rs` | FFI surface. Every function `#[unsafe(no_mangle)] pub extern "C" fn`. Every body wrapped in `ffi_boundary("name", \|\| { ... })`. | Implements the cbindgen contract; nothing else should call these directly from Rust. |
| `kotlin-desktop-toolkit/.../win32/Xxx.kt` | Kotlin wrapper. Public types live here; FFI calls go through `ffiDownCall`. | All FFI helpers stay `internal`. Public surface is the toolkit user's API. |

There is no `pointer_api.rs` — pointer events are push-only via the `Event` enum in `events.rs`.

## cbindgen

Driven by `native/desktop-win32/cbindgen.toml`:

```toml
language = "C"
[macro_expansion]   bitflags = true
[parse]             parse_deps = true; include = ["desktop-win32", "desktop-common"]
[enum]              prefix_with_name = true       # NativeLogLevel::Off → NativeLogLevel_Off
[export]            prefix = "Native"
```

Consequences:
- Every exported type appears in the generated header as `Native<RustName>`. Enum variants are doubly prefixed: `NativeLogLevel_Off`.
- `desktop-common` types (`AutoDropArray<T>`, `BorrowedStrPtr`, `RustAllocatedStrPtr`, `FfiOption<T>`, `BorrowedArray<T>`, `BorrowedUtf8`, etc.) are inlined into the Win32 header — they are **not** re-exported through Rust `pub use`. cbindgen walks the dependency directly via `parse_deps`.
- Items annotated `/// cbindgen:ignore` are excluded. Used for the `DLL_HINSTANCE` static, `DllMain`, and Rust-side registries (`DATA_OBJECT_REGISTRY`, `DATA_OBJECT_NEXT_ID`).

Generic type instantiations (e.g. `AutoDropArray<RustAllocatedStrPtr>`, `FfiOption<AutoDropByteArray>`) appear in the header as monomorphised types: `NativeAutoDropArray_RustAllocatedStrPtr`, `NativeFfiOption_AutoDropByteArray`. JExtract turns them into matching Java layout classes.

## The `ffi_utils` zoo

All pointer / array / option wrappers live in `desktop-common::ffi_utils`. Pick the right one when designing a new FFI signature.

### Strings

| Type | Direction | Allocator | Lifetime | Drop |
|---|---|---|---|---|
| `BorrowedStrPtr<'a>` | Kotlin → Rust | Kotlin (`Arena.ofConfined`) | Scoped to the FFI call | None — Arena closes |
| `RustAllocatedStrPtr` | Rust → Kotlin | Rust (`CString::into_raw`) | `'static` | Kotlin must call `native_string_drop` (Strings.kt does this in `finally`) |
| `AutoDropStrPtr` | Rust-internal RAII | Rust | scope-bound | `Drop` impl calls `deallocate()` |
| `BorrowedUtf8<'a>` | Kotlin → Rust, length-delimited | Kotlin | Scoped | None |

Encoding at the boundary is **always UTF-8, NUL-terminated** for `*StrPtr` types. UTF-16 (`HSTRING`) is used only inside Win32 calls — see `strings.rs` (`copy_from_utf8_string`, `copy_from_wide_string`).

Mixing `BorrowedStrPtr` (NUL-terminated) and `BorrowedUtf8` (length-delimited) silently misreads memory. Choose one and stick with it per parameter.

### Arrays

| Type | Direction | Allocator | Lifetime | Drop |
|---|---|---|---|---|
| `AutoDropArray<T>` (`{ ptr: *const T, len: usize }`) | Rust → Kotlin | Rust (`Box::leak` on `Box<[T]>`) | until explicit drop | Kotlin calls a typed `native_*_drop` (no drop fn embedded in the struct) |
| `BorrowedArray<'a, T>` (`{ ptr, len, _phantom }`) | Kotlin → Rust | Kotlin Arena | scoped | None |

`AutoDropArray<T>::Drop` reconstructs `Box<[T]>` from raw parts and lets it drop, which recursively drops each `T`. So `AutoDropArray<AutoDropStrPtr>` frees the array allocation and every string. The matching Kotlin helper (`Strings.kt::listOfStringsFromNative`) wraps the read in `try` and the drop-call in `finally { ffiDownCall { native_string_array_drop(seg) } }`.

### Optionals

`FfiOption<T: PanicDefault>` = `{ is_some: bool, value: T }`. The `value` slot always holds a valid `T` (`T::default()` when `is_some == false`). Used to return nullable strings / arrays / structs without a separate sentinel.

The `IntoFfiOption` trait (data_object_api.rs:32-44) converts `anyhow::Result<T>` → `anyhow::Result<FfiOption<T>>`. **Current scope is too wide** — it swallows every error to `None` (with a `trace!` log only). It should swallow only "format not found" (`DV_E_FORMATETC` / `DV_E_TYMED`); allocation failures and type mismatches should surface as exceptions. See `TODO.md`.

### Opaque pointers (Rust-allocated objects)

| Type | Backing | Created by | Dropped by |
|---|---|---|---|
| `RustAllocatedRawPtr<'a>` | `Box<T>` | `from_value(T)` → `Box::into_raw` | `to_owned::<T>()` returns the `Box`; let it drop |
| `RustAllocatedRcPtr<'a>` | `Rc<T>` | `from_rc(Rc<T>)` → `Rc::into_raw` | `to_rc::<T>()` returns the `Rc`; let it drop |
| `BorrowedOpaquePtr<'a>` | borrowed `&T` | wrap an existing `&T` | None — caller still owns |
| `ComObject<T>` (windows-core) | COM refcount on an `implement!`-decorated struct | `ComObject::new(T)` | Refcount → 0 (i.e. last `Release`) |
| `ComInterfaceRawPtr` (com.rs) | `*mut c_void` carrying an `IUnknown` strong ref | `from_object(ComObject)` → `cast::<IUnknown>().into_raw()` | `Drop` calls `from_raw` → `IUnknown::Release` |

The wrappers don't carry the inner Rust type — the type is supplied at the use site via turbofish (`borrow::<Application>()`, `to_rc::<Window>()`, etc.). To make call sites readable, each subsystem defines a thin convenience alias in its `_api.rs`, e.g.:

```rust
// application_api.rs:9
pub type AppPtr<'a> = RustAllocatedRawPtr<'a>;

// window_api.rs:23
pub type WindowPtr<'a> = RustAllocatedRcPtr<'a>;
```

These aliases are purely for naming — they don't bind the inner type, change the ABI, or alter ownership semantics. `AppPtr` and `WindowPtr` are interchangeable with their underlying `RustAllocated*Ptr` everywhere they appear; the alias just signals intent at the FFI-call site. `ComInterfaceRawPtr` is *not* an alias — it's a distinct struct in `com.rs` with its own `Drop` impl that releases the COM ref.

The `borrow::<R>()` / `borrow_mut::<R>()` methods on `RustAllocatedRawPtr` produce a `&R` / `&mut R` from the raw pointer **without consuming the box**. They do this by `Box::leak(self.to_owned())` (ffi_utils.rs:105-112) — i.e. reconstruct the `Box` and immediately leak it. Soundness depends on caller discipline (no concurrent `to_owned`, single thread of ownership). **Marked open for review (see `TODO.md`).**

Asymmetric pair: `RcPtr` has only `to_rc` (consumes) and `borrow` via cloning the `Rc` semantically; `RawPtr` separates `to_owned` (consume) from `borrow` / `borrow_mut` (non-consuming view).

### `PanicDefault`

Trait at `desktop-common::logger::PanicDefault`. Required for any type that crosses the FFI boundary as a return value. Defines the value returned by `ffi_boundary` when the closure returns an `Err` (or, as a safety net, when it panics unexpectedly). For most types this is just `Self::default()`; for `FfiOption<T>` it is `FfiOption::none()`; for opaque pointers it is the equivalent null/empty value.

If you add a new FFI return type, you must impl `PanicDefault` for it. cbindgen has no opinion on this — the trait is purely for `ffi_boundary`'s error-fallback path.

## The Rust ↔ Kotlin error channel

The designed path: `Err(anyhow_err)` → log + thread-local store → Kotlin throws. Panic catching is a safety net for unexpected unwinds (see `ARCHITECTURE.md` → Error handling model).

```
Rust side:                                      Kotlin side:
┌──────────────────────┐                        ┌────────────────────────┐
│ ffi_boundary(closure)│                        │ ffiDownCall { native() │
│   on Err (designed): │  ←── exception_msg ─┐  │   } → checkExceptions()│
│     log + append to  │                     │  │   → if non-empty:      │
│     LAST_EXCEPTION_  │                     │  │       clearExceptions()│
│     MSGS (thread-    │                     │  │       throw NativeError│
│     local, cap 10)   │                     │  └────────────────────────┘
│   on panic (safety   │                     │
│     net): same path  │                     │
│   return R::default()│  via thread-local ──┘
└──────────────────────┘
```

Conventions:
- `ffi_boundary("name", || { … })` is the **only** wrapper for `extern "C"` bodies. Inside it you may use `?` freely on `anyhow::Result`.
- `AssertUnwindSafe` is applied unconditionally inside `ffi_boundary`. Code that mutates shared state through an unexpected panic has no protection — this is a known limitation of the safety net, not a per-callsite decision.
- Background-thread errors are **silently lost**. `LAST_EXCEPTION_MSGS` is thread-local; only the calling thread's `ffiDownCall` will see them. If you spawn a worker thread inside an FFI call, install your own logging (or join and propagate).
- `NativeError` is a subclass of `java.lang.Error`, not `Exception` — Kotlin `try { … } catch (e: Exception)` will not catch it. This is intentional: native errors are typically unrecoverable.

## `ffiDownCall` scoping rule

`ffiDownCall { ... }` must wrap **only the native FFI call** — the line that crosses the JExtract boundary. Specifically, it must NOT wrap:

- `Arena.ofConfined().use { ... }` — arena setup is not a native call
- `someObject.withPointer { ... }` — pointer-accessor helpers are inline lookups
- Any post-processing of the returned struct (field reads, conversions)
- Any helper call that itself wraps `ffiDownCall` (e.g. `stringFromNative`, `listOfStringsFromNative`) — double-wrapping would conflate exception attribution

Drop calls in `finally` blocks (`native_*_drop`) each get their own narrow `ffiDownCall`. If a helper already drops internally, callers must NOT additionally wrap the helper.

The reason wider scopes are wrong: they obscure which line actually crosses the FFI boundary, and they conflate exception attribution when a nested helper's own native call (e.g. a drop) errors — the surrounding `ffiDownCall` would attribute it to the wrong operation.

The lexical nesting of `withPointer` / `Arena.use` / `ffiDownCall` is not uniformly standardised across the codebase. The rule is "ffiDownCall narrows to the native call"; the concrete template is whatever makes that scope unambiguous in the surrounding code.

## `ffiUpCall` (Kotlin callbacks invoked from Rust)

```kotlin
internal fun <T> ffiUpCall(defaultResult: T, body: () -> T): T
```

Wraps every Kotlin lambda that Rust may invoke through a function pointer (event handler, drop-target callbacks, dispatcher trampoline). Catches every `Throwable`, logs it, and returns `defaultResult`. **Kotlin exceptions never propagate into Rust.** If you need an "error" signal back into Rust, design it explicitly as a return value (e.g. `EventHandlerResult.Stop`).

Allocate the upcall stub in an `Arena.ofShared()` whose lifetime matches the Rust object that will hold the function pointer (e.g. `Application.applicationCallbacks`, `DragDropManager.dropTargetCallbacks`). Confined arenas would close before the Rust side calls the stub.

## Kotlin lifecycle conventions

| Concept | Implementation |
|---|---|
| Opaque Rust object on Kotlin side | `class X(internal val ptr: MemorySegment) : AutoCloseable` |
| Drop on close | `override fun close() { if (ptr != MemorySegment.NULL) ffiDownCall { x_drop(ptr) }; ptr = MemorySegment.NULL }` (where `ptr` is `var`) |
| Pass the raw segment to a callee | `internal inline fun <R> withPointer(block: (MemorySegment) -> R): R = block(ptr)` (defined per class, e.g. Window.kt:69) |
| Use-after-close guard | `private inline fun <R> requireOpen(block: (MemorySegment) -> R): R` — throws `IllegalStateException` if `ptr == NULL`, otherwise invokes `block` with the captured pointer (DataObject.kt:121-125 prototype). The lambda-taking shape captures the pointer **once**, avoiding TOCTOU between the null-check and the use; prefer it over the two-step `val captured = requireOpen(); use(captured)` form. |
| Block-scoped builder | `companion object { fun build(block: Builder.() -> Unit): X = … }` — used by `DataObject.build { … }` to hide the multi-step create / configure / convert flow |
| Pre-init deferred work | `Application.onStartup(action)` — queues into a `ConcurrentLinkedQueue` until first `runEventLoop` dispatch tick; subsequent calls go through `invokeOnDispatcher` |

Throwing-helper naming: prefer `require*` / `check*` / `ensure*` for inline preconditions (they throw on failure). Reserve `let` / `if*` / `takeIf*` / `letIf*` for genuine no-op semantics.

## COM lifecycle conventions

The Win32 OLE / drag-drop / clipboard subsystems use `windows-core` to implement COM interfaces. Patterns:

| Step | API |
|---|---|
| Define a COM impl | `#[implement(IDataObject)] struct DataObject { … }`; impl block uses `impl IDataObject_Impl for DataObject_Impl { … }` |
| Allocate one | `let obj: ComObject<DataObject> = ComObject::new(DataObject::new());` — refcount = 1 |
| Hand off to the OS or another COM caller | `let raw = ComInterfaceRawPtr::from_object(obj);` — internally `cast::<IUnknown>().into_raw()`, refcount stays 1 |
| Release | `Drop for ComInterfaceRawPtr` calls `IUnknown::from_raw(self.0).Release()` — refcount → 0 → struct drops |
| Borrow back into a typed interface | `let iface: IDataObject = unsafe { raw.borrow() };` — does not change refcount |

Apartment requirement: **STA**. `OleInitialize(None)` is called by `Application::init_apartment` (application.rs:31). Any drag-drop or OLE clipboard operation must run on the same STA thread.

`STGMEDIUM` ownership rules (per Win32 docs):
- `pUnkForRelease == None` → caller owns the medium and must call `ReleaseStgMedium` (which dispatches based on `tymed`) or free the medium directly.
- `pUnkForRelease == Some(unknown)` → `ReleaseStgMedium` releases the unknown.
- `StgMediumGuard::drop` (data_reader.rs:23-25) calls `ReleaseStgMedium` unconditionally — correct in both cases per the rules above.

## Decision flow for a new FFI function

1. **Rust impl** lives in `xxx.rs`, takes plain Rust types, returns `anyhow::Result<T>`.
2. **`xxx_api.rs`** declares `#[unsafe(no_mangle)] pub extern "C" fn xxx_do_thing(...) -> R` and wraps the body in `ffi_boundary("xxx_do_thing", || { ... })`. `R` must impl `PanicDefault`.
3. **Parameter types**: pick from the `ffi_utils` zoo. Strings → `BorrowedStrPtr`. Arrays → `BorrowedArray<T>`. Opaque receivers → `WindowPtr` / `AppPtr` / `ComInterfaceRawPtr`. Owned out-params → `RustAllocated*` / `AutoDrop*` / `FfiOption<…>`.
4. **Drop pairing**: for any owned-out value, ensure a matching `*_drop` exists. If using `AutoDropArray<T>`, the matching drop must be the typed one (`native_byte_array_drop`, `native_string_array_drop`, etc.).
5. **Kotlin wrapper** in `Xxx.kt`: call the JExtract-generated function inside `ffiDownCall`. An `Arena` is needed only when the JExtract glue requires one — e.g. when the function returns a struct by value (JExtract takes the arena as the first parameter to allocate the return slot) or when you need to allocate input structs / `Borrowed*` strings. Functions that take and return only primitives, opaque pointers, or pre-existing segments don't need an arena. For nullable returns use `optional*FromNative` helpers in `Strings.kt` / `Arrays.kt`. Drop calls live in `finally`.
6. **Callbacks**: every Kotlin lambda crossing into Rust goes through `ffiUpCall` with a sensible default. Allocate the stub in `Arena.ofShared()` whose lifetime matches the holding Rust object.
