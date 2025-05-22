use core::panic;
use std::cell::RefCell;
use std::collections::HashMap;

use anyhow::Context;
use desktop_common::logger::{PanicDefault, ffi_boundary};
use objc2::{available, msg_send, rc::Retained, ClassType};
use objc2_app_kit::{NSCursor, NSHorizontalDirections, NSVerticalDirections};

#[unsafe(no_mangle)]
pub extern "C" fn cursor_push_hide() {
    ffi_boundary("cursor_push_hide", || {
        unsafe {
            NSCursor::hide();
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn cursor_pop_hide() {
    ffi_boundary("cursor_pop_hide", || {
        unsafe {
            NSCursor::unhide();
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn cursor_set_icon(icon: CursorIcon) {
    ffi_boundary("cursor_set_icon", || {
        CURSOR_ICONS_CACHE.with(|cache| {
            let ns_cursor = cache.borrow_mut().ns_cursor_from_icon(icon);
            unsafe {
                ns_cursor.set();
            }
        });
        Ok(())
    });
}

impl PanicDefault for CursorIcon {
    fn default() -> Self {
        Self::Unknown
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cursor_get_icon() -> CursorIcon {
    ffi_boundary("cursor_get_icon", || {
        CURSOR_ICONS_CACHE.with(|cache| {
            let current = unsafe { NSCursor::currentCursor() };
            let icon = cache.borrow_mut().icon_from_ns_cursor(&current)?;
            Ok(icon)
        })
    })
}

thread_local! {
    static CURSOR_ICONS_CACHE: RefCell<CursorIconsCache> = RefCell::new(CursorIconsCache::new());
}

#[derive(Debug)]
struct CursorIconsCache {
    cache: HashMap<CursorIcon, Retained<NSCursor>>,
    inverted: HashMap<Retained<NSCursor>, CursorIcon>,
}

impl CursorIconsCache {
    fn new() -> Self {
        let mut cache = Self {
            cache: HashMap::new(),
            inverted: HashMap::new(),
        };
        // Add this cursor to cache because it used by app at the beggining
        cache.ns_cursor_from_icon(CursorIcon::ArrowCursor);
        cache
    }

    fn ns_cursor_from_icon(&mut self, icon: CursorIcon) -> Retained<NSCursor> {
        let ns_cursor = self.cache.entry(icon).or_insert_with(|| Self::new_ns_cursor(icon)).clone();
        self.inverted.insert(ns_cursor.clone(), icon);
        ns_cursor
    }

    fn icon_from_ns_cursor(&self, cursor: &NSCursor) -> anyhow::Result<CursorIcon> {
        self.inverted
            .get(cursor)
            .copied()
            .with_context(|| format!("Unknown cursor: {cursor:?}"))
    }

    fn new_ns_cursor(icon: CursorIcon) -> Retained<NSCursor> {
        match icon {
            CursorIcon::ArrowCursor => NSCursor::arrowCursor(),
            CursorIcon::IBeamCursor => NSCursor::IBeamCursor(),
            CursorIcon::CrosshairCursor => NSCursor::crosshairCursor(),
            CursorIcon::ClosedHandCursor => NSCursor::closedHandCursor(),
            CursorIcon::OpenHandCursor => NSCursor::openHandCursor(),
            CursorIcon::PointingHandCursor => NSCursor::pointingHandCursor(),

            CursorIcon::ResizeLeftCursor => {
                if available!(macos = 15.0) {
                    unsafe { NSCursor::columnResizeCursorInDirections(NSHorizontalDirections::Left) }
                } else {
                    #[allow(deprecated)]
                    NSCursor::resizeLeftCursor()
                }
            },
            CursorIcon::ResizeRightCursor => {
                if available!(macos = 15.0) {
                    unsafe { NSCursor::columnResizeCursorInDirections(NSHorizontalDirections::Right) }
                } else {
                    #[allow(deprecated)]
                    NSCursor::resizeRightCursor()
                }
            },
            CursorIcon::ResizeLeftRightCursor => {
                if available!(macos = 15.0) {
                    unsafe { NSCursor::columnResizeCursorInDirections(NSHorizontalDirections::All) }
                } else {
                    #[allow(deprecated)]
                    NSCursor::resizeLeftRightCursor()
                }
            },
            CursorIcon::ResizeUpCursor => {
                if available!(macos = 15.0) {
                    unsafe { NSCursor::rowResizeCursorInDirections(NSVerticalDirections::Up) }
                } else {
                    #[allow(deprecated)]
                    NSCursor::resizeUpCursor()
                }
            },
            CursorIcon::ResizeDownCursor => {
                if available!(macos = 15.0) {
                    unsafe { NSCursor::rowResizeCursorInDirections(NSVerticalDirections::Down) }
                } else {
                    #[allow(deprecated)]
                    NSCursor::resizeDownCursor()
                }
            },
            CursorIcon::ResizeUpDownCursor => {
                if available!(macos = 15.0) {
                    unsafe { NSCursor::rowResizeCursorInDirections(NSVerticalDirections::All) }
                } else {
                    #[allow(deprecated)]
                    NSCursor::resizeUpDownCursor()
                }
            },

            // Next two is undocumented
            // see: https://stackoverflow.com/questions/27242353/cocoa-predefined-resize-mouse-cursor
            CursorIcon::ResizeUpLeftDownRight => unsafe { msg_send![NSCursor::class(), _windowResizeNorthWestSouthEastCursor] },
            CursorIcon::ResizeUpRightDownLeft => unsafe { msg_send![NSCursor::class(), _windowResizeNorthEastSouthWestCursor] },

            CursorIcon::DisappearingItemCursor => NSCursor::disappearingItemCursor(),
            CursorIcon::IBeamCursorForVerticalLayout => NSCursor::IBeamCursorForVerticalLayout(),
            CursorIcon::OperationNotAllowedCursor => NSCursor::operationNotAllowedCursor(),
            CursorIcon::DragLinkCursor => NSCursor::dragLinkCursor(),
            CursorIcon::DragCopyCursor => NSCursor::dragCopyCursor(),
            CursorIcon::ContextualMenuCursor => NSCursor::contextualMenuCursor(),
            CursorIcon::ZoomInCursor => unsafe { NSCursor::zoomInCursor() },
            CursorIcon::ZoomOutCursor => unsafe { NSCursor::zoomOutCursor() },
            CursorIcon::ColumnResizeCursor => unsafe { NSCursor::columnResizeCursor() },
            CursorIcon::RowResizeCursor => unsafe { NSCursor::rowResizeCursor() },
            CursorIcon::Unknown => panic!("Can't create Unknown cursor"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[repr(C)]
pub enum CursorIcon {
    Unknown,
    ArrowCursor,
    IBeamCursor,
    CrosshairCursor,
    ClosedHandCursor,
    OpenHandCursor,
    PointingHandCursor,

    ResizeLeftCursor,
    ResizeRightCursor,
    ResizeLeftRightCursor,
    ResizeUpCursor,
    ResizeDownCursor,
    ResizeUpDownCursor,

    ResizeUpLeftDownRight,
    ResizeUpRightDownLeft,

    DisappearingItemCursor,
    IBeamCursorForVerticalLayout,
    OperationNotAllowedCursor,
    DragLinkCursor,
    DragCopyCursor,
    ContextualMenuCursor,
    ZoomInCursor,
    ZoomOutCursor,
    ColumnResizeCursor,
    RowResizeCursor,
}
