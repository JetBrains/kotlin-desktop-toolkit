use super::application_state::ApplicationState;
use crate::linux::events::ClipboardDataFFI;
use log::debug;
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
use std::io::Write;

pub const TEXT_MIME_TYPE: &str = "text/plain;charset=utf-8";
pub const URI_LIST_MIME_TYPE: &str = "text/uri-list";

delegate_data_device!(ApplicationState);

impl DataDeviceHandler for ApplicationState {
    fn enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice, x: f64, y: f64, wl_surface: &WlSurface) {
        debug!("DataDeviceHandler::enter: {}, {x}x{y}", wl_surface.id());
    }

    fn leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice) {
        debug!("DataDeviceHandler::leave");
    }

    fn motion(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice, x: f64, y: f64) {
        debug!("DataDeviceHandler::motion: {x}x{y}");
    }

    fn selection(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice) {
        debug!("DataDeviceHandler::selection");
    }

    fn drop_performed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice) {
        debug!("DataDeviceHandler::drop_performed");
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

        if self.copy_paste_source.as_ref().map(|s| s.inner()) == Some(source) {
            match &self.clipboard_content {
                ClipboardContent::Text(s) | ClipboardContent::FileList(s) => {
                    fd.write_all(s.as_bytes()).unwrap();
                }
                ClipboardContent::None => {}
            }
        } else if self.drag_source.as_ref().map(|s| s.inner()) == Some(source) {
            if let Some(drag_content) = &self.drag_content {
                fd.write_all(drag_content.as_bytes()).unwrap();
            }
        }
    }

    fn cancelled(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, source: &WlDataSource) {
        debug!("DataSourceHandler::cancelled");
        if self.copy_paste_source.as_ref().map(|s| s.inner()) == Some(source) {
            self.clipboard_content = ClipboardContent::None;
            self.copy_paste_source = None;
        } else if self.drag_source.as_ref().map(|s| s.inner()) == Some(source) {
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
