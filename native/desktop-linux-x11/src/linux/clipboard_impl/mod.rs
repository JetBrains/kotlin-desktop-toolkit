/*
SPDX-License-Identifier: Apache-2.0 OR MIT

Copyright 2022 The Arboard contributors

The project to which this file belongs is licensed under either of
the Apache 2.0 or the MIT license at the licensee's choice. The terms
and conditions of the chosen license apply to this file.
*/
#![warn(unreachable_pub)]

mod common;

pub use common::{Error, LinuxClipboardKind};
use std::time::Instant;

mod x11;

/// The struct for accessing the clipboard.
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
/// The clipboard and its content is "hosted" inside of the application that last put data onto it.
/// This means that when the last `Clipboard` instance is dropped, the contents may become unavailable to other apps.
/// See [SetExtLinux] for more details.
#[allow(rustdoc::broken_intra_doc_links)]
pub struct Clipboard {
    pub(crate) platform: x11::Clipboard,
}

impl Clipboard {
    /// Creates an instance of the clipboard.
    ///
    /// # Errors
    ///
    /// On some platforms or desktop environments, an error can be returned if clipboards are not
    /// supported. This may be retried.
    pub fn new(get_data_transfer_data: Box<dyn Fn(LinuxClipboardKind, &str) -> Option<Vec<u8>> + Send>) -> Result<Self, Error> {
        Ok(Clipboard {
            platform: x11::Clipboard::new(get_data_transfer_data)?,
        })
    }

    /// Clears any contents that may be present from the platform's default clipboard,
    /// regardless of the format of the data.
    ///
    /// # Errors
    ///
    /// Returns error on Windows or Linux if clipboard cannot be cleared.
    pub fn clear(&mut self, selection: LinuxClipboardKind) -> Result<(), Error> {
        self.platform.clear(selection)
    }

    /// Begins a "get" operation to retrieve data from the clipboard.
    pub fn get(&self, selection: LinuxClipboardKind) -> Get<'_> {
        Get::new(&self.platform, selection)
    }

    /// Begins a "set" operation to set the clipboard's contents.
    pub fn set(&mut self, selection: LinuxClipboardKind) -> Set<'_> {
        Set::new(&mut self.platform, selection)
    }
}

pub(crate) struct Get<'clipboard> {
    clipboard: &'clipboard x11::Clipboard,
    selection: LinuxClipboardKind,
}

impl<'clipboard> Get<'clipboard> {
    pub(crate) fn new(clipboard: &'clipboard x11::Clipboard, selection: LinuxClipboardKind) -> Self {
        Self { clipboard, selection }
    }

    pub(crate) fn custom_format(self, mime_type: &str) -> Result<Vec<u8>, Error> {
        self.clipboard.get_custom_format(self.selection, mime_type)
    }
}

/// Configuration on how long to wait for a new X11 copy event is emitted.
#[derive(Default)]
pub(crate) enum WaitConfig {
    /// Waits until the given [`Instant`] has reached.
    Until(Instant),

    /// Waits forever until a new event is reached.
    Forever,

    /// It shouldn't wait.
    #[default]
    None,
}

pub(crate) struct Set<'clipboard> {
    clipboard: &'clipboard mut x11::Clipboard,
    wait: WaitConfig,
    selection: LinuxClipboardKind,
    pub(crate) pending_formats: Vec<String>,
}

impl<'clipboard> Set<'clipboard> {
    pub(crate) fn new(clipboard: &'clipboard mut x11::Clipboard, selection: LinuxClipboardKind) -> Self {
        Self {
            clipboard,
            wait: WaitConfig::default(),
            selection,
            pending_formats: Vec::new(),
        }
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
        self.clipboard.commit_all_formats(self.pending_formats, self.selection, self.wait)
    }

    // pub(crate) fn get_supported_formats(&self, selection: LinuxClipboardKind) -> Result<Vec<String>, Error> {
    //     self.clipboard.get_supported_formats(selection)
    // }

    /// Whether to wait for the clipboard's contents to be replaced after setting it.
    ///
    /// The Wayland and X11 clipboards work by having the clipboard content being, at any given
    /// time, "owned" by a single process, and that process is expected to reply to all the requests
    /// from any other system process that wishes to access the clipboard's contents. As a
    /// consequence, when that process exits the contents of the clipboard will effectively be
    /// cleared since there is no longer anyone around to serve requests for it.
    ///
    /// This poses a problem for short-lived programs that just want to copy to the clipboard and
    /// then exit, since they don't want to wait until the user happens to copy something else just
    /// to finish. To resolve that, whenever the user copies something you can offload the actual
    /// work to a newly-spawned daemon process which will run in the background (potentially
    /// outliving the current process) and serve all the requests. That process will then
    /// automatically and silently exit once the user copies something else to their clipboard so it
    /// doesn't take up too many resources.
    ///
    /// To support that pattern, this method will not only have the contents of the clipboard be
    /// set, but will also wait and continue to serve requests until the clipboard is overwritten.
    /// As long as you don't exit the current process until that method has returned, you can avoid
    /// all surprising situations where the clipboard's contents seemingly disappear from under your
    /// feet.
    ///
    /// See the [daemonize example] for a demo of how you could implement this.
    ///
    /// [daemonize example]: https://github.com/1Password/arboard/blob/master/examples/daemonize.rs
    fn wait(mut self) -> Self {
        self.wait = WaitConfig::Forever;
        self
    }

    /// Whether or not to wait for the clipboard's content to be replaced after setting it. This waits until the
    /// `deadline` has exceeded.
    ///
    /// This is useful for short-lived programs so it won't block until new contents on the clipboard
    /// were added.
    ///
    /// Note: this is a superset of [`wait()`][wait] and will overwrite any state
    /// that was previously set using it.
    fn wait_until(mut self, deadline: Instant) -> Self {
        self.wait = WaitConfig::Until(deadline);
        self
    }
}

pub(crate) struct Clear<'clipboard> {
    clipboard: &'clipboard mut x11::Clipboard,
}

impl<'clipboard> Clear<'clipboard> {
    pub(crate) fn new(clipboard: &'clipboard mut x11::Clipboard) -> Self {
        Self { clipboard }
    }

    pub(crate) fn clear(self, selection: LinuxClipboardKind) -> Result<(), Error> {
        self.clipboard.clear(selection)
    }
}
