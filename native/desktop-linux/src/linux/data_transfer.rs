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
        data_device::{DataDeviceData, DataDeviceHandler},
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
    application_api::{DataSource, DragAndDropAction, DragAndDropActions, DragAndDropQueryData},
    application_state::ApplicationState,
    events::{
        DataTransferAvailableEvent, DataTransferCancelledEvent, DataTransferContent, DragAndDropFinishedEvent, DragAndDropLeaveEvent,
        DropPerformedEvent, WindowId,
    },
};

delegate_data_device!(ApplicationState);
delegate_primary_selection!(ApplicationState);

impl From<DragAndDropAction> for DndAction {
    fn from(value: DragAndDropAction) -> Self {
        match value {
            DragAndDropAction::None => Self::None,
            DragAndDropAction::Copy => Self::Copy,
            DragAndDropAction::Move => Self::Move,
        }
    }
}

impl From<DragAndDropActions> for DndAction {
    fn from(value: DragAndDropActions) -> Self {
        Self::from_bits_truncate(value.0.into())
    }
}

impl From<DndAction> for DragAndDropAction {
    fn from(value: DndAction) -> Self {
        match value {
            DndAction::None => Self::None,
            DndAction::Copy => Self::Copy,
            DndAction::Move => Self::Move,
            DndAction::Ask => Self::Copy, // TODO
            _ => Self::None,
        }
    }
}

struct DragOfferMimetypeAndActions {
    pub mime_type: Option<String>,
    pub supported_actions: DndAction,
    pub preferred_action: DndAction,
}

impl ApplicationState {
    #[must_use]
    fn get_drag_offer_actions(&self, drag_offer: &DragOffer, x: f64, y: f64, window_id: WindowId) -> DragOfferMimetypeAndActions {
        drag_offer.with_mime_types(|mime_types| {
            debug!("Drop handler: {x}x{y}, mime_types={mime_types:?}");

            let drag_and_drop_query_data = DragAndDropQueryData {
                window_id,
                location_in_window: (x, y).into(),
            };
            let target_info = (self.callbacks.query_drag_and_drop_target)(&drag_and_drop_query_data);

            let mime_type = target_info
                .supported_mime_types
                .as_str()
                .unwrap()
                .split(',')
                .find(|supported_mime_type| mime_types.iter().any(|s| s == supported_mime_type))
                .map(str::to_owned);

            debug!("query_drag_and_drop_target -> {target_info:?}, mime_type={mime_type:?}");

            DragOfferMimetypeAndActions {
                mime_type,
                supported_actions: target_info.supported_actions.into(),
                preferred_action: target_info.preferred_action.into(),
            }
        })
    }

    #[must_use]
    fn on_drag_enter_or_move(&self, data_device: &WlDataDevice, x: f64, y: f64, wl_surface: Option<&WlSurface>) -> Option<WindowId> {
        let drag_offer = data_device.data::<DataDeviceData>().and_then(DataDeviceData::drag_offer)?;
        let surface = wl_surface.unwrap_or(&drag_offer.surface);
        let Some(window_id) = self.get_window_id(surface) else {
            warn!("Drop handler: couldn't find window for {surface:?}");
            return None;
        };
        let mime_type_and_actions = self.get_drag_offer_actions(&drag_offer, x, y, window_id);
        drag_offer.set_actions(mime_type_and_actions.supported_actions, mime_type_and_actions.preferred_action);
        drag_offer.accept_mime_type(0, mime_type_and_actions.mime_type); // TODO?: proper serial
        Some(window_id)
    }
}

impl DataDeviceHandler for ApplicationState {
    fn enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: &WlDataDevice, x: f64, y: f64, wl_surface: &WlSurface) {
        debug!("DataDeviceHandler::enter: {}, {x}x{y}", wl_surface.id());
        self.current_drag_target_window_id = self.on_drag_enter_or_move(data_device, x, y, Some(wl_surface));
    }

    fn motion(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: &WlDataDevice, x: f64, y: f64) {
        debug!("DataDeviceHandler::motion: {x}x{y}");
        self.current_drag_target_window_id = self.on_drag_enter_or_move(data_device, x, y, None);
    }

    fn leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice) {
        debug!("DataDeviceHandler::leave");
        // DataDeviceData::drag_offer is always None here

        if let Some(window_id) = self.current_drag_target_window_id.take() {
            self.send_event(DragAndDropLeaveEvent { window_id });
        }
    }

    fn selection(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: &WlDataDevice) {
        let Some(selection_offer) = data_device.data::<DataDeviceData>().and_then(DataDeviceData::selection_offer) else {
            return;
        };
        selection_offer.with_mime_types(|mime_types| {
            debug!("DataDeviceHandler::selection: mime_types={mime_types:?}");
            let mime_types = CString::new(mime_types.join(",")).unwrap();
            self.send_event(DataTransferAvailableEvent::new(&mime_types));
        });
    }

    fn drop_performed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: &WlDataDevice) {
        self.current_drag_target_window_id = None;

        let Some(drag_offer) = data_device.data::<DataDeviceData>().and_then(DataDeviceData::drag_offer) else {
            debug!("DataDeviceHandler::drop_performed: no drag offer");
            return;
        };
        let x = drag_offer.x;
        let y = drag_offer.y;
        let surface = &drag_offer.surface;
        let action = drag_offer.selected_action;

        let Some(window_id) = self.get_window_id(surface) else {
            warn!("Drop handler: couldn't find window for {surface:?}");
            return;
        };
        let mime_type_and_actions = self.get_drag_offer_actions(&drag_offer, x, y, window_id);
        let Some(mime_type) = mime_type_and_actions.mime_type else {
            debug!("Drop handler: no matching MIME type");
            self.send_event(DragAndDropLeaveEvent { window_id });
            return;
        };

        let mime_type_cstr = CString::from_str(&mime_type).unwrap();

        let read_pipe = drag_offer.receive(mime_type).unwrap();
        self.loop_handle
            .insert_source(read_pipe, move |(), res, state| {
                let Some(window_id) = state.get_window_id(&drag_offer.surface) else {
                    warn!(
                        "DataDeviceHandler::drop_performed: couldn't find window for {:?}",
                        drag_offer.surface
                    );
                    state.send_event(DragAndDropLeaveEvent { window_id });
                    return PostAction::Remove;
                };
                let f = unsafe { res.get_mut() };
                let mut buf = Vec::new();
                let size = f.read_to_end(&mut buf).unwrap();
                debug!("DataDeviceHandler::drop_performed read {size} bytes, value: {buf:?}");
                let content = DataTransferContent::new(&buf, &mime_type_cstr);
                drag_offer.finish();
                drag_offer.destroy();
                state.send_event(DropPerformedEvent {
                    window_id,
                    content,
                    action: action.into(),
                });

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
            self.current_drag_source_window_id = None;
            self.current_drag_source_action = None;
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
    }

    fn dnd_finished(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _source: &WlDataSource) {
        debug!("DataSourceHandler::dnd_finished");
        let window_id = self.current_drag_source_window_id.take().unwrap();
        let action = self.current_drag_source_action.take().unwrap();
        self.send_event(DragAndDropFinishedEvent {
            window_id,
            action: action.into(),
        });
    }

    fn action(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _source: &WlDataSource, action: DndAction) {
        debug!("DataSourceHandler::action: {action:?}");
        self.current_drag_source_action = Some(action);
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
