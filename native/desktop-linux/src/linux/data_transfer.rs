use std::{
    ffi::CString,
    io::{Read, Write},
};

use desktop_common::ffi_utils::BorrowedStrPtr;
use log::{debug, error, warn};
use smithay_client_toolkit::{
    data_device_manager::{
        self, WritePipe,
        data_device::{DataDeviceData, DataDeviceHandler},
        data_offer::{DataOfferHandler, DragOffer},
        data_source::{CopyPasteSource, DataSourceHandler, DragSource},
    },
    delegate_data_device, delegate_primary_selection,
    primary_selection::{device::PrimarySelectionDeviceHandler, selection::PrimarySelectionSourceHandler},
    reexports::{
        calloop::{LoopHandle, PostAction},
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
    events::{DataTransferAvailableEvent, DataTransferCancelledEvent, DataTransferContent, DragAndDropLeaveEvent, DropPerformedEvent},
};

delegate_data_device!(ApplicationState);
delegate_primary_selection!(ApplicationState);

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

impl ApplicationState {
    fn on_drag_enter_or_move(&mut self, data_device: &WlDataDevice, x: f64, y: f64) {
        let Some(drag_offer) = data_device.data::<DataDeviceData>().and_then(DataDeviceData::drag_offer) else {
            return;
        };
        let Some(window_id) = self.get_window_id(&drag_offer.surface) else {
            warn!("Drop handler: couldn't find window for {:?}", drag_offer.surface);
            return;
        };

        self.drag_destination_mime_type = drag_offer.with_mime_types(|mime_types| {
            debug!("Drop handler: {x}x{y}, mime_types={mime_types:?}");

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
}

impl DataDeviceHandler for ApplicationState {
    fn enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: &WlDataDevice, x: f64, y: f64, wl_surface: &WlSurface) {
        debug!("DataDeviceHandler::enter: {}, {x}x{y}", wl_surface.id());
        self.on_drag_enter_or_move(data_device, x, y);
    }

    fn leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: &WlDataDevice) {
        debug!("DataDeviceHandler::leave");
        self.drag_destination_mime_type = None;
        // TODO: add event
        let Some(drag_offer) = data_device.data::<DataDeviceData>().and_then(DataDeviceData::drag_offer) else {
            debug!("DataDeviceHandler::leave: no drag offer");
            return;
        };
        let Some(window_id) = self.get_window_id(&drag_offer.surface) else {
            warn!("DataDeviceHandler::leave: couldn't find window for {:?}", drag_offer.surface);
            return;
        };
        self.send_event(DragAndDropLeaveEvent { window_id });
    }

    fn motion(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: &WlDataDevice, x: f64, y: f64) {
        self.on_drag_enter_or_move(data_device, x, y);
    }

    fn selection(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: &WlDataDevice) {
        let Some(selection_offer) = data_device.data::<DataDeviceData>().and_then(DataDeviceData::selection_offer) else {
            return;
        };
        selection_offer.with_mime_types(|mime_types| {
            debug!("DataDeviceHandler::selection: mime_types={mime_types:?}");
            let mime_types = CString::new(mime_types.join(",")).unwrap();
            self.send_event(DataTransferAvailableEvent::new(DataSource::Clipboard, &mime_types));
        });
    }

    fn drop_performed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: &WlDataDevice) {
        let Some(drag_offer) = data_device.data::<DataDeviceData>().and_then(DataDeviceData::drag_offer) else {
            debug!("DataDeviceHandler::drop_performed: no drag offer");
            return;
        };

        let surface = &drag_offer.surface;

        let Some(window_id) = self.get_window_id(surface) else {
            warn!("DataDeviceHandler::drop_performed: couldn't find window for {surface:?}");
            return;
        };

        let Some(mime_type) = self.drag_destination_mime_type.take() else {
            debug!("DataDeviceHandler::drop_performed: no matching MIME type");
            self.send_event(DropPerformedEvent {
                window_id,
                content: DataTransferContent::null(),
            });
            return;
        };

        let read_pipe = match drag_offer.receive(mime_type.clone()) {
            Ok(v) => v,
            Err(e) => {
                warn!("DataDeviceHandler::drop_performed: failed receiving data offer: {e}");
                self.send_event(DropPerformedEvent {
                    window_id,
                    content: DataTransferContent::null(),
                });
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
                state.send_event(DropPerformedEvent { window_id, content });
            },
        ) {
            self.send_event(DropPerformedEvent {
                window_id,
                content: DataTransferContent::null(),
            });
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

impl PrimarySelectionDeviceHandler for ApplicationState {
    fn selection(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, primary_selection_device: &ZwpPrimarySelectionDeviceV1) {
        debug!("PrimarySelectionDeviceHandler::selection");
        let Some(selection_offer) = primary_selection_device.data().and_then(DataDeviceData::selection_offer) else {
            return;
        };
        selection_offer.with_mime_types(|mime_types| {
            debug!("PrimarySelectionDeviceHandler::selection: mime_types={mime_types:?}");
            let mime_types = CString::new(mime_types.join(",")).unwrap();
            self.send_event(DataTransferAvailableEvent::new(DataSource::PrimarySelection, &mime_types));
        });
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
        self.primary_selection_source = None;
        source.destroy();
        self.send_event(DataTransferCancelledEvent {
            data_source: DataSource::PrimarySelection,
        });
    }
}
