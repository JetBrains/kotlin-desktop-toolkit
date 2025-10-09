use std::{
    ffi::CString,
    io::{Read, Write},
};

use desktop_common::ffi_utils::BorrowedStrPtr;
use log::{debug, error, warn};
use smithay_client_toolkit::{
    data_device_manager::{
        self,
        WritePipe,
        data_device::DataDeviceHandler,
        data_offer::{DataOfferHandler, DragOffer},
        data_source::{CopyPasteSource, DataSourceHandler, DragSource},
    },
    delegate_data_device,
    reexports::{
        calloop::{LoopHandle, PostAction},
        client::{
            Connection, Proxy, QueueHandle,
            protocol::{
                wl_data_device::WlDataDevice, wl_data_device_manager::DndAction, wl_data_source::WlDataSource, wl_surface::WlSurface,
            },
        },
    },
};

use crate::linux::{
    application_api::{DataSource, DragAndDropQueryData},
    application_state::ApplicationState,
    events::{DataTransferAvailableEvent, DataTransferCancelledEvent, DataTransferContent, DataTransferEvent},
};

delegate_data_device!(ApplicationState);

#[must_use]
pub fn read_from_pipe<'l, F>(
    f_name: &'static str,
    read_pipe: data_device_manager::ReadPipe,
    mime_type: String,
    loop_handle: &LoopHandle<'l, ApplicationState>,
    mut callback: F,
) -> bool
where
    for<'a> F: FnMut(&mut ApplicationState, DataTransferContent<'a>) + 'l,
{
    let mime_type_cstr = CString::new(mime_type).unwrap();
    if let Err(e) = loop_handle.insert_source(read_pipe, move |(), res, state| {
        let f = unsafe { res.get_mut() };
        let mut buf = Vec::new();
        let content = match f.read_to_end(&mut buf) {
            Ok(size) => {
                debug!("{f_name}: read {size} bytes");
                DataTransferContent::new(&buf, &mime_type_cstr)
            }
            Err(e) => {
                warn!("{f_name}: error receiving data: {e}");
                DataTransferContent::null()
            }
        };

        callback(state, content);
        PostAction::Remove
    }) {
        warn!("{f_name}: failed to start reading: {e}");
        false
    } else {
        true
    }
}

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

        self.drag_destination_mime_type = drag_offer.with_mime_types(|mime_types| {
            let drag_and_drop_query_data = DragAndDropQueryData {
                window_id: self.get_window(&drag_offer.surface).unwrap().window_id,
                point: (x, y).into(),
            };
            let supported_mime_types = (self.callbacks.get_drag_and_drop_supported_mime_types)(&drag_and_drop_query_data);

            supported_mime_types
                .as_str()
                .unwrap()
                .split(',')
                .find(|supported_mime_type| mime_types.iter().any(|s| s == supported_mime_type))
                .map(str::to_owned)
        });
        drag_offer.accept_mime_type(0, self.drag_destination_mime_type.clone());
        if self.drag_destination_mime_type.is_some() {
            // Accept the action
            drag_offer.set_actions(DndAction::Copy, DndAction::Copy);
        }
    }

    fn selection(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, wl_data_device: &WlDataDevice) {
        let Some(data_device) = &self.data_device else { return };
        if data_device.inner() != wl_data_device {
            return;
        }
        let Some(selection_offer) = data_device.data().selection_offer() else {
            return;
        };
        selection_offer.with_mime_types(|mime_types| {
            debug!("DataDeviceHandler::selection: mime_types={mime_types:?}");
            let mime_types = CString::new(mime_types.join(",")).unwrap();
            self.send_event(DataTransferAvailableEvent::new(&mime_types));
        });
    }

    fn drop_performed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice) {
        debug!("DataDeviceHandler::drop_performed");

        let Some(data_device) = &self.data_device else {
            debug!("DataDeviceHandler::drop_performed: no data device");
            return;
        };
        let Some(drag_offer) = data_device.data().drag_offer() else {
            debug!("DataDeviceHandler::drop_performed: no drag offer");
            return;
        };

        let Some(mime_type) = self.drag_destination_mime_type.take() else {
            debug!("DataDeviceHandler::drop_performed: no matching MIME type");
            self.send_event(DataTransferEvent { serial: -1, content: DataTransferContent::null() });
            return;
        };

        let read_pipe = match drag_offer.receive(mime_type.clone()) {
            Ok(v) => v,
            Err(e) => {
                warn!("DataDeviceHandler::drop_performed: failed receiving data offer: {e}");
                self.send_event(DataTransferEvent { serial: -1, content: DataTransferContent::null() });
                return;
            }
        };

        if !read_from_pipe(
            "DataDeviceHandler::drop_performed",
            read_pipe,
            mime_type,
            &self.loop_handle,
            move |state, content| {
                drag_offer.finish();
                drag_offer.destroy();
                state.send_event(DataTransferEvent { serial: -1, content });
            },
        ) {
            self.send_event(DataTransferEvent { serial: -1, content: DataTransferContent::null() });
        }
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
        let data_type = if self.copy_paste_source.as_ref().map(CopyPasteSource::inner) == Some(source) {
            DataSource::Clipboard
        } else if self.drag_source.as_ref().map(DragSource::inner) == Some(source) {
            DataSource::DragAndDrop
        } else {
            return;
        };
        let mime_cstr = CString::new(mime).unwrap();
        let data = (self.callbacks.get_data_transfer_data)(data_type, BorrowedStrPtr::new(&mime_cstr));
        match data.as_slice() {
            Ok(slice) => {
                fd.write_all(slice).expect("Write to data source failed");
                data.deinit();
            }
            Err(e) => error!("Error sending clipboard data: {e}"),
        }
    }

    fn cancelled(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, source: &WlDataSource) {
        debug!("DataSourceHandler::cancelled");
        let data_source = if self.copy_paste_source.as_ref().map(CopyPasteSource::inner) == Some(source) {
            self.copy_paste_source = None;
            Some(DataSource::Clipboard)
        } else if self.drag_source.as_ref().map(DragSource::inner) == Some(source) {
            self.drag_source = None;
            Some(DataSource::DragAndDrop)
        } else {
            None
        };
        if let Some(data_source) = data_source {
            self.send_event(DataTransferCancelledEvent { data_source });
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
