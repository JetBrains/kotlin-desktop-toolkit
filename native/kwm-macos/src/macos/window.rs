use std::{borrow::{Borrow, BorrowMut}, cell::{Cell, RefCell}, ffi::{c_void, CStr}, rc::Rc};

use anyhow::{ensure, Context, Ok};
use bitflags::Flags;
use objc2::{
    declare_class, msg_send, msg_send_id, mutability::{self, MainThreadOnly}, rc::Retained, runtime::{AnyObject, Bool, ProtocolObject}, sel, ClassType, DeclaredClass
};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSBackingStoreType, NSButton, NSColor, NSEvent, NSLayoutConstraint, NSNormalWindowLevel, NSScreen, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow, NSWindowButton, NSWindowCollectionBehavior, NSWindowDelegate, NSWindowOrderingMode, NSWindowStyleMask, NSWindowTitleVisibility};
use objc2_foundation::{CGPoint, CGRect, CGSize, MainThreadMarker, NSArray, NSMutableArray, NSNotification, NSNumber, NSObject, NSObjectNSComparisonMethods, NSObjectProtocol, NSRect, NSString};

use crate::{
    common::{Color, LogicalPixels, LogicalPoint, LogicalSize, StrPtr},
    define_objc_ref,
    macos::{application_api::AppState, custom_titlebar::CustomTitlebar, events::{handle_mouse_down, handle_mouse_move, handle_mouse_up, handle_window_close_request, handle_window_focus_change, handle_window_full_screen_toggle, handle_window_move, handle_window_resize, handle_window_screen_change}},
};

use super::{application_api::MyNSApplication, custom_titlebar::CustomTitlebarCell, events::{Event, MouseMovedEvent}, metal_api::MetalView, screen::{NSScreenExts, ScreenId}, window_api::{WindowBackground, WindowId, WindowParams, WindowVisualEffect}};

#[allow(dead_code)]
pub(crate) struct Window {
    pub(crate) ns_window: Retained<MyNSWindow>,
    pub(crate) delegate: Retained<WindowDelegate>,
    pub(crate) root_view: Retained<RootView>,
    pub(crate) background_state: RefCell<WindowBackgroundState>,
    pub(crate) custom_titlebar: Option<CustomTitlebarCell>,
}

pub(crate) struct WindowBackgroundState {
    is_transparent: bool,
    substrate: Option<Retained<NSVisualEffectView>>
}

impl WindowParams {
    fn title(&self) -> &str {
        unsafe { CStr::from_ptr(self.title) }.to_str().unwrap()
    }
}

impl From<WindowVisualEffect> for NSVisualEffectMaterial {
    fn from(value: WindowVisualEffect) -> Self {
        match value {
            WindowVisualEffect::TitlebarEffect => NSVisualEffectMaterial::Titlebar,
            WindowVisualEffect::SelectionEffect => NSVisualEffectMaterial::Selection,
            WindowVisualEffect::MenuEffect => NSVisualEffectMaterial::Menu,
            WindowVisualEffect::PopoverEffect => NSVisualEffectMaterial::Popover,
            WindowVisualEffect::SidebarEffect => NSVisualEffectMaterial::Sidebar,
            WindowVisualEffect::HeaderViewEffect => NSVisualEffectMaterial::HeaderView,
            WindowVisualEffect::SheetEffect => NSVisualEffectMaterial::Sheet,
            WindowVisualEffect::WindowBackgroundEffect => NSVisualEffectMaterial::WindowBackground,
            WindowVisualEffect::HUDWindowEffect => NSVisualEffectMaterial::HUDWindow,
            WindowVisualEffect::FullScreenUIEffect => NSVisualEffectMaterial::FullScreenUI,
            WindowVisualEffect::ToolTipEffect => NSVisualEffectMaterial::ToolTip,
            WindowVisualEffect::ContentBackgroundEffect => NSVisualEffectMaterial::ContentBackground,
            WindowVisualEffect::UnderWindowBackgroundEffect => NSVisualEffectMaterial::UnderWindowBackground,
            WindowVisualEffect::UnderPageBackgroundEffect => NSVisualEffectMaterial::UnderPageBackground,
        }
    }
}

pub(crate) trait NSWindowExts {
    fn window_id(&self) -> WindowId;

    fn get_size(&self) -> LogicalSize;
    fn get_origin(&self) -> LogicalPoint;
    fn set_rect(&self, origin: LogicalPoint, size: LogicalSize, animate: bool);

    fn set_max_size(&self, size: LogicalSize);
    fn set_min_size(&self, size: LogicalSize);
    fn get_max_size(&self) -> LogicalSize;
    fn get_min_size(&self) -> LogicalSize;

    fn is_full_screen(&self) -> bool;
}

impl NSWindowExts for NSWindow {
    fn window_id(&self) -> WindowId {
        unsafe {
            self.windowNumber() as WindowId
        }
    }

    fn get_size(&self) -> LogicalSize {
        self.frame().size.into()
    }

    fn get_origin(&self) -> LogicalPoint {
        self.frame().origin.into()
    }

    fn set_rect(&self, origin: LogicalPoint, size: LogicalSize, animate: bool) {
        unsafe {
            self.setFrame_display_animate(CGRect::new(origin.into(), size.into()), true, animate);
        }
    }

    fn set_max_size(&self, size: LogicalSize) {
        self.setMaxSize(size.into());
    }

    fn set_min_size(&self, size: LogicalSize) {
        self.setMinSize(size.into());
    }

    fn get_max_size(&self) -> LogicalSize {
        return unsafe {
            self.maxSize().into()
        };
    }

    fn get_min_size(&self) -> LogicalSize {
        return unsafe {
            self.minSize().into()
        };
    }

    fn is_full_screen(&self) -> bool {
        return self.styleMask().contains(NSWindowStyleMask::FullScreen);
    }
}

impl Window {
    pub(crate) fn new(mtm: MainThreadMarker, params: &WindowParams) -> Window {
        let rect = CGRect::new(params.origin.into(), params.size.into());

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

        let ns_window = MyNSWindow::new(mtm, rect, style);

        let custom_titlebar = if params.use_custom_titlebar {
            ns_window.setTitlebarAppearsTransparent(true);
            ns_window.setTitleVisibility(NSWindowTitleVisibility::NSWindowTitleHidden);
            // see: https://github.com/JetBrains/JetBrainsRuntime/commit/f02479a649f188b4cf7a22fc66904570606a3042
            let titlebar = Rc::new(RefCell::new(unsafe { CustomTitlebar::init_custom_titlebar(params.titlebar_height) }.unwrap()));
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
        ns_window.setTitle(&NSString::from_str(params.title()));
        unsafe {
            ns_window.setReleasedWhenClosed(false);
        }
        ns_window.makeKeyAndOrderFront(None);
        ns_window.setLevel(NSNormalWindowLevel);
        unsafe {
            ns_window.setRestorable(false);
        }

        let delegate = WindowDelegate::new(mtm, ns_window.clone(), custom_titlebar.clone());
        ns_window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

        let root_view = RootView::new(mtm);
        ns_window.setAcceptsMouseMovedEvents(true);


        let container = unsafe { NSView::new(mtm) };
        unsafe {
            container.setAutoresizesSubviews(true);
            container.addSubview_positioned_relativeTo(&root_view, NSWindowOrderingMode::NSWindowAbove, None);
        }

        ns_window.setContentView(Some(&container));
        assert!(ns_window.makeFirstResponder(Some(&root_view)) == true); // todo remove assert

        let window_background = RefCell::new(WindowBackgroundState {
            is_transparent: false,
            substrate: None,
        });

        return Window {
            ns_window,
            delegate,
            root_view,
            custom_titlebar,
            background_state: window_background
        };
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
                self.ns_window.setBackgroundColor(Some(unsafe { &NSColor::clearColor() }));
                background_state.is_transparent = true;
            },
            WindowBackground::SolidColor(color) => {
                if let Some(substrate) = background_state.substrate.take() {
                    unsafe {
                        substrate.removeFromSuperview();
                    }
                }
                self.ns_window.setOpaque(true);
                let ns_color: Retained<NSColor> = From::<Color>::from(color);
                self.ns_window.setBackgroundColor(Some(&ns_color));
                background_state.is_transparent = false;
            },
            WindowBackground::VisualEffect(window_visual_effect) => {
                let substrate = if let Some(substrate) = background_state.substrate.take() {
                    substrate
                } else {
                    let substrate = unsafe {
                        NSVisualEffectView::new(mtm)
                    };
                    unsafe {
                        substrate.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
                        substrate.setState(NSVisualEffectState::Active);
                        substrate.setFrameSize(self.ns_window.frame().size);
                        substrate.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewWidthSizable | NSAutoresizingMaskOptions::NSViewHeightSizable);
                    }
                    let container = self.ns_window.contentView().context("No container")?;
                    unsafe {
                        container.addSubview_positioned_relativeTo(&substrate, NSWindowOrderingMode::NSWindowBelow, None); // None means below all views
                    }
                    substrate
                };
                unsafe {
                    substrate.setMaterial(window_visual_effect.into());
                }
                self.ns_window.setOpaque(true);
                background_state.is_transparent = false;
                background_state.substrate = Some(substrate);
            },
        }
        Ok(())
    }

    pub(crate) fn attach_layer(&self, layer: &MetalView) {
        let content_view = self.ns_window.contentView().unwrap();

        unsafe {
            layer.ns_view.setFrameSize(content_view.frame().size);
            content_view.addSubview_positioned_relativeTo(&layer.ns_view, NSWindowOrderingMode::NSWindowBelow, Some(&self.root_view));
        }
    }
}

pub(crate) struct WindowDelegateIvars {
    ns_window: Retained<MyNSWindow>,
    custom_titlebar: Option<CustomTitlebarCell>
}

declare_class!(
    pub(crate) struct WindowDelegate;

    unsafe impl ClassType for WindowDelegate {
        type Super = NSObject;
        type Mutability = MainThreadOnly;
        const NAME: &'static str = "WindowDelegate";
    }

    impl DeclaredClass for WindowDelegate {
        type Ivars = WindowDelegateIvars;
    }

    unsafe impl NSObjectProtocol for WindowDelegate {}

    unsafe impl NSWindowDelegate for WindowDelegate {
        #[method(windowDidResize:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidResize(&self, _notification: &NSNotification) {
            handle_window_resize(&*self.ivars().ns_window);
        }

        #[method(windowDidChangeScreen:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidChangeScreen(&self, _notification: &NSNotification) {
            handle_window_screen_change(&*self.ivars().ns_window);
        }

        #[method(windowDidMove:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidMove(&self, _notification: &NSNotification) {
            handle_window_move(&*self.ivars().ns_window);
        }

        #[method(windowWillEnterFullScreen:)]
        #[allow(non_snake_case)]
        unsafe fn windowWillEnterFullScreen(&self, _notification: &NSNotification) {
            let ivars = self.ivars();
            CustomTitlebar::before_enter_fullscreen(&ivars.custom_titlebar, &ivars.ns_window);
        }

        #[method(windowDidEnterFullScreen:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidEnterFullScreen(&self, _notification: &NSNotification) {
            handle_window_full_screen_toggle(&*self.ivars().ns_window);
        }

        #[method(windowDidExitFullScreen:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidExitFullScreen(&self, _notification: &NSNotification) {
            let ivars = self.ivars();
            CustomTitlebar::after_exit_fullscreen(&ivars.custom_titlebar, &ivars.ns_window);
            handle_window_full_screen_toggle(&*self.ivars().ns_window);
        }

        #[method(windowDidBecomeKey:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidBecomeKey(&self, _notification: &NSNotification) {
            handle_window_focus_change(&*self.ivars().ns_window);
        }

        #[method(windowDidResignKey:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidResignKey(&self, _notification: &NSNotification) {
            handle_window_focus_change(&*self.ivars().ns_window);
        }

        #[method(windowDidBecomeMain:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidBecomeMain(&self, _notification: &NSNotification) {
            handle_window_focus_change(&*self.ivars().ns_window);
        }

        #[method(windowDidResignMain:)]
        #[allow(non_snake_case)]
        unsafe fn windowDidResignMain(&self, _notification: &NSNotification) {
            handle_window_focus_change(&*self.ivars().ns_window);
        }

        #[method(windowShouldClose:)]
        #[allow(non_snake_case)]
        unsafe fn windowShouldClose(&self, _notification: &NSNotification) -> bool {
            handle_window_close_request(&*self.ivars().ns_window);
            false
        }
    }
);

impl WindowDelegate {
    fn new(mtm: MainThreadMarker,
           ns_window: Retained<MyNSWindow>,
           custom_titlebar: Option<CustomTitlebarCell>) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(WindowDelegateIvars { ns_window, custom_titlebar });
        unsafe { msg_send_id![super(this), init] }
    }
}

pub(crate) struct MyNSWindowIvars {}

declare_class!(
    pub(crate) struct MyNSWindow;

    unsafe impl ClassType for MyNSWindow {
        type Super = NSWindow;
        type Mutability = MainThreadOnly;
        const NAME: &'static str = "MyNSWindow";
    }

    impl DeclaredClass for MyNSWindow {
        type Ivars = MyNSWindowIvars;
    }

    unsafe impl NSObjectProtocol for MyNSWindow {}

    unsafe impl MyNSWindow {
    }
);

impl MyNSWindow {
    pub(crate) fn new(mtm: MainThreadMarker, rect: NSRect, style: NSWindowStyleMask) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(MyNSWindowIvars {});
        let ns_window: Retained<Self> = unsafe {
            msg_send_id![super(this), initWithContentRect: rect,
                                                styleMask: style,
                                                 // the only non depricated NSBackingStoreType
                                                  backing: NSBackingStoreType::NSBackingStoreBuffered,
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

pub(crate) struct RootViewIvars {}

declare_class!(
    pub(crate) struct RootView;

    unsafe impl ClassType for RootView {
        type Super = NSView;
        type Mutability = MainThreadOnly;
        const NAME: &'static str = "RootView";
    }

    impl DeclaredClass for RootView {
        type Ivars = RootViewIvars;
    }

    unsafe impl NSObjectProtocol for RootView {}

    unsafe impl RootView {
        #[method(mouseMoved:)]
        fn mouse_moved(&self, event: &NSEvent) {
            handle_mouse_move(event); // todo pass to next responder if it's not handled
        }

        #[method(mouseDragged:)]
        fn mouse_dragged(&self, event: &NSEvent) {
            handle_mouse_move(event);
        }

        #[method(mouseDown:)]
        fn mouse_down(&self, event: &NSEvent) {
            handle_mouse_down(event);
        }

        #[method(mouseUp:)]
        fn mouse_up(&self, event: &NSEvent) {
            handle_mouse_up(event);
        }

        // we need those three methods to prevent transparent titlbar from being draggable
        // acceptsFirstMouse, acceptsFirstResponder, opaqueRectForWindowMoveWhenInTitlebar
        // the last one is undocumented in macos
        // please check that titlbar works as expected if you want to change some of them
        // including the case when you click inactive window title bar and starting to drag it
        #[allow(non_snake_case)]
        #[method(acceptsFirstMouse:)]
        fn acceptsFirstMouse(&self, _event: Option<&NSEvent>) -> bool {
            return true.into();
        }

        #[allow(non_snake_case)]
        #[method(acceptsFirstResponder)]
        fn acceptsFirstResponder(&self) -> bool {
            return true.into();
        }

        #[allow(non_snake_case)]
        #[method(_opaqueRectForWindowMoveWhenInTitlebar)]
        fn opaqueRectForWindowMoveWhenInTitlebar(&self) -> NSRect {
            // for windows with non transparent tiile bar this methods doesn't have any effect
            return self.bounds();
        }
    }
);

impl RootView {
    pub(crate) fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(RootViewIvars {});
        let root_view: Retained<Self> = unsafe { msg_send_id![super(this), init] };
        unsafe {
            root_view.setAutoresizesSubviews(true);
            root_view.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewWidthSizable | NSAutoresizingMaskOptions::NSViewHeightSizable);
        }
        root_view
    }
}