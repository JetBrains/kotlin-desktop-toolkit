use std::time::Instant;

use super::{Error, common::private, x11};

pub fn into_unknown<E: std::fmt::Display>(error: E) -> Error {
    Error::Unknown {
        description: error.to_string(),
    }
}

/// Clipboard selection
///
/// Linux has a concept of clipboard "selections" which tend to be used in different contexts. This
/// enum provides a way to get/set to a specific clipboard (the default
/// [`Clipboard`](Self::Clipboard) being used for the common platform API). You can choose which
/// clipboard to use with [`GetExtLinux::clipboard`] and [`SetExtLinux::clipboard`].
///
/// See <https://specifications.freedesktop.org/clipboards-spec/clipboards-0.1.txt> for a better
/// description of the different clipboards.
#[derive(Copy, Clone, Debug)]
pub enum LinuxClipboardKind {
    /// Typically used selection for explicit cut/copy/paste actions (ie. windows/macos like
    /// clipboard behavior)
    Clipboard,

    /// Typically used for mouse selections and/or currently selected text. Accessible via middle
    /// mouse click.
    ///
    /// *On Wayland, this may not be available for all systems (requires a compositor supporting
    /// version 2 or above) and operations using this will return an error if unsupported.*
    Primary,

    /// The secondary clipboard is rarely used but theoretically available on X11.
    ///
    /// *On Wayland, this is not be available and operations using this variant will return an
    /// error.*
    Secondary,
}

pub(crate) enum Clipboard {
    X11(x11::Clipboard),
}

impl Clipboard {
    pub(crate) fn new(get_data_transfer_data: Box<dyn Fn(LinuxClipboardKind, &str) -> Vec<u8> + Send>) -> Result<Self, Error> {
        Ok(Self::X11(x11::Clipboard::new(get_data_transfer_data)?))
    }
}

pub(crate) struct Get<'clipboard> {
    clipboard: &'clipboard Clipboard,
    selection: LinuxClipboardKind,
}

impl<'clipboard> Get<'clipboard> {
    pub(crate) fn new(clipboard: &'clipboard Clipboard) -> Self {
        Self {
            clipboard,
            selection: LinuxClipboardKind::Clipboard,
        }
    }

    pub(crate) fn custom_format(self, mime_type: &str) -> Result<Vec<u8>, Error> {
        match self.clipboard {
            Clipboard::X11(clipboard) => clipboard.get_custom_format(self.selection, mime_type),
        }
    }
}

/// Linux-specific extensions to the [`Get`](super::Get) builder.
pub trait GetExtLinux: private::Sealed {
    /// Sets the clipboard the operation will retrieve data from.
    ///
    /// If wayland support is enabled and available, attempting to use the Secondary clipboard will
    /// return an error.
    fn clipboard(self, selection: LinuxClipboardKind) -> Self;
}

impl GetExtLinux for super::Get<'_> {
    fn clipboard(mut self, selection: LinuxClipboardKind) -> Self {
        self.platform.selection = selection;
        self
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
    clipboard: &'clipboard mut Clipboard,
    wait: WaitConfig,
    selection: LinuxClipboardKind,
}

impl<'clipboard> Set<'clipboard> {
    pub(crate) fn new(clipboard: &'clipboard mut Clipboard) -> Self {
        Self {
            clipboard,
            wait: WaitConfig::default(),
            selection: LinuxClipboardKind::Clipboard,
        }
    }

    pub(crate) fn commit_all(self, formats: Vec<String>) -> Result<(), Error> {
        match self.clipboard {
            Clipboard::X11(clipboard) => clipboard.commit_all_formats(formats, self.selection, self.wait),
        }
    }
}

/// Linux specific extensions to the [`Set`](super::Set) builder.
pub trait SetExtLinux: private::Sealed {
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
    fn wait(self) -> Self;

    /// Whether or not to wait for the clipboard's content to be replaced after setting it. This waits until the
    /// `deadline` has exceeded.
    ///
    /// This is useful for short-lived programs so it won't block until new contents on the clipboard
    /// were added.
    ///
    /// Note: this is a superset of [`wait()`][SetExtLinux::wait] and will overwrite any state
    /// that was previously set using it.
    fn wait_until(self, deadline: Instant) -> Self;

    /// Sets the clipboard the operation will store its data to.
    ///
    /// If wayland support is enabled and available, attempting to use the Secondary clipboard will
    /// return an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use arboard::{Clipboard, SetExtLinux, LinuxClipboardKind};
    /// # fn main() -> Result<(), arboard::Error> {
    /// let mut ctx = Clipboard::new()?;
    ///
    /// let clipboard = "This goes in the traditional (ex. Copy & Paste) clipboard.";
    /// ctx.set().clipboard(LinuxClipboardKind::Clipboard).text(clipboard.to_owned())?;
    ///
    /// let primary = "This goes in the primary keyboard. It's typically used via middle mouse click.";
    /// ctx.set().clipboard(LinuxClipboardKind::Primary).text(primary.to_owned())?;
    /// # Ok(())
    /// # }
    /// ```
    fn clipboard(self, selection: LinuxClipboardKind) -> Self;
}

impl SetExtLinux for super::Set<'_> {
    fn wait(mut self) -> Self {
        self.platform.wait = WaitConfig::Forever;
        self
    }

    fn wait_until(mut self, deadline: Instant) -> Self {
        self.platform.wait = WaitConfig::Until(deadline);
        self
    }

    fn clipboard(mut self, selection: LinuxClipboardKind) -> Self {
        self.platform.selection = selection;
        self
    }
}

pub(crate) struct Clear<'clipboard> {
    clipboard: &'clipboard mut Clipboard,
}

impl<'clipboard> Clear<'clipboard> {
    pub(crate) fn new(clipboard: &'clipboard mut Clipboard) -> Self {
        Self { clipboard }
    }

    pub(crate) fn clear(self) -> Result<(), Error> {
        self.clear_inner(LinuxClipboardKind::Clipboard)
    }

    fn clear_inner(self, selection: LinuxClipboardKind) -> Result<(), Error> {
        match self.clipboard {
            Clipboard::X11(clipboard) => clipboard.clear(selection),
        }
    }
}

/// Linux specific extensions to the [Clear] builder.
pub trait ClearExtLinux: private::Sealed {
    /// Performs the "clear" operation on the selected clipboard.
    ///
    /// ### Example
    ///
    /// ```no_run
    /// # use arboard::{Clipboard, LinuxClipboardKind, ClearExtLinux, Error};
    /// # fn main() -> Result<(), Error> {
    /// let mut clipboard = Clipboard::new()?;
    ///
    /// clipboard
    ///     .clear_with()
    ///     .clipboard(LinuxClipboardKind::Secondary)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// If wayland support is enabled and available, attempting to use the Secondary clipboard will
    /// return an error.
    fn clipboard(self, selection: LinuxClipboardKind) -> Result<(), Error>;
}

impl ClearExtLinux for super::Clear<'_> {
    fn clipboard(self, selection: LinuxClipboardKind) -> Result<(), Error> {
        self.platform.clear_inner(selection)
    }
}
