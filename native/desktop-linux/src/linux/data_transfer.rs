use std::{
    ffi::CString,
    io::{Read, Write},
    str::FromStr,
};

use desktop_common::ffi_utils::BorrowedStrPtr;
use log::{debug, error, warn};
use smithay_client_toolkit::{
    data_device_manager::{
        self, WritePipe,
        data_device::DataDeviceHandler,
        data_offer::{DataOfferHandler, DragOffer},
        data_source::{CopyPasteSource, DataSourceHandler, DragSource},
    },
    delegate_data_device, delegate_primary_selection,
    primary_selection::{device::PrimarySelectionDeviceHandler, selection::PrimarySelectionSourceHandler},
    reexports::{
        calloop::PostAction,
        client::{
            Connection, Proxy, QueueHandle,
            protocol::{
                wl_data_device::WlDataDevice, wl_data_device_manager::DndAction, wl_data_source::WlDataSource, wl_surface::WlSurface,
            },
        },
        protocols::wp::primary_selection::zv1::client::{
            zwp_primary_selection_device_v1::ZwpPrimarySelectionDeviceV1, zwp_primary_selection_source_v1::ZwpPrimarySelectionSourceV1,
        },
    },
};

use crate::linux::{
    application_api::{DataSource, DragAndDropQueryData},
    application_state::ApplicationState,
    events::{DataTransferAvailableEvent, DataTransferCancelledEvent, DataTransferContent, DropPerformedEvent},
};

delegate_data_device!(ApplicationState);
delegate_primary_selection!(ApplicationState);

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
        let Some(window_id) = self.get_window_id(&drag_offer.surface) else {
            warn!("DataDeviceHandler::motion: couldn't find window for {:?}", drag_offer.surface);
            return;
        };

        self.drag_destination_mime_type = drag_offer.with_mime_types(|mime_types| {
            let drag_and_drop_query_data = DragAndDropQueryData {
                window_id,
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
            debug!("DataDeviceHandler::drop_performed: destination MIME type not set");
            return;
        };
        let read_pipe = drag_offer.receive(mime_type.clone()).unwrap();
        self.loop_handle
            .insert_source(read_pipe, move |(), res, state| {
                let Some(window_id) = state.get_window_id(&drag_offer.surface) else {
                    warn!(
                        "DataDeviceHandler::drop_performed: couldn't find window for {:?}",
                        drag_offer.surface
                    );
                    return PostAction::Remove;
                };
                let f = unsafe { res.get_mut() };
                let mut buf = Vec::new();
                let size = f.read_to_end(&mut buf).unwrap();
                debug!("DataDeviceHandler::drop_performed read {size} bytes for {mime_type}");
                debug!("DataDeviceHandler::drop_performed value: {buf:?}");
                let mime_type_cstr = CString::from_str(&mime_type).unwrap();
                let content = DataTransferContent::new(&buf, &mime_type_cstr);
                state.send_event(DropPerformedEvent { window_id, content });

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

impl PrimarySelectionDeviceHandler for ApplicationState {
    fn selection(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _primary_selection_device: &ZwpPrimarySelectionDeviceV1) {
        debug!("PrimarySelectionDeviceHandler::selection");
    }
}

impl PrimarySelectionSourceHandler for ApplicationState {
    fn send_request(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &ZwpPrimarySelectionSourceV1,
        mime: String,
        mut write_pipe: data_device_manager::WritePipe,
    ) {
        debug!("PrimarySelectionSourceHandler::send_request: mime={mime}");
        let mime_cstr = CString::new(mime).unwrap();
        let data = (self.callbacks.get_data_transfer_data)(DataSource::PrimarySelection, BorrowedStrPtr::new(&mime_cstr));
        match data.as_slice() {
            Ok(slice) => {
                write_pipe.write_all(slice).expect("Write to data source failed");
                data.deinit();
            }
            Err(e) => error!("Error sending clipboard data: {e}"),
        }
    }

    fn cancelled(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, source: &ZwpPrimarySelectionSourceV1) {
        debug!("PrimarySelectionSourceHandler::cancelled");
        self.primary_selection_source.take();
        source.destroy();
    }
}
