/*
SPDX-License-Identifier: Apache-2.0 OR MIT

Copyright 2022 The Arboard contributors

The project to which this file belongs is licensed under either of
the Apache 2.0 or the MIT license at the licensee's choice. The terms
and conditions of the chosen license apply to this file.
*/
#![warn(unreachable_pub)]

mod common;

pub use common::Error;

mod platform;
mod x11;

#[cfg(all(unix, not(any(target_os = "macos", target_os = "android", target_os = "emscripten")),))]
pub use platform::{ClearExtLinux, GetExtLinux, LinuxClipboardKind, SetExtLinux};

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
    pub fn new(get_data_transfer_data: Box<dyn Fn(LinuxClipboardKind, &str) -> Vec<u8> + Send>) -> Result<Self, Error> {
        Ok(Clipboard {
            platform: platform::Clipboard::new(get_data_transfer_data)?,
        })
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
        Clear {
            platform: platform::Clear::new(&mut self.platform),
        }
    }

    /// Begins a "get" operation to retrieve data from the clipboard.
    pub fn get(&self) -> Get<'_> {
        Get {
            platform: platform::Get::new(&self.platform),
        }
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
    pub fn custom_format(self, mime_type: &str) -> Result<Vec<u8>, Error> {
        self.platform.custom_format(mime_type)
    }
}

/// A builder for an operation that sets a value to the clipboard.
#[must_use]
pub struct Set<'clipboard> {
    pub(crate) platform: platform::Set<'clipboard>,
    pub(crate) pending_formats: Vec<String>,
}

impl Set<'_> {
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
    pub fn custom_format(&mut self, mime_type: String) {
        self.pending_formats.push(mime_type);
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
