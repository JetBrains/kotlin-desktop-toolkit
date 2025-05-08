use super::application_state::ApplicationState;
use crate::linux::application_api::{DataSource, DragAndDropQueryData};
use crate::linux::events::DataWithMimeFFI;
use crate::linux::geometry::{LogicalPixels, LogicalPoint};
use desktop_common::ffi_utils::BorrowedStrPtr;
use log::{debug, warn};
use smithay_client_toolkit::data_device_manager::data_source::{CopyPasteSource, DragSource};
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
use std::ffi::CString;
use std::io::{Read, Write};
use std::str::FromStr;

delegate_data_device!(ApplicationState);

impl DataDeviceHandler for ApplicationState {
    fn enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice, x: f64, y: f64, wl_surface: &WlSurface) {
        debug!("DataDeviceHandler::enter: {}, {x}x{y}", wl_surface.id());
    }

    fn leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice) {
        debug!("DataDeviceHandler::leave");
        self.drag_destination_mime_type = None;
    }

    fn motion(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice, x: f64, y: f64) {
        debug!("DataDeviceHandler::motion: {x}x{y}");

        let Some(data_device) = &self.data_device else {
            return;
        };
        let Some(drag_offer) = data_device.data().drag_offer() else {
            return;
        };

        drag_offer.with_mime_types(|mime_types| {
            let drag_and_drop_query_data = DragAndDropQueryData {
                window_id: self.get_window(&drag_offer.surface).unwrap().window_id,
                point: LogicalPoint {
                    x: LogicalPixels(x),
                    y: LogicalPixels(y),
                },
            };
            let supported_mime_types = (self.callbacks.drag_and_drop_query_handler)(&drag_and_drop_query_data);

            for supported_mime_type in supported_mime_types.as_str().unwrap().split(',') {
                if mime_types.iter().any(|s| s == supported_mime_type) {
                    debug!("DataDeviceHandler::motion: accepted mime_type={supported_mime_type}");
                    drag_offer.accept_mime_type(0, Some(supported_mime_type.to_owned()));
                    // Accept the action now just in case
                    drag_offer.set_actions(DndAction::Copy, DndAction::Copy);
                    return;
                }
            }
            debug!("DataDeviceHandler::motion: didn't accept any of the mime_types={mime_types:?}");
        });
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

        let Some(mime_type) = self.drag_destination_mime_type.take() else {
            return;
        };
        let read_pipe = drag_offer.receive(mime_type.clone()).unwrap();
        self.loop_handle
            .insert_source(read_pipe, move |(), res, state| {
                let f = unsafe { res.get_mut() };
                let mut buf = Vec::new();
                let size = f.read_to_end(&mut buf).unwrap();

                debug!("DataDeviceHandler::drop_performed read {size} bytes");
                debug!("DataDeviceHandler::drop_performed value: {buf:?}");
                if let Some(key_window) = state.get_window(&drag_offer.surface) {
                    let mime_type_cstr = CString::from_str(&mime_type).unwrap();
                    (key_window.event_handler)(&DataWithMimeFFI::new(&buf, &mime_type_cstr).into());
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
        //
        // if let Some(clipboard_content) = self.clipboard_content.as_ref().map(|c| c.data.clone()) {
        //     self.loop_handle
        //         .insert_source(fd,move |(), res, _state| {
        //             let f = unsafe { res.get_mut() };
        //             f.write_all(&clipboard_content).unwrap();
        //             PostAction::Remove
        //         })
        //         .unwrap();
        // }
        let data_type = if self.copy_paste_source.as_ref().map(CopyPasteSource::inner) == Some(source) {
            DataSource::Clipboard
        } else if self.drag_source.as_ref().map(DragSource::inner) == Some(source) {
            DataSource::DragAndDrop
        } else {
            return;
        };
        let mime_cstr = CString::new(mime).unwrap();
        let data = (self.callbacks.get_data_source_data)(data_type, BorrowedStrPtr::new(&mime_cstr));
        fd.write_all(data.as_slice().unwrap()).unwrap();
        data.deinit();
    }

    fn cancelled(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, source: &WlDataSource) {
        debug!("DataSourceHandler::cancelled");
        if self.copy_paste_source.as_ref().map(CopyPasteSource::inner) == Some(source) {
            self.copy_paste_source = None;
        } else if self.drag_source.as_ref().map(DragSource::inner) == Some(source) {
            self.drag_source = None;
        }
    }

    fn dnd_dropped(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _source: &WlDataSource) {
        debug!("DataSourceHandler::dnd_dropped");
        self.drag_destination_mime_type = None;
    }

    fn dnd_finished(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _source: &WlDataSource) {
        debug!("DataSourceHandler::dnd_finished");
        self.drag_destination_mime_type = None;
    }

    fn action(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _source: &WlDataSource, action: DndAction) {
        debug!("DataSourceHandler::action: {action:?}");
    }
}

pub struct MimeTypes {
    pub val: Vec<String>,
}

impl MimeTypes {
    pub fn new(mime_types_str: &str) -> Self {
        Self {
            val: mime_types_str.split(',').map(str::to_owned).collect(),
        }
    }
}

pub struct ClipboardContent {
    pub data: Vec<u8>,
    pub mime_types: Vec<String>,
}

impl ClipboardContent {
    pub(crate) fn new(data: &[u8], mime_types_str: &str) -> Self {
        Self {
            data: data.to_owned(),
            mime_types: mime_types_str.split(',').map(str::to_owned).collect(),
        }
    }
}
