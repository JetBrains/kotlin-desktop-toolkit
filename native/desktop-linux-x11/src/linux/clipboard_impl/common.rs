/*
SPDX-License-Identifier: Apache-2.0 OR MIT

Copyright 2022 The Arboard contributors

The project to which this file belongs is licensed under either of
the Apache 2.0 or the MIT license at the licensee's choice. The terms
and conditions of the chosen license apply to this file.
*/

/// Clipboard selection
///
/// Linux has a concept of clipboard "selections" which tend to be used in different contexts. This
/// enum provides a way to get/set to a specific clipboard.
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
}

/// An error that might happen during a clipboard operation.
///
/// Note that both the `Display` and the `Debug` trait is implemented for this type in such a way
/// that they give a short human-readable description of the error; however the documentation
/// gives a more detailed explanation for each error kind.
#[non_exhaustive]
pub enum Error {
    /// The clipboard contents were not available in the requested format.
    /// This could either be due to the clipboard being empty or the clipboard contents having
    /// an incompatible format to the requested one (eg when calling `get_image` on text)
    ContentNotAvailable,

    /// The selected clipboard is not supported by the current configuration (system and/or environment).
    ///
    /// This can be caused by a few conditions:
    /// - Using the Primary clipboard with an older Wayland compositor (that doesn't support version 2)
    /// - Using the Secondary clipboard on Wayland
    ClipboardNotSupported,

    /// The native clipboard is not accessible due to being held by another party.
    ///
    /// This "other party" could be a different process or it could be within
    /// the same program. So for example you may get this error when trying
    /// to interact with the clipboard from multiple threads at once.
    ///
    /// Note that it's OK to have multiple `Clipboard` instances. The underlying
    /// implementation will make sure that the native clipboard is only
    /// opened for transferring data and then closed as soon as possible.
    ClipboardOccupied,

    /// The image or the text that was about the be transferred to/from the clipboard could not be
    /// converted to the appropriate format.
    ConversionFailure,

    /// Any error that doesn't fit the other error types.
    ///
    /// The `description` field is only meant to help the developer and should not be relied on as a
    /// means to identify an error case during runtime.
    Unknown { description: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ContentNotAvailable => f.write_str("The clipboard contents were not available in the requested format or the clipboard is empty."),
            Error::ClipboardNotSupported => f.write_str("The selected clipboard is not supported with the current system configuration."),
            Error::ClipboardOccupied => f.write_str("The native clipboard is not accessible due to being held by another party."),
            Error::ConversionFailure => f.write_str("The image or the text that was about the be transferred to/from the clipboard could not be converted to the appropriate format."),
            Error::Unknown { description } => f.write_fmt(format_args!("Unknown error while interacting with the clipboard: {description}")),
        }
    }
}

impl std::error::Error for Error {}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        macro_rules! kind_to_str {
			($( $e: pat ),*) => {
				match self {
					$(
						$e => stringify!($e),
					)*
				}
			}
		}
        let name = kind_to_str!(
            ContentNotAvailable,
            ClipboardNotSupported,
            ClipboardOccupied,
            ConversionFailure,
            Unknown { .. }
        );
        f.write_fmt(format_args!("{name} - \"{self}\""))
    }
}

impl Error {
    pub(crate) fn unknown<M: Into<String>>(message: M) -> Self {
        Error::Unknown {
            description: message.into(),
        }
    }
}

#[cfg(any(windows, all(unix, not(target_os = "macos"))))]
pub(crate) struct ScopeGuard<F: FnOnce()> {
    callback: Option<F>,
}

#[cfg(any(windows, all(unix, not(target_os = "macos"))))]
impl<F: FnOnce()> ScopeGuard<F> {
    #[cfg_attr(all(windows, not(feature = "image-data")), allow(dead_code))]
    pub(crate) fn new(callback: F) -> Self {
        ScopeGuard { callback: Some(callback) }
    }
}

#[cfg(any(windows, all(unix, not(target_os = "macos"))))]
impl<F: FnOnce()> Drop for ScopeGuard<F> {
    fn drop(&mut self) {
        if let Some(callback) = self.callback.take() {
            (callback)();
        }
    }
}

pub fn into_unknown<E: std::fmt::Display>(error: E) -> Error {
    Error::Unknown {
        description: error.to_string(),
    }
}
