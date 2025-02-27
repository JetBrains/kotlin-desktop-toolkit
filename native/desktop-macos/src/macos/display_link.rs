use std::{ffi::c_void, ptr::addr_of};

use super::screen::ScreenId;
use crate::logger::{PanicDefault, ffi_boundary};
use anyhow::Result;
use dispatch_sys::{
    _dispatch_main_q, _dispatch_source_type_data_add, dispatch_object_t, dispatch_queue_t, dispatch_resume, dispatch_set_context,
    dispatch_source_cancel, dispatch_source_create, dispatch_source_merge_data, dispatch_source_set_event_handler_f, dispatch_source_t,
    dispatch_suspend,
};
use display_link_sys::CGDirectDisplayID;
use objc2_foundation::MainThreadMarker;

pub type DisplayLinkCallback = extern "C" fn();

#[allow(dead_code)]
pub struct DisplayLinkBox {
    display_link: DisplayLink, // we need it for drop
}

type DisplayLinkPtr = *mut DisplayLinkBox;

#[unsafe(no_mangle)]
pub extern "C" fn display_link_create(screen_id: ScreenId, on_next_frame: DisplayLinkCallback) -> DisplayLinkPtr {
    let display_link_box = ffi_boundary("display_link_create", || {
        let _mtm = MainThreadMarker::new().unwrap();
        let display_link = DisplayLink::new(screen_id, on_next_frame).unwrap();
        Ok(Some(DisplayLinkBox { display_link }))
    });
    display_link_box.map_or(std::ptr::null_mut(), |v| Box::into_raw(Box::new(v)))
}

#[unsafe(no_mangle)]
pub extern "C" fn display_link_drop(display_link_ptr: DisplayLinkPtr) {
    ffi_boundary("display_link_drop", || {
        let display_link: Box<DisplayLinkBox> = unsafe {
            assert!(!display_link_ptr.is_null());
            Box::from_raw(display_link_ptr)
        };
        std::mem::drop(display_link);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn display_link_set_running(display_link_ptr: DisplayLinkPtr, value: bool) {
    ffi_boundary("display_link_set_running", || {
        let display_link = unsafe { &mut display_link_ptr.read().display_link };
        if value != display_link.is_running() {
            if value {
                display_link.start().unwrap();
            } else {
                display_link.stop().unwrap();
            }
        }
        Ok(())
    });
}

impl PanicDefault for bool {
    fn default() -> Self {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn display_link_is_running(display_link_ptr: DisplayLinkPtr) -> bool {
    ffi_boundary("display_link_is_running", || {
        let display_link = unsafe { &mut display_link_ptr.read().display_link };
        Ok(display_link.is_running())
    })
}

// derived from https://github.com/zed-industries/zed/blob/7425d242bc91d054df3c05f2b88307cfb3e9132f/crates/gpui/src/platform/mac/display_link.rs#L25
// https://github.com/zed-industries/zed/blob/b17f2089a2fb7e2f142eb71bf78b007f397629b0/crates/gpui/LICENSE-APACHE
pub struct DisplayLink {
    display_link: display_link_sys::DisplayLink,
    frame_requests: dispatch_source_t,
}

impl DisplayLink {
    pub fn new(display_id: CGDirectDisplayID, callback: unsafe extern "C" fn()) -> Result<Self> {
        unsafe extern "C" fn display_link_callback(
            _display_link_out: *mut display_link_sys::CVDisplayLink,
            _current_time: *const display_link_sys::CVTimeStamp,
            _output_time: *const display_link_sys::CVTimeStamp,
            _flags_in: i64,
            _flags_out: *mut i64,
            frame_requests: *mut c_void,
        ) -> i32 {
            let frame_requests = frame_requests as dispatch_source_t;
            unsafe { dispatch_source_merge_data(frame_requests, 1) };
            0
        }

        unsafe extern "C" fn callback_impl(callback: *mut c_void) {
            unsafe {
                let callback: unsafe extern "C" fn() = std::mem::transmute(callback);
                callback();
            }
        }

        unsafe {
            let frame_requests = dispatch_source_create(&_dispatch_source_type_data_add, 0, 0, dispatch_get_main_queue());
            dispatch_set_context(dispatch_object_t { _ds: frame_requests }, callback as *mut c_void);
            dispatch_source_set_event_handler_f(frame_requests, Some(callback_impl));

            dispatch_resume(dispatch_sys::dispatch_object_t { _ds: frame_requests });

            let display_link = display_link_sys::DisplayLink::new(display_id, display_link_callback, frame_requests.cast::<c_void>())?;

            Ok(Self {
                display_link,
                frame_requests,
            })
        }
    }

    pub fn is_running(&mut self) -> bool {
        unsafe { self.display_link.is_running() }
    }

    pub fn start(&mut self) -> Result<()> {
        unsafe {
            self.display_link.start()?;
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        unsafe {
            self.display_link.stop()?;
        }
        Ok(())
    }
}

impl Drop for DisplayLink {
    fn drop(&mut self) {
        if self.is_running() {
            self.stop().unwrap();
        }

        unsafe {
            dispatch_suspend(dispatch_sys::dispatch_object_t { _ds: self.frame_requests });
        }

        unsafe {
            dispatch_source_cancel(self.frame_requests);
        }
    }
}

/// cbindgen:ignore
mod display_link_sys {
    //! Derived from display-link crate under the following license:
    //! <https://github.com/BrainiumLLC/display-link/blob/master/LICENSE-MIT>
    //! Apple docs: [CVDisplayLink](https://developer.apple.com/documentation/corevideo/cvdisplaylinkoutputcallback?language=objc)
    #![allow(clippy::mixed_attributes_style, dead_code, non_upper_case_globals)]

    pub type CGDirectDisplayID = u32;

    use anyhow::Result;
    use foreign_types::{ForeignType, foreign_type};
    use std::{
        ffi::c_void,
        fmt::{self, Debug, Formatter},
    };

    #[derive(Debug)]
    pub enum CVDisplayLink {}

    foreign_type! {
        pub unsafe type DisplayLink {
            type CType = CVDisplayLink;
            fn drop = CVDisplayLinkRelease;
            fn clone = CVDisplayLinkRetain;
        }
    }

    impl Debug for DisplayLink {
        fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
            formatter.debug_tuple("DisplayLink").field(&self.as_ptr()).finish()
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct CVTimeStamp {
        pub version: u32,
        pub video_time_scale: i32,
        pub video_time: i64,
        pub host_time: u64,
        pub rate_scalar: f64,
        pub video_refresh_period: i64,
        pub smpte_time: CVSMPTETime,
        pub flags: u64,
        pub reserved: u64,
    }

    pub type CVTimeStampFlags = u64;

    pub const kCVTimeStampVideoTimeValid: CVTimeStampFlags = 1 << 0;
    pub const kCVTimeStampHostTimeValid: CVTimeStampFlags = 1 << 1;
    pub const kCVTimeStampSMPTETimeValid: CVTimeStampFlags = 1 << 2;
    pub const kCVTimeStampVideoRefreshPeriodValid: CVTimeStampFlags = 1 << 3;
    pub const kCVTimeStampRateScalarValid: CVTimeStampFlags = 1 << 4;
    pub const kCVTimeStampTopField: CVTimeStampFlags = 1 << 16;
    pub const kCVTimeStampBottomField: CVTimeStampFlags = 1 << 17;
    pub const kCVTimeStampVideoHostTimeValid: CVTimeStampFlags = kCVTimeStampVideoTimeValid | kCVTimeStampHostTimeValid;
    pub const kCVTimeStampIsInterlaced: CVTimeStampFlags = kCVTimeStampTopField | kCVTimeStampBottomField;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct CVSMPTETime {
        pub subframes: i16,
        pub subframe_divisor: i16,
        pub counter: u32,
        pub time_type: u32,
        pub flags: u32,
        pub hours: i16,
        pub minutes: i16,
        pub seconds: i16,
        pub frames: i16,
    }

    pub type CVSMPTETimeType = u32;

    pub const kCVSMPTETimeType24: CVSMPTETimeType = 0;
    pub const kCVSMPTETimeType25: CVSMPTETimeType = 1;
    pub const kCVSMPTETimeType30Drop: CVSMPTETimeType = 2;
    pub const kCVSMPTETimeType30: CVSMPTETimeType = 3;
    pub const kCVSMPTETimeType2997: CVSMPTETimeType = 4;
    pub const kCVSMPTETimeType2997Drop: CVSMPTETimeType = 5;
    pub const kCVSMPTETimeType60: CVSMPTETimeType = 6;
    pub const kCVSMPTETimeType5994: CVSMPTETimeType = 7;

    pub type CVSMPTETimeFlags = u32;

    pub const kCVSMPTETimeValid: CVSMPTETimeFlags = 1 << 0;
    pub const kCVSMPTETimeRunning: CVSMPTETimeFlags = 1 << 1;

    pub type CVDisplayLinkOutputCallback = unsafe extern "C" fn(
        display_link_out: *mut CVDisplayLink,
        // A pointer to the current timestamp. This represents the timestamp when the callback is called.
        current_time: *const CVTimeStamp,
        // A pointer to the output timestamp. This represents the timestamp for when the frame will be displayed.
        output_time: *const CVTimeStamp,
        // Unused
        flags_in: i64,
        // Unused
        flags_out: *mut i64,
        // A pointer to app-defined data.
        display_link_context: *mut c_void,
    ) -> i32;

    #[link(name = "CoreFoundation", kind = "framework")]
    #[link(name = "CoreVideo", kind = "framework")]
    #[allow(improper_ctypes, unknown_lints, clippy::duplicated_attributes)]
    unsafe extern "C" {
        pub fn CVDisplayLinkCreateWithActiveCGDisplays(display_link_out: *mut *mut CVDisplayLink) -> i32;
        pub fn CVDisplayLinkSetCurrentCGDisplay(display_link: &mut DisplayLinkRef, display_id: u32) -> i32;
        pub fn CVDisplayLinkSetOutputCallback(
            display_link: &mut DisplayLinkRef,
            callback: CVDisplayLinkOutputCallback,
            user_info: *mut c_void,
        ) -> i32;
        pub fn CVDisplayLinkStart(display_link: &mut DisplayLinkRef) -> i32;
        pub fn CVDisplayLinkStop(display_link: &mut DisplayLinkRef) -> i32;
        pub fn CVDisplayLinkIsRunning(display_link: &mut DisplayLinkRef) -> bool;
        pub fn CVDisplayLinkRelease(display_link: *mut CVDisplayLink);
        pub fn CVDisplayLinkRetain(display_link: *mut CVDisplayLink) -> *mut CVDisplayLink;
    }

    impl DisplayLink {
        /// Apple docs: [CVDisplayLinkCreateWithCGDisplay](https://developer.apple.com/documentation/corevideo/1456981-cvdisplaylinkcreatewithcgdisplay?language=objc)
        pub unsafe fn new(display_id: CGDirectDisplayID, callback: CVDisplayLinkOutputCallback, user_info: *mut c_void) -> Result<Self> {
            unsafe {
                let mut display_link: *mut CVDisplayLink = 0 as _;

                let code = CVDisplayLinkCreateWithActiveCGDisplays(&mut display_link);
                anyhow::ensure!(code == 0, "could not create display link, code: {}", code);

                let mut display_link = Self::from_ptr(display_link);

                let code = CVDisplayLinkSetOutputCallback(&mut display_link, callback, user_info);
                anyhow::ensure!(code == 0, "could not set output callback, code: {}", code);

                let code = CVDisplayLinkSetCurrentCGDisplay(&mut display_link, display_id);
                anyhow::ensure!(code == 0, "could not assign display to display link, code: {}", code);

                Ok(display_link)
            }
        }
    }

    impl DisplayLinkRef {
        /// Apple docs: [CVDisplayLinkStart](https://developer.apple.com/documentation/corevideo/1457193-cvdisplaylinkstart?language=objc)
        pub unsafe fn start(&mut self) -> Result<()> {
            let code = unsafe { CVDisplayLinkStart(self) };
            anyhow::ensure!(code == 0, "could not start display link, code: {}", code);
            Ok(())
        }

        /// Apple docs: [CVDisplayLinkStop](https://developer.apple.com/documentation/corevideo/1457281-cvdisplaylinkstop?language=objc)
        pub unsafe fn stop(&mut self) -> Result<()> {
            let code = unsafe { CVDisplayLinkStop(self) };
            anyhow::ensure!(code == 0, "could not stop display link, code: {}", code);
            Ok(())
        }

        /// Apple docs: [CVDisplayLinkIsRunning](https://developer.apple.com/documentation/corevideo/cvdisplaylinkisrunning(_:)?language=objc)
        pub unsafe fn is_running(&mut self) -> bool {
            unsafe { CVDisplayLinkIsRunning(self) }
        }
    }
}

fn dispatch_get_main_queue() -> dispatch_queue_t {
    addr_of!(_dispatch_main_q).cast_mut()
}

/// cbindgen:ignore
#[allow(non_camel_case_types, dead_code)]
mod dispatch_sys {
    /* automatically generated by rust-bindgen 0.70.1 */

    pub const DISPATCH_TIME_NOW: u32 = 0;
    pub const DISPATCH_QUEUE_PRIORITY_HIGH: u32 = 2;
    pub type dispatch_function_t = ::std::option::Option<unsafe extern "C" fn(arg1: *mut ::std::os::raw::c_void)>;
    pub type dispatch_time_t = u64;
    unsafe extern "C" {
        pub fn dispatch_time(when: dispatch_time_t, delta: i64) -> dispatch_time_t;
    }
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub union dispatch_object_t {
        pub _os_obj: *mut _os_object_s,
        pub _do: *mut dispatch_object_s,
        pub _dq: *mut dispatch_queue_s,
        pub _dqa: *mut dispatch_queue_attr_s,
        pub _dg: *mut dispatch_group_s,
        pub _ds: *mut dispatch_source_s,
        pub _dch: *mut dispatch_channel_s,
        pub _dm: *mut dispatch_mach_s,
        pub _dmsg: *mut dispatch_mach_msg_s,
        pub _dsema: *mut dispatch_semaphore_s,
        pub _ddata: *mut dispatch_data_s,
        pub _dchannel: *mut dispatch_io_s,
    }
    unsafe extern "C" {
        pub fn dispatch_set_context(object: dispatch_object_t, context: *mut ::std::os::raw::c_void);
    }
    unsafe extern "C" {
        pub fn dispatch_suspend(object: dispatch_object_t);
    }
    unsafe extern "C" {
        pub fn dispatch_resume(object: dispatch_object_t);
    }
    pub type dispatch_queue_t = *mut dispatch_queue_s;
    pub type dispatch_queue_global_t = dispatch_queue_t;
    unsafe extern "C" {
        pub fn dispatch_async_f(queue: dispatch_queue_t, context: *mut ::std::os::raw::c_void, work: dispatch_function_t);
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct dispatch_queue_s {
        pub _address: u8,
    }
    unsafe extern "C" {
        pub static mut _dispatch_main_q: dispatch_queue_s;
    }
    unsafe extern "C" {
        pub fn dispatch_get_global_queue(identifier: isize, flags: usize) -> dispatch_queue_global_t;
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct dispatch_queue_attr_s {
        pub _address: u8,
    }
    unsafe extern "C" {
        pub fn dispatch_after_f(
            when: dispatch_time_t,
            queue: dispatch_queue_t,
            context: *mut ::std::os::raw::c_void,
            work: dispatch_function_t,
        );
    }
    pub type dispatch_source_t = *mut dispatch_source_s;
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct dispatch_source_type_s {
        _unused: [u8; 0],
    }
    pub type dispatch_source_type_t = *const dispatch_source_type_s;
    unsafe extern "C" {
        pub static _dispatch_source_type_data_add: dispatch_source_type_s;
    }
    unsafe extern "C" {
        pub fn dispatch_source_create(
            type_: dispatch_source_type_t,
            handle: usize,
            mask: usize,
            queue: dispatch_queue_t,
        ) -> dispatch_source_t;
    }
    unsafe extern "C" {
        pub fn dispatch_source_set_event_handler_f(source: dispatch_source_t, handler: dispatch_function_t);
    }
    unsafe extern "C" {
        pub fn dispatch_source_cancel(source: dispatch_source_t);
    }
    unsafe extern "C" {
        pub fn dispatch_source_merge_data(source: dispatch_source_t, value: usize);
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct dispatch_data_s {
        pub _address: u8,
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct _os_object_s {
        pub _address: u8,
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct dispatch_object_s {
        pub _address: u8,
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct dispatch_group_s {
        pub _address: u8,
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct dispatch_source_s {
        pub _address: u8,
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct dispatch_channel_s {
        pub _address: u8,
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct dispatch_mach_s {
        pub _address: u8,
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct dispatch_mach_msg_s {
        pub _address: u8,
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct dispatch_semaphore_s {
        pub _address: u8,
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct dispatch_io_s {
        pub _address: u8,
    }
}
