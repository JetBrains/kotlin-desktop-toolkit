use super::{
    text_input_client::TextInputClient,
    window_api::{WindowPtr, with_window},
};

#[unsafe(no_mangle)]
pub extern "C" fn window_set_text_input_client(window_ptr: WindowPtr, client: TextInputClient) {
    with_window(&window_ptr, "window_set_text_input_client", |window| {
        window.set_text_input_client(Some(client))
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_clear_text_input_client(window_ptr: WindowPtr) {
    with_window(&window_ptr, "window_clear_text_input_client", |window| {
        window.set_text_input_client(None)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_ime_enabled(window_ptr: WindowPtr, enabled: bool) {
    with_window(&window_ptr, "window_set_ime_enabled", |window| window.set_ime_enabled(enabled));
}

#[unsafe(no_mangle)]
pub extern "C" fn window_notify_selection_changed(window_ptr: WindowPtr) {
    with_window(&window_ptr, "window_notify_selection_changed", |window| {
        window.update_ime_windows();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_notify_layout_changed(window_ptr: WindowPtr) {
    with_window(&window_ptr, "window_notify_layout_changed", |window| {
        window.update_ime_windows();
        Ok(())
    });
}
