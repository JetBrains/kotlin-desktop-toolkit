#![cfg(target_os = "windows")]

use std::sync::atomic::{AtomicPtr, Ordering};

use windows::{
    Win32::{
        Foundation::{HINSTANCE, TRUE},
        System::SystemServices::DLL_PROCESS_ATTACH,
    },
    core::BOOL,
};

mod logger_api;
pub mod win32;

extern crate desktop_common;

static DLL_HINSTANCE: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

fn get_dll_instance() -> HINSTANCE {
    HINSTANCE(DLL_HINSTANCE.load(Ordering::Relaxed))
}

#[unsafe(no_mangle)]
extern "system" fn DllMain(instance: HINSTANCE, reason: u32, _reserved: *mut std::ffi::c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        DLL_HINSTANCE.store(instance.0, Ordering::Relaxed);
    }
    TRUE
}
