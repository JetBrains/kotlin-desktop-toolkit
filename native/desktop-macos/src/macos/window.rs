use std::{
    cell::{Cell, RefCell},
    ptr::NonNull,
    rc::Rc,
};

use anyhow::{Context, Ok};
use log::debug;
use objc2::{
    DeclaredClass, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject, Sel},
};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSBackingStoreType, NSColor, NSEvent, NSEventModifierFlags, NSModeSwitchFunctionKey, NSNormalWindowLevel, NSRightArrowFunctionKey, NSScreen, NSTextInputClient, NSTextInputContext, NSTrackingArea, NSTrackingAreaOptions, NSUpArrowFunctionKey, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow, NSWindowCollectionBehavior, NSWindowDelegate, NSWindowOrderingMode, NSWindowStyleMask, NSWindowTitleVisibility
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSAttributedString, NSAttributedStringKey, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRange,
    NSRangePointer, NSRect, NSSize, NSString, NSUInteger,
};

use crate::{
    common::{BorrowedStrPtr, LogicalPoint, LogicalRect, LogicalSize},
    logger::catch_panic,
    macos::{
        keyboard::KeyEventInfo,
        text_operations::{SetMarkedTextOperation, TextRange, UnmarkTextOperation},
    },
};

use super::{
    application_api::MyNSApplication,
    custom_titlebar::{CustomTitlebar, CustomTitlebarCell},
    events::{Event, EventHandler},
    keyboard::unpack_key_event,
    metal_api::MetalView,
    screen::NSScreenExts,
    string::{borrow_ns_string, copy_to_ns_string},
    text_operations::{TextChangedOperation, TextCommandOperation, TextOperation},
    window_api::{WindowBackground, WindowCallbacks, WindowId, WindowParams, WindowVisualEffect},
};

pub(crate) struct Window {
    pub(crate) ns_window: Retained<MyNSWindow>,
    #[allow(dead_code)]
    pub(crate) delegate: Retained<WindowDelegate>,
    pub(crate) root_view: Retained<RootView>,
    pub(crate) background_state: RefCell<WindowBackgroundState>,
    #[allow(dead_code)]
    pub(crate) custom_titlebar: Option<CustomTitlebarCell>,
}

pub(crate) struct WindowBackgroundState {
    is_transparent: bool,
    substrate: Option<Retained<NSVisualEffectView>>,
}

impl From<WindowVisualEffect> for NSVisualEffectMaterial {
    fn from(value: WindowVisualEffect) -> Self {
        match value {
            WindowVisualEffect::TitlebarEffect => Self::Titlebar,
            WindowVisualEffect::SelectionEffect => Self::Selection,
            WindowVisualEffect::MenuEffect => Self::Menu,
            WindowVisualEffect::PopoverEffect => Self::Popover,
            WindowVisualEffect::SidebarEffect => Self::Sidebar,
            WindowVisualEffect::HeaderViewEffect => Self::HeaderView,
            WindowVisualEffect::SheetEffect => Self::Sheet,
            WindowVisualEffect::WindowBackgroundEffect => Self::WindowBackground,
            WindowVisualEffect::HUDWindowEffect => Self::HUDWindow,
            WindowVisualEffect::FullScreenUIEffect => Self::FullScreenUI,
            WindowVisualEffect::ToolTipEffect => Self::ToolTip,
            WindowVisualEffect::ContentBackgroundEffect => Self::ContentBackground,
            WindowVisualEffect::UnderWindowBackgroundEffect => Self::UnderWindowBackground,
            WindowVisualEffect::UnderPageBackgroundEffect => Self::UnderPageBackground,
        }
    }
}

pub(crate) trait NSWindowExts {
    fn me(&self) -> &NSWindow;

    fn window_id(&self) -> WindowId {
        unsafe { self.me().windowNumber() }
    }

    fn get_size(&self) -> LogicalSize {
        self.me().frame().size.into()
    }

    fn get_origin(&self, mtm: MainThreadMarker) -> anyhow::Result<LogicalPoint> {
        let screen_height = NSScreen::primary(mtm)?.height();
        let rect = LogicalRect::from_macos_coords(self.me().frame(), screen_height);
        Ok(rect.origin)
    }

    fn get_content_rect(&self, mtm: MainThreadMarker) -> anyhow::Result<LogicalRect> {
        let ns_window = self.me();
        let window_frame = ns_window.frame();
        let content_frame = ns_window.contentRectForFrameRect(window_frame);
        let screen_height = NSScreen::primary(mtm)?.height();
        Ok(LogicalRect::from_macos_coords(content_frame, screen_height))
    }

    fn set_rect(&self, rect: &LogicalRect, animate: bool, mtm: MainThreadMarker) -> anyhow::Result<()> {
        let screen_height = NSScreen::primary(mtm)?.height();
        unsafe {
            let frame = rect.as_macos_coords(screen_height);
            self.me().setFrame_display_animate(frame, true, animate);
        }
        Ok(())
    }

    fn set_content_rect(&self, rect: &LogicalRect, animate: bool, mtm: MainThreadMarker) -> anyhow::Result<()> {
        let ns_window = self.me();
        let screen_height = NSScreen::primary(mtm)?.height();
        let content_frame = rect.as_macos_coords(screen_height);
        let window_frame = unsafe { ns_window.frameRectForContentRect(content_frame) };
        unsafe {
            self.me().setFrame_display_animate(window_frame, true, animate);
        }
        Ok(())
    }

    fn set_max_size(&self, size: LogicalSize) {
        self.me().setMaxSize(size.into());
    }

    fn set_min_size(&self, size: LogicalSize) {
        self.me().setMinSize(size.into());
    }

    fn get_max_size(&self) -> LogicalSize {
        unsafe { self.me().maxSize().into() }
    }

    fn get_min_size(&self) -> LogicalSize {
        unsafe { self.me().minSize().into() }
    }

    fn is_full_screen(&self) -> bool {
        self.me().styleMask().contains(NSWindowStyleMask::FullScreen)
    }
}

impl NSWindowExts for NSWindow {
    fn me(&self) -> &NSWindow {
        self
    }
}

impl Window {
    pub(crate) fn new(mtm: MainThreadMarker, params: &WindowParams, callbacks: WindowCallbacks) -> anyhow::Result<Self> {
        /*
        see doc: https://developer.apple.com/documentation/appkit/nswindow/stylemask-swift.struct/resizable?language=objc

        NSWindowStyleMask::Titled and NSWindowStyleMask::Borderless
        This two are both represented by the same bit.
        Whem window is borderles it can't become key or main, and there is no decorations

        NSWindowStyleMask::Closable
        NSWindowStyleMask::Miniaturizable
        if one is presented then buttons showed but only one is active
        if none is presented then buttons isn't shown

        NSWindowStyleMask::FullScreen is basically a read-only marker, if you need to change it use ns_window.toggleFullScreen
        */
        let mut style = NSWindowStyleMask::Titled;

        if params.is_closable {
            style |= NSWindowStyleMask::Closable;
        }
        if params.is_miniaturizable {
            style |= NSWindowStyleMask::Miniaturizable;
        }
        if params.is_resizable {
            style |= NSWindowStyleMask::Resizable;
        }

        if params.use_custom_titlebar {
            style |= NSWindowStyleMask::FullSizeContentView;
        }

        let screen_height = NSScreen::primary(mtm)
            .context("Can't create a window without a screen")?
            .frame()
            .size
            .height;

        // Window rect is relative to primary screen
        let frame = LogicalRect::new(params.origin, params.size).as_macos_coords(screen_height);
        let content_rect = unsafe { NSWindow::contentRectForFrameRect_styleMask(frame, style, mtm) };
        let ns_window = MyNSWindow::new(mtm, content_rect, style);
        let custom_titlebar = if params.use_custom_titlebar {
            ns_window.setTitlebarAppearsTransparent(true);
            ns_window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
            // see: https://github.com/JetBrains/JetBrainsRuntime/commit/f02479a649f188b4cf7a22fc66904570606a3042
            let titlebar = Rc::new(RefCell::new(CustomTitlebar::init_custom_titlebar(params.titlebar_height)));
            unsafe {
                // we assume the window isn't full screen
                (*titlebar).borrow_mut().activate(&ns_window).unwrap();
            }
            Some(titlebar)
        } else {
            None
        };

        let mut collection_behaviour: NSWindowCollectionBehavior = unsafe { ns_window.collectionBehavior() };
        if params.is_full_screen_allowed {
            collection_behaviour |= NSWindowCollectionBehavior::FullScreenPrimary;
        } else {
            collection_behaviour |= NSWindowCollectionBehavior::FullScreenNone;
        }
        unsafe {
            // allow full screen for this window
            // https://developer.apple.com/library/archive/documentation/General/Conceptual/MOSXAppProgrammingGuide/FullScreenApp/FullScreenApp.html#:~:text=Full%2Dscreen%20support%20in%20NSApplication,is%20also%20key%2Dvalue%20observable.
            ns_window.setCollectionBehavior(collection_behaviour);
        }
        ns_window.setTitle(&copy_to_ns_string(&params.title).unwrap());
        unsafe {
            ns_window.setReleasedWhenClosed(false);
        }
        ns_window.makeKeyAndOrderFront(None);

        // todo we should use  NSApplication.activate();
        #[allow(deprecated)]
        MyNSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);

        ns_window.setLevel(NSNormalWindowLevel);
        unsafe {
            ns_window.setRestorable(false);
        }

        let delegate = WindowDelegate::new(mtm, ns_window.clone(), callbacks.event_handler, custom_titlebar.clone());
        ns_window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

        let root_view = RootView::new(mtm, callbacks);
        ns_window.setAcceptsMouseMovedEvents(true);

        let container = unsafe { NSView::new(mtm) };
        unsafe {
            container.setAutoresizesSubviews(true);
            container.addSubview_positioned_relativeTo(&root_view, NSWindowOrderingMode::Above, None);
        }

        ns_window.setContentView(Some(&container));
        assert!(ns_window.makeFirstResponder(Some(&root_view))); // todo remove assert

        let window_background = RefCell::new(WindowBackgroundState {
            is_transparent: false,
            substrate: None,
        });

        Ok(Self {
            ns_window,
            delegate,
            root_view,
            custom_titlebar,
            background_state: window_background,
        })
    }

    pub(crate) fn set_background(&self, mtm: MainThreadMarker, background: WindowBackground) -> anyhow::Result<()> {
        let mut background_state = self.background_state.borrow_mut();
        match background {
            WindowBackground::Transparent => {
                if let Some(substrate) = background_state.substrate.take() {
                    unsafe {
                        substrate.removeFromSuperview();
                    }
                }
                self.ns_window.setOpaque(false);
                let ns_color = unsafe { NSColor::clearColor() };
                self.ns_window.setBackgroundColor(Some(&ns_color));
                background_state.is_transparent = true;
            }
            WindowBackground::SolidColor(color) => {
                if let Some(substrate) = background_state.substrate.take() {
                    unsafe {
                        substrate.removeFromSuperview();
                    }
                }
                self.ns_window.setOpaque(true);
                let ns_color: Retained<NSColor> = color.into();
                self.ns_window.setBackgroundColor(Some(&ns_color));
                background_state.is_transparent = false;
            }
            WindowBackground::VisualEffect(window_visual_effect) => {
                let substrate = if let Some(substrate) = background_state.substrate.take() {
                    substrate
                } else {
                    let substrate = unsafe { NSVisualEffectView::new(mtm) };
                    unsafe {
                        substrate.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
                        substrate.setState(NSVisualEffectState::Active);
                        substrate.setFrameSize(self.ns_window.frame().size);
                        substrate.setAutoresizingMask(
                            NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
                        );
                    }
                    let container = self.ns_window.contentView().context("No container")?;
                    unsafe {
                        container.addSubview_positioned_relativeTo(&substrate, NSWindowOrderingMode::Below, None);
                        // None means below all views
                    }
                    substrate
                };
                unsafe {
                    substrate.setMaterial(window_visual_effect.into());
                }
                self.ns_window.setOpaque(true);
                background_state.is_transparent = false;
                background_state.substrate = Some(substrate);
            }
        }
        Ok(())
    }

    pub(crate) fn attach_layer(&self, layer: &MetalView) {
        let content_view = self.ns_window.contentView().unwrap();

        unsafe {
            layer.ns_view.setFrameSize(content_view.frame().size);
            content_view.addSubview_positioned_relativeTo(&layer.ns_view, NSWindowOrderingMode::Below, Some(&self.root_view));
        }
    }
}

pub(crate) struct WindowDelegateIvars {
    ns_window: Retained<MyNSWindow>,
    mtm: MainThreadMarker,
    event_handler: EventHandler,
    custom_titlebar: Option<CustomTitlebarCell>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WindowDelegate"]
    #[ivars = WindowDelegateIvars]
    pub(crate) struct WindowDelegate;

    unsafe impl NSObjectProtocol for WindowDelegate {}

    unsafe impl NSWindowDelegate for WindowDelegate {
        #[unsafe(method(windowDidResize:))]
        unsafe fn window_did_resize(&self, _notification: &NSNotification) {
            catch_panic(|| Ok(self.handle_event(Event::new_window_resize_event)));
        }

        #[unsafe(method(windowDidChangeScreen:))]
        unsafe fn window_did_change_screen(&self, _notification: &NSNotification) {
            catch_panic(|| Ok(self.handle_event(Event::new_window_screen_change_event)));
        }

        #[unsafe(method(windowDidMove:))]
        unsafe fn window_did_move(&self, _notification: &NSNotification) {
            catch_panic(|| Ok(self.handle_event(|window| Event::new_window_move_event(window, self.ivars().mtm))));
        }

        #[unsafe(method(windowWillEnterFullScreen:))]
        unsafe fn window_will_enter_full_screen(&self, _notification: &NSNotification) {
            let ivars = self.ivars();
            CustomTitlebar::before_enter_fullscreen(&ivars.custom_titlebar, &ivars.ns_window);
        }

        #[unsafe(method(windowDidEnterFullScreen:))]
        unsafe fn window_did_enter_full_screen(&self, _notification: &NSNotification) {
            catch_panic(|| Ok(self.handle_event(Event::new_window_full_screen_toggle_event)));
        }

        #[unsafe(method(windowDidExitFullScreen:))]
        unsafe fn window_did_exit_full_screen(&self, _notification: &NSNotification) {
            let ivars = self.ivars();
            CustomTitlebar::after_exit_fullscreen(&ivars.custom_titlebar, &ivars.ns_window);
            catch_panic(|| Ok(self.handle_event(Event::new_window_full_screen_toggle_event)));
        }

        #[unsafe(method(windowDidBecomeKey:))]
        unsafe fn window_did_become_key(&self, _notification: &NSNotification) {
            debug!("windowDidBecomeKey");
            catch_panic(|| Ok(self.handle_event(Event::new_window_focus_change_event)));
        }

        #[unsafe(method(windowDidResignKey:))]
        unsafe fn window_did_resign_key(&self, _notification: &NSNotification) {
            debug!("windowDidResignKey");
            catch_panic(|| Ok(self.handle_event(Event::new_window_focus_change_event)));
        }

        #[unsafe(method(windowDidBecomeMain:))]
        unsafe fn window_did_become_main(&self, _notification: &NSNotification) {
            debug!("windowDidBecomeMain");
            catch_panic(|| Ok(self.handle_event(Event::new_window_focus_change_event)));
        }

        #[unsafe(method(windowDidResignMain:))]
        unsafe fn window_did_resign_main(&self, _notification: &NSNotification) {
            debug!("windowDidResignMain");
            catch_panic(|| Ok(self.handle_event(Event::new_window_focus_change_event)));
        }

        #[unsafe(method(windowShouldClose:))]
        unsafe fn window_should_close(&self, _notification: &NSNotification) -> bool {
            catch_panic(|| Ok(self.handle_event(Event::new_window_close_request_event)));
            false
        }
    }
);

impl WindowDelegate {
    fn new(
        mtm: MainThreadMarker,
        ns_window: Retained<MyNSWindow>,
        event_handler: EventHandler,
        custom_titlebar: Option<CustomTitlebarCell>,
    ) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(WindowDelegateIvars {
            ns_window,
            mtm,
            event_handler,
            custom_titlebar,
        });
        unsafe { msg_send![super(this), init] }
    }

    fn handle_event<'a>(&'a self, f: impl FnOnce(&'a NSWindow) -> Event<'a>) -> bool {
        let ivars = self.ivars();
        let event = f(&ivars.ns_window);
        (ivars.event_handler)(&event)
    }
}

pub(crate) struct MyNSWindowIvars {}

define_class!(
    #[unsafe(super(NSWindow))]
    #[name = "MyNSWindow"]
    #[ivars = MyNSWindowIvars]
    pub(crate) struct MyNSWindow;

    unsafe impl NSObjectProtocol for MyNSWindow {}

    impl MyNSWindow {}
);

impl MyNSWindow {
    pub(crate) fn new(mtm: MainThreadMarker, content_rect: NSRect, style: NSWindowStyleMask) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(MyNSWindowIvars {});
        let ns_window: Retained<Self> = unsafe {
            msg_send![super(this), initWithContentRect: content_rect,
                                                styleMask: style,
                                                 // the only non depricated NSBackingStoreType
                                                  backing: NSBackingStoreType::Buffered,
                                                 // When true, the window server defers creating the window device until the window is moved onscreen.
                                                    defer: false,
                                                 // Screen
                                                 // When sceen is specified the rect considered to be in its coordinate system
                                                 // By default it's relative to primary screen
                                                   screen: Option::<&NSScreen>::None]
        };
        ns_window
    }
}

const DEFAULT_NS_RANGE: NSRange = NSRange { location: 0, length: 0 };
const DEFAULT_NS_RECT: NSRect = NSRect::new(NSPoint::new(0f64, 0f64), NSSize::new(0f64, 0f64));

pub(crate) struct RootViewIvars {
    mtm: MainThreadMarker,
    callbacks: WindowCallbacks,
    tracking_area: Cell<Option<Retained<NSTrackingArea>>>,
    handled_key_down_event: Cell<bool>,
    marked_text_range: Cell<Option<NSRange>>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "RootView"]
    #[ivars = RootViewIvars]
    pub(crate) struct RootView;

    unsafe impl NSObjectProtocol for RootView {}

    unsafe impl NSTextInputClient for RootView {
        // Handling marked text

        #[unsafe(method(hasMarkedText))]
        unsafe fn has_marked_text(&self) -> bool {
            catch_panic(|| Ok(self.has_marked_text_impl())).unwrap_or(false)
        }

        #[unsafe(method(markedRange))]
        unsafe fn marked_range(&self) -> NSRange {
            catch_panic(|| Ok(self.marked_range_impl())).flatten().unwrap_or(DEFAULT_NS_RANGE)
        }

        #[unsafe(method(selectedRange))]
        unsafe fn selected_range(&self) -> NSRange {
            catch_panic(|| Ok(self.selected_range_impl())).unwrap_or(DEFAULT_NS_RANGE)
        }

        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        unsafe fn set_marked_text_selected_range_replacement_range(
            &self,
            string: &AnyObject,
            selected_range: NSRange,
            replacement_range: NSRange,
        ) {
            catch_panic(|| self.set_marked_text_selected_range_replacement_range_impl(string, selected_range, replacement_range));
        }

        #[unsafe(method(unmarkText))]
        unsafe fn unmark_text(&self) {
            catch_panic(|| self.unmark_text_impl());
        }

        #[unsafe(method_id(validAttributesForMarkedText))]
        unsafe fn valid_attributes_for_marked_text(&self) -> Retained<NSArray<NSAttributedStringKey>> {
            debug!("validAttributesForMarkedText");
            let v = vec![
                NSString::from_str("NSFont"),
                NSString::from_str("NSUnderline"),
                NSString::from_str("NSColor"),
                NSString::from_str("NSBackgroundColor"),
                NSString::from_str("NSUnderlineColor"),
                NSString::from_str("NSMarkedClauseSegment"),
                NSString::from_str("NSLanguage"),
                NSString::from_str("NSTextInputReplacementRangeAttributeName"),
                NSString::from_str("NSGlyphInfo"),
                NSString::from_str("NSTextAlternatives"),
                NSString::from_str("NSTextInsertionUndoable"),
            ];
            NSArray::from_retained_slice(&v)
        }

        // Storing text

        #[unsafe(method_id(attributedSubstringForProposedRange:actualRange:))]
        unsafe fn attributed_substring_for_proposed_range_actual_range(
            &self,
            range: NSRange,
            actual_range: NSRangePointer,
        ) -> Option<Retained<NSAttributedString>> {
            catch_panic(|| Ok(self.attributed_substring_for_proposed_range_actual_range_impl(range, actual_range))).flatten()
        }

        #[unsafe(method(insertText:replacementRange:))]
        unsafe fn insert_text_replacement_range(
            &self,
            string: &AnyObject,
            replacement_range: NSRange,
        ) {
            catch_panic(|| self.insert_text_replacement_range_impl(string, replacement_range));
        }

        // Getting character coordinates

        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        unsafe fn first_rect_for_character_range_actual_range(
            &self,
            range: NSRange,
            actual_range: NSRangePointer,
        ) -> NSRect {
            catch_panic(|| self.first_rect_for_character_range_actual_range_impl(range, actual_range)).unwrap_or(DEFAULT_NS_RECT)
        }

        #[unsafe(method(characterIndexForPoint:))]
        unsafe fn character_index_for_point(&self, point: NSPoint) -> NSUInteger {
            catch_panic(|| self.character_index_for_point_impl(point)).unwrap_or(0)
        }

        #[unsafe(method(doCommandBySelector:))]
        unsafe fn do_command_by_selector(&self, selector: Sel) {
            catch_panic(|| self.do_command_by_selector_impl(selector));
        }
    }

    impl RootView {
        #[unsafe(method(updateTrackingArea))]
        fn update_tracking_area(&self) {
            let mtm = unsafe { MainThreadMarker::new_unchecked() };
            catch_panic(|| {
                self.update_tracking_area_impl(mtm);
                Ok(())
            });
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_mouse_event(event, Event::new_mouse_move_event)));
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_mouse_event(event, Event::new_mouse_drag_event)));
        }

        #[unsafe(method(rightMouseDragged:))]
        fn right_mouse_dragged(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_mouse_event(event, Event::new_mouse_drag_event)));
        }

        #[unsafe(method(otherMouseDragged:))]
        fn other_mouse_dragged(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_mouse_event(event, Event::new_mouse_drag_event)));
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_mouse_event(event, Event::new_mouse_enter_event)));
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_mouse_event(event, Event::new_mouse_exit_event)));
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_mouse_event(event, Event::new_mouse_down_event)));
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_mouse_event(event, Event::new_mouse_up_event)));
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_mouse_event(event, Event::new_mouse_down_event)));
        }

        #[unsafe(method(rightMouseUp:))]
        fn right_mouse_up(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_mouse_event(event, Event::new_mouse_up_event)));
        }


        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_mouse_event(event, Event::new_scroll_wheel_event)));
        }

        #[unsafe(method(otherMouseDown:))]
        fn other_mouse_down(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_mouse_event(event, Event::new_mouse_down_event)));
        }

        #[unsafe(method(otherMouseUp:))]
        fn other_mouse_up(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_mouse_event(event, Event::new_mouse_up_event)));
        }

        // Needed for e.g. Ctrl+Tab event reporting
        #[unsafe(method(_wantsKeyDownForEvent:))]
        fn wants_key_down_for_event(&self, event: &NSEvent) -> bool {
            debug!("_wantsKeyDownForEvent: {event:?}");
            return true.into();
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, nsevent: &NSEvent) {
            catch_panic(|| self.key_down_impl(nsevent));
        }

        #[unsafe(method(keyUp:))]
        fn key_up(&self, nsevent: &NSEvent) {
            catch_panic(|| self.key_up_impl(nsevent));
        }

        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: &NSEvent) {
            catch_panic(|| Ok(self.handle_event(&Event::new_modifiers_changed_event(event))));
        }

        // we need those three methods to prevent transparent titlbar from being draggable
        // acceptsFirstMouse, acceptsFirstResponder, opaqueRectForWindowMoveWhenInTitlebar
        // the last one is undocumented in macos
        // please check that titlbar works as expected if you want to change some of them
        // including the case when you click inactive window title bar and starting to drag it
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            return true.into();
        }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            return true.into();
        }

        #[unsafe(method(_opaqueRectForWindowMoveWhenInTitlebar))]
        fn opaque_rect_for_window_move_when_in_titlebar(&self) -> NSRect {
            // for windows with non transparent tiile bar this methods doesn't have any effect
            return self.bounds();
        }
    }
);

fn get_maybe_attributed_string(string: &AnyObject) -> Result<(Option<&NSAttributedString>, Retained<NSString>), anyhow::Error> {
    if let Some(ns_attributed_string) = string.downcast_ref::<NSAttributedString>() {
        let text = ns_attributed_string.string();
        Ok((Some(ns_attributed_string), text))
    } else if let Some(text) = string.downcast_ref::<NSString>() {
        Ok((None, text.into()))
    } else {
        // This method is guaranteed to get either a `NSString` or a `NSAttributedString`.
        panic!("unexpected text {string:?}")
    }
}

impl RootView {
    fn window_id(&self) -> anyhow::Result<WindowId> {
        let window = self.window().context("No window for view")?;
        Ok(window.window_id())
    }

    pub(crate) fn new(mtm: MainThreadMarker, callbacks: WindowCallbacks) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(RootViewIvars {
            mtm,
            callbacks,
            tracking_area: Cell::new(None),
            handled_key_down_event: Cell::new(false),
            marked_text_range: Cell::new(None),
        });
        let root_view: Retained<Self> = unsafe { msg_send![super(this), init] };
        unsafe {
            root_view.setAutoresizesSubviews(true);
            root_view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);
        }
        root_view.update_tracking_area_impl(mtm);
        root_view
    }

    fn handle_event<'a>(&'a self, event: &'a Event) -> bool {
        let callbacks = &self.ivars().callbacks;
        (callbacks.event_handler)(event)
    }

    fn handle_mouse_event<'a>(&'a self, ns_event: &'a NSEvent, f: impl FnOnce(&'a NSEvent, MainThreadMarker) -> Event<'a>) -> bool {
        let ivars = self.ivars();
        let event = f(ns_event, ivars.mtm);
        self.handle_event(&event)
    }

    fn handle_text_operation(&self, operation: &TextOperation) -> bool {
        let callbacks = &self.ivars().callbacks;
        (callbacks.text_operation_handler)(operation)
    }

    fn update_tracking_area_impl(&self, mtm: MainThreadMarker) {
        let rect = self.bounds();
        let options = NSTrackingAreaOptions::MouseEnteredAndExited
            | NSTrackingAreaOptions::ActiveInKeyWindow
            | NSTrackingAreaOptions::EnabledDuringMouseDrag
            | NSTrackingAreaOptions::CursorUpdate
            | NSTrackingAreaOptions::InVisibleRect
            | NSTrackingAreaOptions::AssumeInside;
        let tracking_area = unsafe { NSTrackingArea::initWithRect_options_owner_userInfo(mtm.alloc(), rect, options, Some(self), None) };
        if let Some(old_tracking_area) = self.ivars().tracking_area.take() {
            unsafe {
                self.removeTrackingArea(&old_tracking_area);
            }
        }
        unsafe {
            self.addTrackingArea(&tracking_area);
        }
        self.ivars().tracking_area.set(Some(tracking_area));
    }

    fn has_marked_text_impl(&self) -> bool {
        let ret = self.marked_range_impl().is_some();
        debug!("hasMarkedText: {ret}");
        ret
    }

    fn marked_range_impl(&self) -> Option<NSRange> {
        debug!("markedRange");
        self.ivars().marked_text_range.get()
    }

    fn selected_range_impl(&self) -> NSRange {
        debug!("selectedRange");
        DEFAULT_NS_RANGE // TODO
    }

    fn set_marked_text_selected_range_replacement_range_impl(
        &self,
        string: &AnyObject,
        selected_range: NSRange,
        replacement_range: NSRange,
    ) -> anyhow::Result<bool> {
        let window_id = self.window_id()?;
        let (ns_attributed_string, text) = get_maybe_attributed_string(string)?;
        debug!(
            "setMarkedText, window_id={window_id:?}, marked_text={:?}, string={:?}, selected_range={:?}, replacement_range={:?}",
            ns_attributed_string, text, selected_range, replacement_range
        );
        let handled = self.handle_text_operation(&TextOperation::SetMarkedText(SetMarkedTextOperation {
            window_id,
            text: borrow_ns_string(&text),
            selected_range: TextRange {
                location: selected_range.location,
                length: selected_range.length,
            },
            replacement_range: TextRange {
                location: replacement_range.location,
                length: replacement_range.length,
            },
        }));
        if handled {
            self.ivars().handled_key_down_event.set(true);
            self.ivars().marked_text_range.set(Some(selected_range));
        }
        Ok(handled)
    }

    fn unmark_text_impl(&self) -> anyhow::Result<bool> {
        debug!("unmarkText");
        let handled = self.handle_text_operation(&TextOperation::UnmarkText(UnmarkTextOperation {
            window_id: self.window_id()?,
        }));
        if handled {
            self.ivars().handled_key_down_event.set(true);
            self.ivars().marked_text_range.set(None);
        }
        Ok(handled)
    }

    fn attributed_substring_for_proposed_range_actual_range_impl(
        &self,
        range: NSRange,
        actual_range: NSRangePointer,
    ) -> Option<Retained<NSAttributedString>> {
        let actual_range = NonNull::new(actual_range);
        debug!(
            "attributedSubstringForProposedRange, range={:?}, actual_range={:?}",
            range,
            actual_range.map(|r| unsafe { r.read() })
        );
        None // TODO
    }

    fn insert_text_replacement_range_impl(&self, string: &AnyObject, replacement_range: NSRange) -> anyhow::Result<bool> {
        let window_id = self.window_id()?;
        let (ns_attributed_string, text) = get_maybe_attributed_string(string)?;
        debug!(
            "insertText, marked_text={:?}, string={:?}, replacement_range={:?}",
            ns_attributed_string, text, replacement_range
        );

        let handled = self.handle_text_operation(&TextOperation::TextChanged(TextChangedOperation {
            window_id,
            text: borrow_ns_string(&text),
        }));
        if handled {
            self.ivars().handled_key_down_event.set(true);
            self.ivars().marked_text_range.set(None);
        }
        Ok(handled)
    }

    fn first_rect_for_character_range_actual_range_impl(&self, range: NSRange, actual_range: NSRangePointer) -> anyhow::Result<NSRect> {
        let actual_range = NonNull::new(actual_range);
        debug!(
            "firstRectForCharacterRange: range={:?}, actual_range={:?}",
            range,
            actual_range.map(|r| unsafe { r.read() })
        );
        Ok(DEFAULT_NS_RECT) // TODO
    }

    fn character_index_for_point_impl(&self, point: NSPoint) -> anyhow::Result<NSUInteger> {
        debug!("characterIndexForPoint: {:?}", point);
        Ok(0) // TODO
    }

    fn is_ime_navigation_key(key_event_info: &KeyEventInfo) -> bool {
        const ESC_KEYCODE: u32 = 0x1b; // 27
        let first_char: Option<u32> = if key_event_info.chars.length() > 0 {
            Some(unsafe { key_event_info.chars.characterAtIndex(0).into() })
        } else {
            None
        };
        first_char.map_or(true, |ch| {
            (NSUpArrowFunctionKey..=NSRightArrowFunctionKey).contains(&ch) || ch == ESC_KEYCODE
        })
    }

    fn has_function_modifier(key_event_info: &KeyEventInfo) -> bool {
        if key_event_info.modifiers.contains(NSEventModifierFlags::Function.0) {
            let first_char: Option<u32> = if key_event_info.chars.length() > 0 {
                Some(unsafe { key_event_info.chars.characterAtIndex(0).into() })
            } else {
                None
            };
            first_char.map_or(true, |ch| !(NSUpArrowFunctionKey..=NSModeSwitchFunctionKey).contains(&ch))
        } else {
            false
        }
    }

    fn send_event_to_input_context(&self, ns_event: &NSEvent, input_context: &NSTextInputContext) -> bool {
        if !unsafe { input_context.handleEvent(ns_event) } {
            false
        } else {
            self.ivars().handled_key_down_event.get()
        }
    }

    fn key_down_impl(&self, ns_event: &NSEvent) -> anyhow::Result<bool> {
        debug!("keyDown start: {ns_event:?}");
        let key_event_info = unpack_key_event(ns_event)?;
        debug!("keyDown key_event_info: {key_event_info:?}");
        let key_event = Event::new_key_down_event(&key_event_info);
        let handled: bool = if let Some(input_context) = self.inputContext() {
            if self.has_marked_text_impl()
                || dbg!(Self::is_ime_navigation_key(&key_event_info)
                    && !key_event_info.modifiers.contains(NSEventModifierFlags::Control.0)
                    && !Self::has_function_modifier(&key_event_info))
            {
                self.send_event_to_input_context(&ns_event, &input_context) || self.handle_event(&key_event)
            } else {
                self.handle_event(&key_event) || self.send_event_to_input_context(&ns_event, &input_context)
            }
        } else {
            self.handle_event(&key_event)
        };

        debug!("keyDown end: handled = {handled}");
        self.ivars().handled_key_down_event.set(false);
        Ok(handled)
    }

    fn key_up_impl(&self, ns_event: &NSEvent) -> anyhow::Result<bool> {
        debug!("keyUp start: {ns_event:?}");
        let handled = self.handle_event(&Event::new_key_up_event(&unpack_key_event(ns_event)?));
        debug!("keyUp end: handled = {handled}");
        Ok(handled)
    }

    fn do_command_by_selector_impl(&self, selector: Sel) -> anyhow::Result<bool> {
        let s = selector.name();
        if s == c"noop:" {
            debug!("Ignoring the noop: selector, forwarding the raw event");
            return Ok(false);
        }
        debug!("do_command_by_selector: {s:?}");
        let handled = self.handle_text_operation(&TextOperation::TextCommand(TextCommandOperation {
            window_id: self.window_id()?,
            command: BorrowedStrPtr::new(s),
        }));
        self.ivars().handled_key_down_event.set(handled);
        Ok(handled)
    }
}
