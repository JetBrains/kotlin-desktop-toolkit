use crate::macos::string::copy_to_ns_string;
use desktop_common::ffi_utils::BorrowedStrPtr;
use desktop_common::logger::ffi_boundary;
use objc2::MainThreadMarker;
use objc2_app_kit::NSSound;

#[unsafe(no_mangle)]
pub extern "C" fn sound_play_named(sound_name: BorrowedStrPtr) -> bool {
    ffi_boundary("sound_play_named", || {
        let _mtm = MainThreadMarker::new().unwrap();
        let ns_sound_name = copy_to_ns_string(&sound_name)?;
        if let Some(sound) = NSSound::soundNamed(&ns_sound_name) {
            Ok(sound.play())
        } else {
            Err(anyhow::format_err!(format!("Sound '{ns_sound_name}' not found")))
        }
    })
}
