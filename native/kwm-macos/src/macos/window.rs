use std::{borrow::{Borrow, BorrowMut}, cell::RefCell, ffi::{c_void, CStr}, rc::Rc};

use anyhow::{ensure, Context, Ok};
use bitflags::Flags;
use objc2::{
    declare_class, msg_send_id,
    mutability::{self, MainThreadOnly},
    rc::Retained,
    runtime::{AnyObject, Bool, ProtocolObject},
    sel, ClassType, DeclaredClass,
};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSBackingStoreType, NSButton, NSEvent, NSLayoutConstraint, NSNormalWindowLevel, NSView, NSWindow, NSWindowButton, NSWindowCollectionBehavior, NSWindowDelegate, NSWindowStyleMask, NSWindowTitleVisibility};
use objc2_foundation::{CGPoint, CGRect, CGSize, MainThreadMarker, NSArray, NSMutableArray, NSNotification, NSNumber, NSObject, NSObjectNSComparisonMethods, NSObjectProtocol, NSString};

use crate::{
    common::{LogicalPixels, LogicalPoint, LogicalSize, StrPtr},
    define_objc_ref,
    macos::{application_api::AppState, events::{handle_mouse_moved, handle_window_close_request, handle_window_focus_change, handle_window_full_screen_toggle, handle_window_move, handle_window_resize, handle_window_screen_change}},
};

use super::{events::{Event, MouseMovedEvent}, metal_api::MetalView, screen::{NSScreenExts, ScreenId}};


type CustomTitlebarCell = Rc<RefCell<CustomTitlebar>>;

pub struct Window {
    ns_window: Retained<NSWindow>,
    delegate: Retained<WindowDelegate>,
    root_view: Retained<RootView>,
    custom_titlebar: Option<CustomTitlebarCell>,
}

pub type WindowId = i64;

#[repr(C)]
pub struct WindowParams {
    pub origin: LogicalPoint,
    pub size: LogicalSize,
    pub title: StrPtr,

    pub is_resizable: bool,
    pub is_closable: bool,
    pub is_miniaturizable: bool,

    pub is_full_screen_allowed: bool,
    pub use_custom_titlebar: bool,
}

impl WindowParams {
    fn title(&self) -> &str {
        unsafe { CStr::from_ptr(self.title) }.to_str().unwrap()
    }
}

#[no_mangle]
pub extern "C" fn window_create(params: &WindowParams) -> Box<Window> {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let window = Window::new(mtm, params);
    return Box::new(window)
}

#[no_mangle]
pub extern "C" fn window_drop(window: Box<Window>) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    window.ns_window.close();
    std::mem::drop(window);
}

#[no_mangle]
pub extern "C" fn window_get_window_id(window: &Window) -> WindowId {
    return window.ns_window.window_id();
}

#[no_mangle]
pub extern "C" fn window_get_screen_id(window: &Window) -> ScreenId {
    return window.ns_window.screen().unwrap().screen_id();
}

#[no_mangle]
pub extern "C" fn window_scale_factor(window: &Window) -> f64 {
    return window.ns_window.backingScaleFactor();
}

#[no_mangle]
pub extern "C" fn window_attach_layer(window: &Window, layer: &MetalView) {
    let content_view = window.ns_window.contentView().unwrap();
    let layer_view = &layer.ns_view;
    unsafe {
        layer_view.setFrameSize(content_view.frame().size);
        content_view.addSubview(&*layer.ns_view);
    }
}

#[no_mangle]
pub extern "C" fn window_get_origin(window: &Window) -> LogicalPoint {
    return window.ns_window.get_origin()
}

#[no_mangle]
pub extern "C" fn window_get_size(window: &Window) -> LogicalSize {
    return window.ns_window.get_size()
}

#[no_mangle]
pub extern "C" fn window_set_rect(window: &Window, origin: LogicalPoint, size: LogicalSize, animate: bool) {
    window.ns_window.set_rect(origin.into(), size.into(), animate);
}

#[no_mangle]
pub extern "C" fn window_is_key(window: &Window) -> bool {
    return window.ns_window.isKeyWindow();
}

#[no_mangle]
pub extern "C" fn window_is_main(window: &Window) -> bool {
    return unsafe {
        window.ns_window.isMainWindow()
    }
}

#[no_mangle]
pub extern "C" fn window_get_max_size(window: &Window) -> LogicalSize {
    return window.ns_window.get_max_size();
}

#[no_mangle]
pub extern "C" fn window_set_max_size(window: &Window, size: LogicalSize) {
    window.ns_window.set_max_size(size);
}

#[no_mangle]
pub extern "C" fn window_get_min_size(window: &Window) -> LogicalSize {
    return window.ns_window.get_min_size();
}

#[no_mangle]
pub extern "C" fn window_set_min_size(window: &Window, size: LogicalSize) {
    window.ns_window.set_min_size(size);
}

#[no_mangle]
pub extern "C" fn window_toggle_full_screen(window: &Window) {
    window.ns_window.toggleFullScreen(None);
}

#[no_mangle]
pub extern "C" fn window_is_full_screen(window: &Window) -> bool {
    return window.ns_window.is_full_screen();
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
    fn new(mtm: MainThreadMarker, params: &WindowParams) -> Window {
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

        let ns_window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer_screen(
                mtm.alloc(),
                rect,
                style,
                // the only non depricated NSBackingStoreType
                NSBackingStoreType::NSBackingStoreBuffered,
                // When true, the window server defers creating the window device until the window is moved onscreen.
                false,
                // Screen
                // When sceen is specified the rect considered to be in its coordinate system
                // By default it's relative to primary screen
                None
            )
        };

        let custom_titlebar = if params.use_custom_titlebar {
            ns_window.setTitlebarAppearsTransparent(true);
            ns_window.setTitleVisibility(NSWindowTitleVisibility::NSWindowTitleHidden);
            // see: https://github.com/JetBrains/JetBrainsRuntime/commit/f02479a649f188b4cf7a22fc66904570606a3042
            let titlebar = Rc::new(RefCell::new(unsafe { CustomTitlebar::init_custom_titlebar(&*ns_window, 100.0) }.unwrap()));
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
        ns_window.setContentView(Some(&*root_view));
        assert!(ns_window.makeFirstResponder(Some(&*root_view)) == true); // todo remove assert

        return Window {
            ns_window,
            delegate,
            root_view,
            custom_titlebar
        };
    }
}

struct CustomTitlebar {
    constraints: Option<Retained<NSArray<NSLayoutConstraint>>>,
    height: LogicalPixels
}

struct TitlebarViews {
    close_button: Retained<NSButton>,
    miniaturize_button: Retained<NSButton>,
    zoom_button: Retained<NSButton>,
    titlebar: Retained<NSView>,
    titlebar_container: Retained<NSView>,
    theme_frame: Retained<NSView>
}

impl TitlebarViews {
    unsafe fn retireve_from_window(ns_window: &NSWindow) -> anyhow::Result<TitlebarViews> {
        // The view hierarchy normally looks as follows:
        // NSThemeFrame
        // ├─NSView (content view)
        // └─NSTitlebarContainerView
        //   ├─_NSTitlebarDecorationView (only on Mojave 10.14 and newer)
        //   └─NSTitlebarView
        //     ├─NSVisualEffectView (only on Big Sur 11 and newer)
        //     ├─NSView (only on Big Sur and newer)
        //     ├─_NSThemeCloseWidget - Close
        //     ├─_NSThemeZoomWidget - Full Screen
        //     ├─_NSThemeWidget - Minimize (note the different order compared to their layout)
        //     └─AWTWindowDragView (we will create this)
        //
        // But the order and presence of decorations and effects has been unstable across different macOS versions,
        // even patch upgrades, which is why the code below uses scans instead of indexed access
        //
        let close_button = ns_window.standardWindowButton(NSWindowButton::NSWindowCloseButton).context("No Close Button")?;
        let miniaturize_button = ns_window.standardWindowButton(NSWindowButton::NSWindowMiniaturizeButton).context("No Miniaturize Button")?;
        let zoom_button = ns_window.standardWindowButton(NSWindowButton::NSWindowZoomButton).context("No Zoom Button")?;

        let titlebar = close_button.superview().context("No titlebar view")?;
        let titlebar_container = titlebar.superview().context("No titlebar container")?;
        let theme_frame = titlebar_container.superview().context("No theme frame")?;
        return Ok(Self {
            close_button,
            miniaturize_button,
            zoom_button,
            titlebar,
            titlebar_container,
            theme_frame,
        })
    }

    unsafe fn setTranslatesAutoresizingMaskIntoConstraints(&self, value: bool) {
        self.titlebar_container.setTranslatesAutoresizingMaskIntoConstraints(value);
        self.titlebar.setTranslatesAutoresizingMaskIntoConstraints(value);

        self.close_button.setTranslatesAutoresizingMaskIntoConstraints(value);
        self.miniaturize_button.setTranslatesAutoresizingMaskIntoConstraints(value);
        self.zoom_button.setTranslatesAutoresizingMaskIntoConstraints(value);

        // theme frame should keep folowing autoresizing mask to match window constraints
        // self.theme_frame.setTranslatesAutoresizingMaskIntoConstraints(value);
    }

    fn horizontal_button_offset(titlebar_height: LogicalPixels) -> LogicalPixels {
        let minimum_height_without_shrinking = 28.0; // This is the smallest macOS title bar availabe with public APIs as of Monterey
        let shrinking_factor = f64::min(titlebar_height / minimum_height_without_shrinking, 1.0);

        let default_horizontal_buttons_offset = 20.0;
        return shrinking_factor * default_horizontal_buttons_offset;
    }

    unsafe fn build_constraints(&self, titlebar_height: LogicalPixels) -> Retained<NSArray<NSLayoutConstraint>> {

        let mut constraints_array = Vec::new();

        constraints_array.push(self.titlebar_container.leftAnchor().constraintEqualToAnchor(&self.theme_frame.leftAnchor()));
        constraints_array.push(self.titlebar_container.widthAnchor().constraintEqualToAnchor(&self.theme_frame.widthAnchor()));
        constraints_array.push(self.titlebar_container.topAnchor().constraintEqualToAnchor(&self.theme_frame.topAnchor()));
        let height_constraint = self.titlebar_container.heightAnchor().constraintEqualToConstant(titlebar_height);
        constraints_array.push(height_constraint);

        // todo
//        [self.nsWindow setIgnoreMove:YES];
//
//        self.zoomButtonMouseResponder = [[AWTWindowZoomButtonMouseResponder alloc] initWithWindow:self.nsWindow];
//        [self.zoomButtonMouseResponder release]; // property retains the object
//
//        AWTWindowDragView* windowDragView = [[AWTWindowDragView alloc] initWithPlatformWindow:self.javaPlatformWindow];
//        [titlebar addSubview:windowDragView positioned:NSWindowBelow relativeTo:closeButtonView];

        // todo add dragable area here
        for view in [&self.titlebar] {
            constraints_array.push(view.leftAnchor().constraintEqualToAnchor(&self.titlebar_container.leftAnchor()));
            constraints_array.push(view.rightAnchor().constraintEqualToAnchor(&self.titlebar_container.rightAnchor()));
            constraints_array.push(view.topAnchor().constraintEqualToAnchor(&self.titlebar_container.topAnchor()));
            constraints_array.push(view.bottomAnchor().constraintEqualToAnchor(&self.titlebar_container.bottomAnchor()));
        }

        let horizontal_button_offset = Self::horizontal_button_offset(titlebar_height);

        for (index, button) in [&self.close_button, &self.miniaturize_button, &self.zoom_button].iter().enumerate() {
            let button_center_horizontal_shift = titlebar_height / 2f64 + (index as f64 * horizontal_button_offset);


            constraints_array.push(button.widthAnchor().constraintLessThanOrEqualToAnchor_multiplier(&self.titlebar_container.heightAnchor(), 0.5));
            // Those corrections are required to keep the icons perfectly round because macOS adds a constant 2 px in resulting height to their frame
            constraints_array.push(button.heightAnchor()
                                         .constraintEqualToAnchor_multiplier_constant(&button.widthAnchor(), 14.0/12.0, -2.0));
            constraints_array.push(button.centerXAnchor()
                                         .constraintEqualToAnchor_constant(&self.titlebar_container.leftAnchor(),
                                                                           button_center_horizontal_shift));
            constraints_array.push(button.centerYAnchor()
                                         .constraintEqualToAnchor(&self.titlebar_container.centerYAnchor()));

        }

        return NSArray::from_vec(constraints_array);
    }
}

impl CustomTitlebar {
    unsafe fn init_custom_titlebar(ns_window: &NSWindow, titlebar_height: LogicalPixels) -> anyhow::Result<CustomTitlebar> {
        let titlebar_views = TitlebarViews::retireve_from_window(ns_window)?;

        return Ok(CustomTitlebar {
            constraints: None,
            height: titlebar_height
        })
    }

    unsafe fn activate(&mut self, ns_window: &NSWindow) -> anyhow::Result<()> {
        ensure!(self.constraints.is_none());

        let titlebar_views = TitlebarViews::retireve_from_window(ns_window)?;
        titlebar_views.setTranslatesAutoresizingMaskIntoConstraints(false);
        let constraints = titlebar_views.build_constraints(self.height);

        NSLayoutConstraint::activateConstraints(&constraints);

        self.constraints = Some(constraints);

        return Ok(());
    }

    unsafe fn deactivate(&mut self, ns_window: &NSWindow) -> anyhow::Result<()> {
        let titlebar_views = TitlebarViews::retireve_from_window(ns_window)?;

        titlebar_views.setTranslatesAutoresizingMaskIntoConstraints(true);
        if let Some(constraints) = self.constraints.take() {
            NSLayoutConstraint::deactivateConstraints(&constraints);
        }

        return Ok(());
    }

    fn set_titlebar_height(&mut self) {

    }

    fn before_enter_fullscreen(titlebar: &Option<CustomTitlebarCell>, ns_window: &NSWindow) {
        if let Some(titlebar) = titlebar {
            let mut titlebar = (**titlebar).borrow_mut();
            unsafe {
                titlebar.deactivate(ns_window).unwrap();
            }
        }
    }

    fn after_exit_fullscreen(titlebar: &Option<CustomTitlebarCell>, ns_window: &NSWindow) {
        if let Some(titlebar) = titlebar {
            let mut titlebar = (**titlebar).borrow_mut();
            unsafe {
                titlebar.activate(ns_window).unwrap();
            }
        }
    }
}

pub(crate) struct WindowDelegateIvars {
    ns_window: Retained<NSWindow>,
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
           ns_window: Retained<NSWindow>,
           custom_titlebar: Option<CustomTitlebarCell>) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(WindowDelegateIvars { ns_window, custom_titlebar });
        unsafe { msg_send_id![super(this), init] }
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
            handle_mouse_moved(event); // todo pass to next responder if it's not handled
        }
        #[method(mouseDown:)]
        fn mouse_down(&self, event: &NSEvent) {
            println!("Down Event: {event:?}");
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