#![allow(clippy::single_element_loop)]

use std::{cell::RefCell, rc::Rc};

use anyhow::{Context, ensure};
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSLayoutConstraint, NSView, NSWindow, NSWindowButton};
use objc2_foundation::NSArray;

use crate::geometry::LogicalPixels;

pub(crate) type CustomTitlebarCell = Rc<RefCell<CustomTitlebar>>;

pub(crate) struct CustomTitlebar {
    constraints: Option<Retained<NSArray<NSLayoutConstraint>>>,
    height: LogicalPixels,
}

struct TitlebarViews {
    close_button: Retained<NSButton>,
    miniaturize_button: Retained<NSButton>,
    zoom_button: Retained<NSButton>,
    titlebar: Retained<NSView>,
    titlebar_container: Retained<NSView>,
    theme_frame: Retained<NSView>,
}

impl TitlebarViews {
    unsafe fn retireve_from_window(ns_window: &NSWindow) -> anyhow::Result<Self> {
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
            titlebar,
            titlebar_container,
            theme_frame,
        })
    }

    #[allow(non_snake_case)]
    unsafe fn setTranslatesAutoresizingMaskIntoConstraints(&self, value: bool) {
        unsafe {
            self.titlebar_container.setTranslatesAutoresizingMaskIntoConstraints(value);
            self.titlebar.setTranslatesAutoresizingMaskIntoConstraints(value);

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
            self.titlebar_container
                .leftAnchor()
                .constraintEqualToAnchor(&self.theme_frame.leftAnchor())
        });
        constraints_array.push(unsafe {
            self.titlebar_container
                .widthAnchor()
                .constraintEqualToAnchor(&self.theme_frame.widthAnchor())
        });
        constraints_array.push(unsafe {
            self.titlebar_container
                .topAnchor()
                .constraintEqualToAnchor(&self.theme_frame.topAnchor())
        });
        let height_constraint = unsafe { self.titlebar_container.heightAnchor().constraintEqualToConstant(titlebar_height) };
        constraints_array.push(height_constraint);

        for view in [&self.titlebar] {
            constraints_array.push(unsafe { view.leftAnchor().constraintEqualToAnchor(&self.titlebar_container.leftAnchor()) });
            constraints_array.push(unsafe { view.rightAnchor().constraintEqualToAnchor(&self.titlebar_container.rightAnchor()) });
            constraints_array.push(unsafe { view.topAnchor().constraintEqualToAnchor(&self.titlebar_container.topAnchor()) });
            constraints_array.push(unsafe { view.bottomAnchor().constraintEqualToAnchor(&self.titlebar_container.bottomAnchor()) });
        }

        let horizontal_button_offset = Self::horizontal_button_offset(titlebar_height);

        for (index, button) in (0u16..).zip([&self.close_button, &self.miniaturize_button, &self.zoom_button]) {
            let button_center_horizontal_shift = f64::from(index).mul_add(horizontal_button_offset, titlebar_height / 2f64);

            constraints_array.push(unsafe {
                button
                    .widthAnchor()
                    .constraintLessThanOrEqualToAnchor_multiplier(&self.titlebar_container.heightAnchor(), 0.5)
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
                    .constraintEqualToAnchor_constant(&self.titlebar_container.leftAnchor(), button_center_horizontal_shift)
            });
            constraints_array.push(unsafe {
                button
                    .centerYAnchor()
                    .constraintEqualToAnchor(&self.titlebar_container.centerYAnchor())
            });
        }

        NSArray::from_retained_slice(&constraints_array)
    }
}

impl CustomTitlebar {
    pub(crate) const fn init_custom_titlebar(titlebar_height: LogicalPixels) -> Self {
        Self {
            constraints: None,
            height: titlebar_height,
        }
    }

    pub(crate) unsafe fn activate(&mut self, ns_window: &NSWindow) -> anyhow::Result<()> {
        ensure!(self.constraints.is_none());

        let titlebar_views = unsafe { TitlebarViews::retireve_from_window(ns_window)? };
        unsafe { titlebar_views.setTranslatesAutoresizingMaskIntoConstraints(false) };
        let constraints = unsafe { titlebar_views.build_constraints(self.height) };

        unsafe { NSLayoutConstraint::activateConstraints(&constraints) };

        self.constraints = Some(constraints);

        Ok(())
    }

    pub(crate) unsafe fn deactivate(&mut self, ns_window: &NSWindow) -> anyhow::Result<()> {
        let titlebar_views = unsafe { TitlebarViews::retireve_from_window(ns_window) }?;

        unsafe { titlebar_views.setTranslatesAutoresizingMaskIntoConstraints(true) };
        if let Some(constraints) = self.constraints.take() {
            unsafe { NSLayoutConstraint::deactivateConstraints(&constraints) };
        }

        Ok(())
    }

    pub(crate) fn before_enter_fullscreen(titlebar: &Option<CustomTitlebarCell>, ns_window: &NSWindow) {
        if let Some(titlebar) = titlebar {
            let mut titlebar = (**titlebar).borrow_mut();
            unsafe {
                titlebar.deactivate(ns_window).unwrap();
            }
        }
    }

    pub(crate) fn after_exit_fullscreen(titlebar: &Option<CustomTitlebarCell>, ns_window: &NSWindow) {
        if let Some(titlebar) = titlebar {
            let mut titlebar = (**titlebar).borrow_mut();
            unsafe {
                titlebar.activate(ns_window).unwrap();
            }
        }
    }
}
