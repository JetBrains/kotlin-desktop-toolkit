use anyhow::{ensure, Context};
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSLayoutConstraint, NSView, NSWindow, NSWindowButton, NSWindowStyleMask, NSWindowTitleVisibility};
use objc2_foundation::NSArray;
use crate::geometry::LogicalPixels;
use crate::macos::window_api::TitlebarConfiguration;

pub(crate) struct Titlebar {
    ns_window: Retained<NSWindow>,
    state: TitlebarState,
}

enum TitlebarState {
    Regular,
    Custom(CustomTitlebarState),
}

impl Titlebar {
    pub(crate) fn new(ns_window: &NSWindow, title_bar_mode: &TitlebarConfiguration) -> Self {
        let state = match title_bar_mode {
            TitlebarConfiguration::Regular => TitlebarState::Regular,
            TitlebarConfiguration::Custom { title_bar_height } => {
                let mut state = CustomTitlebarState {
                    height: *title_bar_height,
                    constraints: None,
                };
                state.init(ns_window);
                TitlebarState::Custom(state)
            }
        };
        Self {
            ns_window: ns_window.into(),
            state,
        }
    }

    pub(crate) fn set_mode(&mut self, titlebar_mode: &TitlebarConfiguration) {
        match (&mut self.state, titlebar_mode) {
            (TitlebarState::Regular, TitlebarConfiguration::Regular) => {
                // do nothing
            }
            (TitlebarState::Custom(state), TitlebarConfiguration::Regular) => {
                state.deinit(&self.ns_window);
                self.state = TitlebarState::Regular;
            }
            (TitlebarState::Regular, TitlebarConfiguration::Custom { title_bar_height }) => {
                let mut state = CustomTitlebarState {
                    height: *title_bar_height,
                    constraints: None,
                };
                state.init(&self.ns_window);
                self.state = TitlebarState::Custom(state);
            }
            (TitlebarState::Custom(state), TitlebarConfiguration::Custom { title_bar_height }) => {
                if state.height != *title_bar_height {
                    state.deactivate_constraints(&self.ns_window).unwrap();
                    state.height = *title_bar_height;
                    state.activate_constraints(&self.ns_window).unwrap();
                }
            }
        }
    }

    pub(crate) fn before_enter_fullscreen(&mut self) {
        if let TitlebarState::Custom(ref mut state) = self.state {
            state.deactivate_constraints(&self.ns_window).unwrap();
        }
    }

    pub(crate) fn after_enter_fullscreen(&mut self) {
        if let TitlebarState::Custom(..) = self.state {
            set_default_titlebar_enabled(&self.ns_window, true);
        }
    }

    pub(crate) fn before_exit_fullscreen(&mut self) {
        if let TitlebarState::Custom(..) = self.state {
            set_default_titlebar_enabled(&self.ns_window, false);
        }
    }

    pub(crate) fn after_exit_fullscreen(&mut self) {
        if let TitlebarState::Custom(ref mut state) = self.state {
            state.activate_constraints(&self.ns_window).unwrap();
        }
    }
}

struct CustomTitlebarState {
    height: LogicalPixels,
    constraints: Option<Retained<NSArray<NSLayoutConstraint>>>,
}

impl CustomTitlebarState {
    fn init(&mut self, ns_window: &NSWindow) {
        let mut style_mask = ns_window.styleMask();
        style_mask |= NSWindowStyleMask::FullSizeContentView;
        ns_window.setStyleMask(style_mask);
        if !style_mask.contains(NSWindowStyleMask::FullScreen) {
            set_default_titlebar_enabled(ns_window, false);
            self.activate_constraints(ns_window).unwrap();
        }
    }

    fn deinit(&mut self, ns_window: &NSWindow) {
        let mut style_mask = ns_window.styleMask();
        style_mask &= !NSWindowStyleMask::FullSizeContentView;
        ns_window.setStyleMask(style_mask);
        if !style_mask.contains(NSWindowStyleMask::FullScreen) {
            self.deactivate_constraints(ns_window).unwrap();
            set_default_titlebar_enabled(ns_window, true);
        }
    }

    fn activate_constraints(&mut self, ns_window: &NSWindow) -> anyhow::Result<()> {
        ensure!(self.constraints.is_none());

        let titlebar_views = unsafe { TitlebarViews::retrieve_from_window(ns_window)? };
        unsafe { titlebar_views.setTranslatesAutoresizingMaskIntoConstraints(false) };
        let constraints = unsafe { titlebar_views.build_constraints(self.height) };

        unsafe { NSLayoutConstraint::activateConstraints(&constraints) };

        self.constraints = Some(constraints);

        Ok(())
    }

    fn deactivate_constraints(&mut self, ns_window: &NSWindow) -> anyhow::Result<()> {
        let title_bar_views = unsafe { TitlebarViews::retrieve_from_window(ns_window) }?;

        unsafe { title_bar_views.setTranslatesAutoresizingMaskIntoConstraints(true) };
        if let Some(constraints) = self.constraints.take() {
            unsafe { NSLayoutConstraint::deactivateConstraints(&constraints) };
        }
        Ok(())
    }
}

fn set_default_titlebar_enabled(ns_window: &NSWindow, enabled: bool) {
    if enabled {
        ns_window.setTitlebarAppearsTransparent(false);
        ns_window.setTitleVisibility(NSWindowTitleVisibility::Visible);
    } else {
        ns_window.setTitlebarAppearsTransparent(true);
        ns_window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
    }
}

struct TitlebarViews {
    close_button: Retained<NSButton>,
    miniaturize_button: Retained<NSButton>,
    zoom_button: Retained<NSButton>,
    title_bar: Retained<NSView>,
    title_bar_container: Retained<NSView>,
    theme_frame: Retained<NSView>,
}

impl TitlebarViews {
    unsafe fn retrieve_from_window(ns_window: &NSWindow) -> anyhow::Result<Self> {
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
        let close_button = ns_window
            .standardWindowButton(NSWindowButton::CloseButton)
            .context("No Close Button")?;
        let miniaturize_button = ns_window
            .standardWindowButton(NSWindowButton::MiniaturizeButton)
            .context("No Miniaturize Button")?;
        let zoom_button = ns_window
            .standardWindowButton(NSWindowButton::ZoomButton)
            .context("No Zoom Button")?;

        let titlebar = unsafe { close_button.superview() }.context("No titlebar view")?;
        let titlebar_container = unsafe { titlebar.superview() }.context("No titlebar container")?;
        let theme_frame = unsafe { titlebar_container.superview() }.context("No theme frame")?;
        Ok(Self {
            close_button,
            miniaturize_button,
            zoom_button,
            title_bar: titlebar,
            title_bar_container: titlebar_container,
            theme_frame,
        })
    }

    #[allow(non_snake_case)]
    unsafe fn setTranslatesAutoresizingMaskIntoConstraints(&self, value: bool) {
        unsafe {
            self.title_bar_container.setTranslatesAutoresizingMaskIntoConstraints(value);
            self.title_bar.setTranslatesAutoresizingMaskIntoConstraints(value);

            self.close_button.setTranslatesAutoresizingMaskIntoConstraints(value);
            self.miniaturize_button.setTranslatesAutoresizingMaskIntoConstraints(value);
            self.zoom_button.setTranslatesAutoresizingMaskIntoConstraints(value);

            // theme frame should keep folow autoresizing mask to match window constraints
            // self.theme_frame.setTranslatesAutoresizingMaskIntoConstraints(value);
        }
    }

    fn horizontal_button_offset(titlebar_height: LogicalPixels) -> LogicalPixels {
        let minimum_height_without_shrinking = 28.0; // This is the smallest macOS title bar availabe with public APIs as of Monterey
        let shrinking_factor = f64::min(titlebar_height / minimum_height_without_shrinking, 1.0);

        let default_horizontal_buttons_offset = 20.0;
        shrinking_factor * default_horizontal_buttons_offset
    }

    unsafe fn build_constraints(&self, titlebar_height: LogicalPixels) -> Retained<NSArray<NSLayoutConstraint>> {
        let mut constraints_array = Vec::new();

        constraints_array.push(unsafe {
            self.title_bar_container
                .leftAnchor()
                .constraintEqualToAnchor(&self.theme_frame.leftAnchor())
        });
        constraints_array.push(unsafe {
            self.title_bar_container
                .widthAnchor()
                .constraintEqualToAnchor(&self.theme_frame.widthAnchor())
        });
        constraints_array.push(unsafe {
            self.title_bar_container
                .topAnchor()
                .constraintEqualToAnchor(&self.theme_frame.topAnchor())
        });
        let height_constraint = unsafe { self.title_bar_container.heightAnchor().constraintEqualToConstant(titlebar_height) };
        constraints_array.push(height_constraint);

        for view in [&self.title_bar] {
            constraints_array.push(unsafe { view.leftAnchor().constraintEqualToAnchor(&self.title_bar_container.leftAnchor()) });
            constraints_array.push(unsafe { view.rightAnchor().constraintEqualToAnchor(&self.title_bar_container.rightAnchor()) });
            constraints_array.push(unsafe { view.topAnchor().constraintEqualToAnchor(&self.title_bar_container.topAnchor()) });
            constraints_array.push(unsafe { view.bottomAnchor().constraintEqualToAnchor(&self.title_bar_container.bottomAnchor()) });
        }

        let horizontal_button_offset = Self::horizontal_button_offset(titlebar_height);

        for (index, button) in (0u16..).zip([&self.close_button, &self.miniaturize_button, &self.zoom_button]) {
            let button_center_horizontal_shift = f64::from(index).mul_add(horizontal_button_offset, titlebar_height / 2f64);

            constraints_array.push(unsafe {
                button
                    .widthAnchor()
                    .constraintLessThanOrEqualToAnchor_multiplier(&self.title_bar_container.heightAnchor(), 0.5)
            });
            // Those corrections are required to keep the icons perfectly round because macOS adds a constant 2 px in resulting height to their frame
            constraints_array.push(unsafe {
                button
                    .heightAnchor()
                    .constraintEqualToAnchor_multiplier_constant(&button.widthAnchor(), 14.0 / 12.0, -2.0)
            });
            constraints_array.push(unsafe {
                button
                    .centerXAnchor()
                    .constraintEqualToAnchor_constant(&self.title_bar_container.leftAnchor(), button_center_horizontal_shift)
            });
            constraints_array.push(unsafe {
                button
                    .centerYAnchor()
                    .constraintEqualToAnchor(&self.title_bar_container.centerYAnchor())
            });
        }

        NSArray::from_retained_slice(&constraints_array)
    }
}