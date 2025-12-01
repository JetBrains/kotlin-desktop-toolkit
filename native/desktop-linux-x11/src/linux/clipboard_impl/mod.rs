/*
SPDX-License-Identifier: Apache-2.0 OR MIT

Copyright 2022 The Arboard contributors

The project to which this file belongs is licensed under either of
the Apache 2.0 or the MIT license at the licensee's choice. The terms
and conditions of the chosen license apply to this file.
*/
#![warn(unreachable_pub)]

mod common;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

pub use common::Error;
use common::FormatData;

mod platform;
mod x11;

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "android", target_os = "emscripten")),
))]
pub use platform::{ClearExtLinux, GetExtLinux, LinuxClipboardKind, SetExtLinux};

#[cfg(windows)]
pub use platform::SetExtWindows;

#[cfg(target_os = "macos")]
pub use platform::SetExtApple;

/// The OS independent struct for accessing the clipboard.
///
/// Any number of `Clipboard` instances are allowed to exist at a single point in time. Note however
/// that all `Clipboard`s must be 'dropped' before the program exits. In most scenarios this happens
/// automatically but there are frameworks (for example, `winit`) that take over the execution
/// and where the objects don't get dropped when the application exits. In these cases you have to
/// make sure the object is dropped by taking ownership of it in a confined scope when detecting
/// that your application is about to quit.
///
/// It is also valid to have these multiple `Clipboards` on separate threads at once but note that
/// executing multiple clipboard operations in parallel might fail with a `ClipboardOccupied` error.
///
/// # Platform-specific behavior
///
/// `arboard` does its best to abstract over different platforms, but sometimes the platform-specific
/// behavior leaks through unsolvably. These differences, depending on which platforms are being targeted,
/// may affect your app's clipboard architecture (ex, opening and closing a [`Clipboard`] every time
/// or keeping one open in some application/global state).
///
/// ## Linux
///
/// Using either Wayland and X11, the clipboard and its content is "hosted" inside of the application
/// that last put data onto it. This means that when the last `Clipboard` instance is dropped, the contents
/// may become unavailable to other apps. See [SetExtLinux] for more details.
///
/// ## Windows
///
/// The clipboard on Windows is a global object, which may only be opened on one thread at once.
/// This means that `arboard` only truly opens the clipboard during each operation to prevent
/// multiple `Clipboard`s from existing at once.
///
/// This means that attempting operations in parallel has a high likelihood to return an error or
/// deadlock. As such, it is recommended to avoid creating/operating clipboard objects on >1 thread.
#[allow(rustdoc::broken_intra_doc_links)]
pub struct Clipboard {
    pub(crate) platform: platform::Clipboard,
}

impl Clipboard {
    /// Creates an instance of the clipboard.
    ///
    /// # Errors
    ///
    /// On some platforms or desktop environments, an error can be returned if clipboards are not
    /// supported. This may be retried.
    pub fn new() -> Result<Self, Error> {
        Ok(Clipboard { platform: platform::Clipboard::new()? })
    }

    /// Fetches UTF-8 text from the clipboard and returns it.
    ///
    /// # Errors
    ///
    /// Returns error if clipboard is empty or contents are not UTF-8 text.
    pub fn get_text(&mut self) -> Result<String, Error> {
        self.get().text()
    }

    /// Places the text onto the clipboard. Any valid UTF-8 string is accepted.
    ///
    /// # Errors
    ///
    /// Returns error if `text` failed to be stored on the clipboard.
    pub fn set_text<'a, T: Into<Cow<'a, str>>>(&mut self, text: T) -> Result<(), Error> {
        self.set().text(text).commit()
    }

    /// Places the HTML as well as a plain-text alternative onto the clipboard.
    ///
    /// Any valid UTF-8 string is accepted.
    ///
    /// # Errors
    ///
    /// Returns error if both `html` and `alt_text` failed to be stored on the clipboard.
    pub fn set_html<'a, T: Into<Cow<'a, str>>>(
        &mut self,
        html: T,
        alt_text: Option<T>,
    ) -> Result<(), Error> {
        self.set().html(html, alt_text).commit()
    }

    /// Clears any contents that may be present from the platform's default clipboard,
    /// regardless of the format of the data.
    ///
    /// # Errors
    ///
    /// Returns error on Windows or Linux if clipboard cannot be cleared.
    pub fn clear(&mut self) -> Result<(), Error> {
        self.clear_with().default()
    }

    /// Begins a "clear" option to remove data from the clipboard.
    pub fn clear_with(&mut self) -> Clear<'_> {
        Clear { platform: platform::Clear::new(&mut self.platform) }
    }

    /// Begins a "get" operation to retrieve data from the clipboard.
    pub fn get(&mut self) -> Get<'_> {
        Get { platform: platform::Get::new(&mut self.platform) }
    }

    /// Begins a "set" operation to set the clipboard's contents.
    pub fn set(&mut self) -> Set<'_> {
        Set {
            platform: platform::Set::new(&mut self.platform),
            pending_formats: Vec::new(),
        }
    }
}

/// A builder for an operation that gets a value from the clipboard.
#[must_use]
pub struct Get<'clipboard> {
    pub(crate) platform: platform::Get<'clipboard>,
}

impl Get<'_> {
    /// Completes the "get" operation by fetching UTF-8 text from the clipboard.
    pub fn text(self) -> Result<String, Error> {
        self.platform.text()
    }

    /// Completes the "get" operation by fetching HTML from the clipboard.
    pub fn html(self) -> Result<String, Error> {
        self.platform.html()
    }

    /// Completes the "get" operation by fetching a list of file paths from the clipboard.
    pub fn file_list(self) -> Result<Vec<PathBuf>, Error> {
        self.platform.file_list()
    }
}

/// A builder for an operation that sets a value to the clipboard.
#[must_use]
pub struct Set<'clipboard> {
    pub(crate) platform: platform::Set<'clipboard>,
    pub(crate) pending_formats: Vec<FormatData>,
}

impl Set<'_> {
    /// Adds text to the clipboard. Can be chained with other format methods.
    /// Call `commit()` to finalize the operation.
    ///
    /// # Example
    /// ```no_run
    /// # use arboard::Clipboard;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut clipboard = Clipboard::new()?;
    /// clipboard.set()
    ///     .text("plain text")
    ///     .html("<b>bold text</b>", None)
    ///     .commit()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn text<'a, T: Into<Cow<'a, str>>>(mut self, text: T) -> Self {
        let text = text.into().into_owned();
        self.pending_formats.push(FormatData::Text(text));
        self
    }

    /// Adds HTML (with optional plain-text alternative) to the clipboard.
    /// Can be chained with other format methods. Call `commit()` to finalize.
    pub fn html<'a, T: Into<Cow<'a, str>>>(
        mut self,
        html: T,
        alt_text: Option<T>,
    ) -> Self {
        let html = html.into().into_owned();
        let alt_text = alt_text.map(|t| t.into().into_owned());
        self.pending_formats.push(FormatData::Html { html, alt_text });
        self
    }

    /// Adds a list of file paths to the clipboard. Can be chained with other format methods.
    /// Call `commit()` to finalize.
    pub fn file_list(mut self, file_list: &[impl AsRef<Path>]) -> Self {
        let paths: Vec<PathBuf> = file_list.iter().map(|p| p.as_ref().to_path_buf()).collect();
        self.pending_formats.push(FormatData::FileList(paths));
        self
    }

    /// Adds a custom format to the clipboard with a MIME type identifier.
    /// Can be chained with other format methods. Call `commit()` to finalize.
    ///
    /// # Example
    /// ```no_run
    /// # use arboard::Clipboard;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut clipboard = Clipboard::new()?;
    /// clipboard.set()
    ///     .text("fallback text")
    ///     .custom_format("application/json", br#"{"key":"value"}"#.to_vec())
    ///     .commit()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn custom_format(mut self, mime_type: &str, data: Vec<u8>) -> Self {
        self.pending_formats.push(FormatData::Custom {
            mime_type: mime_type.to_string(),
            data,
        });
        self
    }

    /// Commits all added formats to the clipboard.
    ///
    /// This method finalizes the builder and writes all formats that were
    /// added via `text()`, `html()`, `image()`, `file_list()`, or `custom_format()`.
    pub fn commit(self) -> Result<(), Error> {
        if self.pending_formats.is_empty() {
            return Err(Error::Unknown {
                description: "No formats were added to the clipboard".to_string(),
            });
        }
        self.platform.commit_all(self.pending_formats)
    }
}

/// A builder for an operation that clears the data from the clipboard.
#[must_use]
pub struct Clear<'clipboard> {
    pub(crate) platform: platform::Clear<'clipboard>,
}

impl Clear<'_> {
    /// Completes the "clear" operation by deleting any existing clipboard data,
    /// regardless of the format.
    pub fn default(self) -> Result<(), Error> {
        self.platform.clear()
    }
}

/// All tests grouped in one because the windows clipboard cannot be open on
/// multiple threads at once.
#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::Arc, thread, time::Duration};

    #[test]
    fn all_tests() {
        let _ = env_logger::builder().is_test(true).try_init();
        {
            let mut ctx = Clipboard::new().unwrap();
            let text = "some string";
            ctx.set_text(text).unwrap();
            assert_eq!(ctx.get_text().unwrap(), text);

            // We also need to check that the content persists after the drop; this is
            // especially important on X11
            drop(ctx);

            // Give any external mechanism a generous amount of time to take over
            // responsibility for the clipboard, in case that happens asynchronously
            // (it appears that this is the case on X11 plus Mutter 3.34+, see #4)
            thread::sleep(Duration::from_millis(300));

            let mut ctx = Clipboard::new().unwrap();
            assert_eq!(ctx.get_text().unwrap(), text);
        }
        {
            let mut ctx = Clipboard::new().unwrap();
            let text = "Some utf8: 🤓 ∑φ(n)<ε 🐔";
            ctx.set_text(text).unwrap();
            assert_eq!(ctx.get_text().unwrap(), text);
        }
        {
            let mut ctx = Clipboard::new().unwrap();
            let text = "hello world";

            ctx.set_text(text).unwrap();
            assert_eq!(ctx.get_text().unwrap(), text);

            ctx.clear().unwrap();

            match ctx.get_text() {
                Ok(text) => assert!(text.is_empty()),
                Err(Error::ContentNotAvailable) => {}
                Err(e) => panic!("unexpected error: {e}"),
            };

            // confirm it is OK to clear when already empty.
            ctx.clear().unwrap();
        }
        {
            let mut ctx = Clipboard::new().unwrap();
            let html = "<b>hello</b> <i>world</i>!";

            ctx.set_html(html, None).unwrap();

            match ctx.get_text() {
                Ok(text) => assert!(text.is_empty()),
                Err(Error::ContentNotAvailable) => {}
                Err(e) => panic!("unexpected error: {e}"),
            };
        }
        {
            let mut ctx = Clipboard::new().unwrap();

            let html = "<b>hello</b> <i>world</i>!";
            let alt_text = "hello world!";

            ctx.set_html(html, Some(alt_text)).unwrap();
            assert_eq!(ctx.get_text().unwrap(), alt_text);
        }
        {
            let mut ctx = Clipboard::new().unwrap();

            let html = "<b>hello</b> <i>world</i>!";

            ctx.set().html(html, None).unwrap();

            if cfg!(target_os = "macos") {
                // Copying HTML on macOS adds wrapper content to work around
                // historical platform bugs. We control this wrapper, so we are
                // able to check that the full user data still appears and at what
                // position in the final copy contents.
                let content = ctx.get().html().unwrap();
                assert!(content.ends_with(&format!("{html}</body></html>")));
            } else {
                assert_eq!(ctx.get().html().unwrap(), html);
            }
        }
        {
            let mut ctx = Clipboard::new().unwrap();

            let this_dir = env!("CARGO_MANIFEST_DIR");

            let paths = &[
                PathBuf::from(this_dir).join("README.md"),
                PathBuf::from(this_dir).join("Cargo.toml"),
            ];

            ctx.set().file_list(paths).unwrap();
            assert_eq!(ctx.get().file_list().unwrap().as_slice(), paths);
        }
        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "android", target_os = "emscripten")),
        ))]
        {
            use super::{LinuxClipboardKind, SetExtLinux};
            use std::sync::atomic::{self, AtomicBool};

            let mut ctx = Clipboard::new().unwrap();

            const TEXT1: &str = "I'm a little teapot,";
            const TEXT2: &str = "short and stout,";
            const TEXT3: &str = "here is my handle";

            ctx.set().clipboard(LinuxClipboardKind::Clipboard).text(TEXT1.to_string()).unwrap();

            ctx.set().clipboard(LinuxClipboardKind::Primary).text(TEXT2.to_string()).unwrap();

            // The secondary clipboard is not available under wayland
            if !cfg!(feature = "wayland-data-control")
                || std::env::var_os("WAYLAND_DISPLAY").is_none()
            {
                ctx.set().clipboard(LinuxClipboardKind::Secondary).text(TEXT3.to_string()).unwrap();
            }

            assert_eq!(TEXT1, &ctx.get().clipboard(LinuxClipboardKind::Clipboard).text().unwrap());

            assert_eq!(TEXT2, &ctx.get().clipboard(LinuxClipboardKind::Primary).text().unwrap());

            // The secondary clipboard is not available under wayland
            if !cfg!(feature = "wayland-data-control")
                || std::env::var_os("WAYLAND_DISPLAY").is_none()
            {
                assert_eq!(
                    TEXT3,
                    &ctx.get().clipboard(LinuxClipboardKind::Secondary).text().unwrap()
                );
            }

            let was_replaced = Arc::new(AtomicBool::new(false));

            let setter = thread::spawn({
                let was_replaced = was_replaced.clone();
                move || {
                    thread::sleep(Duration::from_millis(100));
                    let mut ctx = Clipboard::new().unwrap();
                    ctx.set_text("replacement text".to_owned()).unwrap();
                    was_replaced.store(true, atomic::Ordering::Release);
                }
            });

            ctx.set().wait().text("initial text".to_owned()).unwrap();

            assert!(was_replaced.load(atomic::Ordering::Acquire));

            setter.join().unwrap();
        }
    }

    // The cross-platform abstraction should allow any number of clipboards
    // to be open at once without issue, as documented under [Clipboard].
    #[test]
    fn multiple_clipboards_at_once() {
        const THREAD_COUNT: usize = 100;

        let mut handles = Vec::with_capacity(THREAD_COUNT);
        let barrier = Arc::new(std::sync::Barrier::new(THREAD_COUNT));

        for _ in 0..THREAD_COUNT {
            let barrier = barrier.clone();
            handles.push(thread::spawn(move || {
                // As long as the clipboard isn't used multiple times at once, multiple instances
                // are perfectly fine.
                let _ctx = Clipboard::new().unwrap();

                thread::sleep(Duration::from_millis(10));

                barrier.wait();
            }));
        }

        for thread_handle in handles {
            thread_handle.join().unwrap();
        }
    }

    #[test]
    fn clipboard_trait_consistently() {
        fn assert_send_sync<T: Send + Sync + 'static>() {}

        assert_send_sync::<Clipboard>();
        assert!(std::mem::needs_drop::<Clipboard>());
    }
}
