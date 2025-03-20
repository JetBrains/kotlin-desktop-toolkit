use log::debug;
use smithay_client_toolkit::{
    delegate_dmabuf,
    dmabuf::{DmabufFeedback, DmabufHandler, DmabufState},
    reexports::{
        client::{Connection, QueueHandle, protocol::wl_buffer::WlBuffer},
        protocols::wp::linux_dmabuf::zv1::client::{
            zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1, zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
        },
    },
};

use super::application_state::ApplicationState;

impl DmabufHandler for ApplicationState {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dma_state
    }

    fn dmabuf_feedback(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _proxy: &ZwpLinuxDmabufFeedbackV1,
        feedback: DmabufFeedback,
    ) {
        debug!("DmabufHandler::dmabuf_feedback: {:?}", feedback.format_table());
    }

    fn created(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _params: &ZwpLinuxBufferParamsV1, _buffer: WlBuffer) {
        debug!("DmabufHandler::created");
    }

    fn failed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _params: &ZwpLinuxBufferParamsV1) {
        debug!("DmabufHandler::failed");
    }

    fn released(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _buffer: &WlBuffer) {
        debug!("DmabufHandler::released");
    }
}

delegate_dmabuf!(ApplicationState);
