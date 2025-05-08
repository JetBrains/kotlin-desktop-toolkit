use super::application_state::ApplicationState;
use crate::linux::events::ClipboardDataFFI;
use log::{debug, warn};
use smithay_client_toolkit::reexports::calloop::PostAction;
use smithay_client_toolkit::{
    data_device_manager::{
        WritePipe,
        data_device::DataDeviceHandler,
        data_offer::{DataOfferHandler, DragOffer},
        data_source::DataSourceHandler,
    },
    delegate_data_device,
    reexports::client::{
        Connection, Proxy, QueueHandle,
        protocol::{wl_data_device::WlDataDevice, wl_data_device_manager::DndAction, wl_data_source::WlDataSource, wl_surface::WlSurface},
    },
};
use std::ffi::CStr;
use std::io::{Read, Write};

pub const TEXT_MIME_TYPE: &str = "text/plain;charset=utf-8";
pub const URI_LIST_MIME_TYPE: &str = "text/uri-list";

delegate_data_device!(ApplicationState);

impl DataDeviceHandler for ApplicationState {
    fn enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice, x: f64, y: f64, wl_surface: &WlSurface) {
        debug!("DataDeviceHandler::enter: {}, {x}x{y}", wl_surface.id());

        let Some(data_device) = &self.data_device else {
            return;
        };
        let Some(drag_offer) = data_device.data().drag_offer() else {
            return;
        };

        drag_offer.with_mime_types(|mime_types| {
            debug!("DataDeviceHandler::enter: mime_types={mime_types:?}");
            if mime_types.iter().any(|e| e == URI_LIST_MIME_TYPE) {
                drag_offer.accept_mime_type(0, Some(URI_LIST_MIME_TYPE.to_owned()));
            } else if mime_types.iter().any(|e| e == TEXT_MIME_TYPE) {
                drag_offer.accept_mime_type(0, Some(TEXT_MIME_TYPE.to_owned()));
            }
        });

        // Accept the action now just in case
        drag_offer.set_actions(DndAction::Copy, DndAction::Copy);
    }

    fn leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice) {
        debug!("DataDeviceHandler::leave");
    }

    fn motion(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice, x: f64, y: f64) {
        debug!("DataDeviceHandler::motion: {x}x{y}");
    }

    fn selection(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice) {
        if let Some(data_device) = &self.data_device {
            if let Some(selection_offer) = data_device.data().selection_offer() {
                selection_offer.with_mime_types(|mime_types| {
                    debug!("DataDeviceHandler::selection: mime_types={mime_types:?}");
                });
            }
        }
    }

    fn drop_performed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice) {
        debug!("DataDeviceHandler::drop_performed");

        let Some(data_device) = &self.data_device else {
            return;
        };
        let Some(drag_offer) = data_device.data().drag_offer() else {
            return;
        };

        let Some(mime) = drag_offer.with_mime_types(|mime_types| {
            debug!("DataDeviceHandler::enter: mime_types={mime_types:?}");
            if mime_types.iter().any(|e| e == URI_LIST_MIME_TYPE) {
                Some(URI_LIST_MIME_TYPE)
            } else if mime_types.iter().any(|e| e == TEXT_MIME_TYPE) {
                Some(TEXT_MIME_TYPE)
            } else {
                None
            }
        }) else {
            return;
        };
        let read_pipe = drag_offer.receive(mime.to_owned()).unwrap();
        self.loop_handle
            .insert_source(read_pipe, move |(), res, state| {
                let f = unsafe { res.get_mut() };
                let mut buf = Vec::new();
                let size = f.read_to_end(&mut buf).unwrap();
                buf.push(0);
                let cstr = CStr::from_bytes_with_nul(&buf).unwrap();

                debug!("DataDeviceHandler::drop_performed read {size} bytes");
                debug!("DataDeviceHandler::drop_performed value: {cstr:?}");
                if let Some(key_window) = state.get_window(&drag_offer.surface) {
                    if mime == URI_LIST_MIME_TYPE {
                        (key_window.event_handler)(&ClipboardDataFFI::new_file_list(cstr).into());
                    } else {
                        (key_window.event_handler)(&ClipboardDataFFI::new_string(cstr).into());
                    }
                } else {
                    warn!("DataDeviceHandler::drop_performed: No target window");
                }

                PostAction::Remove
            })
            .unwrap();
    }
}

impl DataOfferHandler for ApplicationState {
    fn source_actions(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _offer: &mut DragOffer, actions: DndAction) {
        debug!("DataOfferHandler::source_actions: {actions:?}");
    }

    fn selected_action(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _offer: &mut DragOffer, actions: DndAction) {
        debug!("DataOfferHandler::selected_action: {actions:?}");
    }
}

impl DataSourceHandler for ApplicationState {
    fn accept_mime(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _source: &WlDataSource, mime: Option<String>) {
        debug!("DataSourceHandler::accept_mime: {mime:?}");
    }

    fn send_request(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, source: &WlDataSource, mime: String, mut fd: WritePipe) {
        debug!("DataSourceHandler::send_request: {mime}");

        if self
            .copy_paste_source
            .as_ref()
            .map(smithay_client_toolkit::data_device_manager::data_source::CopyPasteSource::inner)
            == Some(source)
        {
            match &self.clipboard_content {
                ClipboardContent::Text(s) | ClipboardContent::FileList(s) => {
                    fd.write_all(s.as_bytes()).unwrap();
                }
                ClipboardContent::None => {}
            }
        } else if self
            .drag_source
            .as_ref()
            .map(smithay_client_toolkit::data_device_manager::data_source::DragSource::inner)
            == Some(source)
        {
            if let Some(drag_content) = &self.drag_content {
                fd.write_all(drag_content.as_bytes()).unwrap();
            }
        }
    }

    fn cancelled(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, source: &WlDataSource) {
        debug!("DataSourceHandler::cancelled");
        if self
            .copy_paste_source
            .as_ref()
            .map(smithay_client_toolkit::data_device_manager::data_source::CopyPasteSource::inner)
            == Some(source)
        {
            self.clipboard_content = ClipboardContent::None;
            self.copy_paste_source = None;
        } else if self
            .drag_source
            .as_ref()
            .map(smithay_client_toolkit::data_device_manager::data_source::DragSource::inner)
            == Some(source)
        {
            self.drag_content = None;
            self.drag_source = None;
        }
    }

    fn dnd_dropped(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _source: &WlDataSource) {
        debug!("DataSourceHandler::dnd_dropped");
    }

    fn dnd_finished(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _source: &WlDataSource) {
        debug!("DataSourceHandler::dnd_finished");
    }

    fn action(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _source: &WlDataSource, action: DndAction) {
        debug!("DataSourceHandler::action: {action:?}");
    }
}

pub enum ClipboardContent {
    None,
    Text(String),
    FileList(String),
}

impl ClipboardContent {
    pub fn new(d: ClipboardDataFFI) -> anyhow::Result<Self> {
        match d {
            ClipboardDataFFI::None => Ok(Self::None),
            ClipboardDataFFI::Text(s) => Ok(Self::Text(s.as_str()?.to_owned())),
            ClipboardDataFFI::FileList(s) => Ok(Self::FileList(s.as_str()?.to_owned())),
        }
    }

    pub fn mime_types(&self) -> Vec<&'static str> {
        match self {
            Self::None => Vec::new(),
            Self::Text(_) => vec![TEXT_MIME_TYPE],
            Self::FileList(_) => vec![TEXT_MIME_TYPE, URI_LIST_MIME_TYPE],
        }
    }
}
