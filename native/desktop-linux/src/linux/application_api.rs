use super::events::EventHandler;
use super::geometry::LogicalRect;
use super::{application::Application, application_state::EglInstance, xdg_desktop_settings_api::XdgDesktopSetting};
use anyhow::{Context, bail};
use desktop_common::ffi_utils::{BorrowedOpaquePtr, BorrowedStrPtr, RustAllocatedRawPtr};
use desktop_common::logger::ffi_boundary;
use log::debug;
use smithay_client_toolkit::reexports::protocols::wp::text_input::zv3::client::zwp_text_input_v3;

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationCallbacks {
    pub on_application_started: extern "C" fn(),
    // Returns true if application should terminate, otherwise termination will be canceled
    pub on_should_terminate: extern "C" fn() -> bool,
    pub on_will_terminate: extern "C" fn(),
    pub on_display_configuration_change: extern "C" fn(),
    pub on_xdg_desktop_settings_change: extern "C" fn(&XdgDesktopSetting),
    pub event_handler: EventHandler,
}

pub type AppPtr<'a> = RustAllocatedRawPtr<'a>;

#[unsafe(no_mangle)]
pub extern "C" fn application_init(callbacks: ApplicationCallbacks) -> AppPtr<'static> {
    let app = ffi_boundary("application_init", || {
        debug!("Application Init");
        Ok(Some(Application::new(callbacks)?))
    });
    AppPtr::from_value(app)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_run_event_loop(mut app_ptr: AppPtr) {
    ffi_boundary("application_run_event_loop", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.run()
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_stop_event_loop(mut app_ptr: AppPtr) {
    debug!("application_stop_event_loop");
    ffi_boundary("application_stop_event_loop", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.exit = true;
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_shutdown(app_ptr: AppPtr) {
    debug!("application_shutdown");
    ffi_boundary("application_shutdown", || {
        let _app = unsafe { app_ptr.to_owned::<Application>() };
        Ok(())
    });
}

#[derive(Debug)]
#[repr(C)]
pub struct GetEglProcFuncData<'a> {
    pub f: extern "C" fn(ctx: BorrowedOpaquePtr<'a>, name: BorrowedStrPtr) -> Option<extern "system" fn()>,
    pub ctx: BorrowedOpaquePtr<'a>,
}

extern "C" fn egl_get_proc_address(ctx_ptr: BorrowedOpaquePtr<'_>, name_ptr: BorrowedStrPtr) -> Option<extern "system" fn()> {
    let name = name_ptr.as_str().unwrap();
    // debug!("egl_get_gl_proc for {name}");
    let egl = unsafe { ctx_ptr.borrow::<EglInstance>() }.expect("egl_get_proc_address: EGL Library not loaded");
    egl.get_proc_address(name)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_get_egl_proc_func(app_ptr: AppPtr<'_>) -> GetEglProcFuncData<'_> {
    debug!("application_get_egl_proc_func");
    let app = unsafe { app_ptr.borrow::<Application>() };
    GetEglProcFuncData {
        f: egl_get_proc_address,
        ctx: BorrowedOpaquePtr::new(app.state.egl.as_ref()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn application_is_event_loop_thread(app_ptr: AppPtr<'_>) -> bool {
    ffi_boundary("application_is_event_loop_thread", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        if let Some(t) = app.state.event_loop_thread_id {
            let current_thread_id = std::thread::current().id();
            Ok(t == current_thread_id)
        } else {
            bail!("Event loop not yet started")
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_run_on_event_loop_async(app_ptr: AppPtr<'_>, f: extern "C" fn()) {
    ffi_boundary("application_run_on_event_loop_async", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        if let Some(s) = &app.state.run_on_event_loop {
            s.send(f).map_err(std::convert::Into::into)
        } else {
            bail!("Event loop not yet started")
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_set_cursor_theme(mut app_ptr: AppPtr<'_>, name: BorrowedStrPtr, size: u32) {
    debug!("application_set_cursor_theme");
    ffi_boundary("application_set_cursor_theme", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.set_cursor_theme(name.as_str().unwrap(), size)
    });
}

#[repr(C)]
#[derive(Debug)]
pub enum TextInputContentPurpose {
    /// default input, allowing all characters
    Normal,
    /// allow only alphabetic characters
    Alpha,
    /// allow only digits
    Digits,
    /// input a number (including decimal separator and sign)
    Number,
    /// input a phone number
    Phone,
    Url,
    /// input an URL
    Email,
    /// input an email address
    Name,
    /// input a name of a person
    Password,
    /// input a password (combine with `sensitive_data` hint)
    Pin,
    /// input is a numeric password (combine with `sensitive_data` hint)
    Date,
    /// input a date
    Time,
    Datetime,
    Terminal,
}

impl TextInputContentPurpose {
    const fn to_system(&self) -> zwp_text_input_v3::ContentPurpose {
        match self {
            Self::Normal => zwp_text_input_v3::ContentPurpose::Normal,
            Self::Alpha => zwp_text_input_v3::ContentPurpose::Alpha,
            Self::Digits => zwp_text_input_v3::ContentPurpose::Digits,
            Self::Number => zwp_text_input_v3::ContentPurpose::Number,
            Self::Phone => zwp_text_input_v3::ContentPurpose::Phone,
            Self::Url => zwp_text_input_v3::ContentPurpose::Url,
            Self::Email => zwp_text_input_v3::ContentPurpose::Email,
            Self::Name => zwp_text_input_v3::ContentPurpose::Name,
            Self::Password => zwp_text_input_v3::ContentPurpose::Password,
            Self::Pin => zwp_text_input_v3::ContentPurpose::Pin,
            Self::Date => zwp_text_input_v3::ContentPurpose::Date,
            Self::Time => zwp_text_input_v3::ContentPurpose::Time,
            Self::Datetime => zwp_text_input_v3::ContentPurpose::Datetime,
            Self::Terminal => zwp_text_input_v3::ContentPurpose::Terminal,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct TextInputContext<'a> {
    pub surrounding_text: BorrowedStrPtr<'a>,
    pub cursor_codepoint_offset: u16,
    pub selection_start_codepoint_offset: u16,
    pub is_multiline: bool,
    pub content_purpose: TextInputContentPurpose,
    pub cursor_rectangle: LogicalRect,
    pub change_caused_by_input_method: bool,
}

impl TextInputContext<'_> {
    fn apply(&self, text_input: &zwp_text_input_v3::ZwpTextInputV3) -> anyhow::Result<()> {
        let surrounding_text = self.surrounding_text.as_str()?;
        let content_hint = if self.is_multiline {
            zwp_text_input_v3::ContentHint::Multiline
        } else {
            zwp_text_input_v3::ContentHint::None
        };
        let cursor_pos_bytes = surrounding_text.char_indices().nth(self.cursor_codepoint_offset.into()).unwrap().0;
        let selection_start_pos_bytes = if self.selection_start_codepoint_offset == self.cursor_codepoint_offset {
            cursor_pos_bytes
        } else {
            surrounding_text
                .char_indices()
                .nth(self.selection_start_codepoint_offset.into())
                .unwrap()
                .0
        };
        #[allow(clippy::cast_possible_truncation)]
        text_input.set_surrounding_text(
            surrounding_text.to_owned(),
            cursor_pos_bytes as i32,
            selection_start_pos_bytes as i32,
        );
        text_input.set_content_type(content_hint, self.content_purpose.to_system());
        text_input.set_text_change_cause(if self.change_caused_by_input_method {
            zwp_text_input_v3::ChangeCause::InputMethod
        } else {
            zwp_text_input_v3::ChangeCause::Other
        });
        text_input.set_cursor_rectangle(
            self.cursor_rectangle.origin.x.round(),
            self.cursor_rectangle.origin.y.round(),
            self.cursor_rectangle.size.width.round(),
            self.cursor_rectangle.size.height.round(),
        );
        text_input.commit();
        Ok(())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn application_text_input_enable(mut app_ptr: AppPtr<'_>, context: TextInputContext) {
    debug!("application_text_input_enable {context:?}");
    ffi_boundary("application_text_input_enable", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        let text_input = app.state.active_text_input.as_mut().context("Active text input")?;
        text_input.enable();
        context.apply(text_input)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_text_input_update(mut app_ptr: AppPtr<'_>, context: TextInputContext) {
    debug!("application_text_input_update {context:?}");
    ffi_boundary("application_text_input_update", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        let text_input = app.state.active_text_input.as_mut().context("Active text input")?;
        context.apply(text_input)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_text_input_disable(mut app_ptr: AppPtr<'_>) {
    debug!("application_text_input_disable");
    ffi_boundary("application_text_input_disable", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        if let Some(text_input) = app.state.active_text_input.as_mut() {
            text_input.disable();
            text_input.commit();
        }
        Ok(())
    });
}
