use crate::gtk::application::send_event;
use crate::gtk::application_api::{DragAndDropAction, DragAndDropActions, DragAndDropQueryData, QueryDragAndDropTarget};
use crate::gtk::events::{DataTransferContent, DragAndDropLeaveEvent, DropPerformedEvent, EventHandler, WindowId};
use gtk4::gio::prelude::InputStreamExtManual;
use gtk4::prelude::{IsA, WidgetExt};
use gtk4::{gdk as gdk4, gio, glib};
use log::{debug, warn};
use std::ffi::CString;

#[derive(Debug)]
pub struct DragOfferMimetypeAndActions {
    pub mime_type: Option<String>,
    pub supported_actions: gdk4::DragAction,
    pub preferred_action: gdk4::DragAction,
}

impl From<DragAndDropAction> for gdk4::DragAction {
    fn from(value: DragAndDropAction) -> Self {
        match value {
            DragAndDropAction::None => Self::empty(),
            DragAndDropAction::Copy => Self::COPY,
            DragAndDropAction::Move => Self::MOVE,
        }
    }
}

impl From<DragAndDropActions> for gdk4::DragAction {
    fn from(value: DragAndDropActions) -> Self {
        Self::from_bits_truncate(value.0.into())
    }
}

impl From<gdk4::DragAction> for DragAndDropAction {
    fn from(value: gdk4::DragAction) -> Self {
        match value {
            gdk4::DragAction::COPY => Self::Copy,
            gdk4::DragAction::MOVE => Self::Move,
            gdk4::DragAction::LINK => Self::Copy, // TODO
            gdk4::DragAction::ASK => Self::Copy,  // TODO
            _ => Self::None,
        }
    }
}

pub fn read_all(input_stream: &gio::InputStream, callback: impl FnOnce(Option<Vec<u8>>) + 'static) {
    debug!("read_all start");
    let buf_all: Vec<u8> = Vec::new();
    let input_stream_clone = input_stream.clone();
    let f = |res: Result<(Vec<u8>, usize), (Vec<u8>, glib::Error)>| {
        fn helper(
            res: Result<(Vec<u8>, usize), (Vec<u8>, glib::Error)>,
            input_stream: gio::InputStream,
            mut buf_all: Vec<u8>,
            callback: impl FnOnce(Option<Vec<u8>>) + 'static,
        ) {
            match res {
                Ok((_chunk_buf, 0)) => {
                    debug!("read_all end: size={}", buf_all.len());
                    callback(Some(buf_all));
                }
                Ok((chunk_buf, size)) => {
                    debug!("read_all loop: size={size}");
                    buf_all.extend_from_slice(&chunk_buf[0..size]);
                    input_stream
                        .clone()
                        .read_async(vec![0; 4096], glib::Priority::DEFAULT, gio::Cancellable::NONE, move |res| {
                            helper(res, input_stream, buf_all, callback);
                        });
                }
                Err((_chunk_buf, e)) => {
                    warn!("application_clipboard_paste: error receiving data: {e}");
                    callback(None);
                }
            }
        }
        helper(res, input_stream_clone, buf_all, callback);
    };
    input_stream.read_async(vec![0; 4096], glib::Priority::DEFAULT, gio::Cancellable::NONE, f);
}

#[must_use]
pub fn get_drag_offer_actions(
    query_drag_and_drop_target: QueryDragAndDropTarget,
    drop: &gdk4::Drop,
    x: f64,
    y: f64,
    window_id: WindowId,
) -> DragOfferMimetypeAndActions {
    let mime_types = drop.formats().mime_types();
    debug!("Drop handler: x={x}, y={y}, mime_types={mime_types:?}");

    let drag_and_drop_query_data = DragAndDropQueryData {
        window_id,
        location_in_window: (x, y).into(),
    };
    let target_info = query_drag_and_drop_target(&drag_and_drop_query_data);

    let supported_mime_with_actions = target_info
        .supported_actions_per_mime
        .as_slice()
        .unwrap()
        .iter()
        .find(|&e| mime_types.iter().any(|s| s == e.supported_mime_type.as_str().unwrap()));

    debug!("query_drag_and_drop_target -> {target_info:?}, supported_mime_with_actions={supported_mime_with_actions:?}");

    if let Some(v) = supported_mime_with_actions {
        DragOfferMimetypeAndActions {
            mime_type: Some(v.supported_mime_type.as_str().unwrap().to_owned()),
            supported_actions: v.supported_actions.into(),
            preferred_action: v.preferred_action.into(),
        }
    } else {
        DragOfferMimetypeAndActions {
            mime_type: None,
            supported_actions: gdk4::DragAction::empty(),
            preferred_action: gdk4::DragAction::empty(),
        }
    }
}

fn get_best_dnd_action(mime_type_and_actions: &DragOfferMimetypeAndActions, available_actions: gdk4::DragAction) -> gdk4::DragAction {
    if available_actions.contains(mime_type_and_actions.preferred_action) {
        mime_type_and_actions.preferred_action
    } else {
        available_actions
            .iter()
            .find(|&action| mime_type_and_actions.supported_actions.contains(action))
            .unwrap_or_else(gdk4::DragAction::empty)
    }
}

pub fn handle_drop_start(
    event_handler: EventHandler,
    window_id: WindowId,
    query_drag_and_drop_target: QueryDragAndDropTarget,
    drop: &gdk4::Drop,
    x: f64,
    y: f64,
) -> bool {
    let mime_type_and_actions = get_drag_offer_actions(query_drag_and_drop_target, drop, x, y, window_id);
    dbg!(&mime_type_and_actions);
    let Some(mime_type) = mime_type_and_actions.mime_type.as_ref() else {
        debug!("DropStart: no matching MIME type");
        drop.finish(gdk4::DragAction::empty());
        send_event(
            event_handler,
            DropPerformedEvent {
                window_id,
                content: DataTransferContent::null(),
                action: DragAndDropAction::None,
            },
        );
        return false;
    };

    drop.status(mime_type_and_actions.supported_actions, mime_type_and_actions.preferred_action);

    let action = get_best_dnd_action(&mime_type_and_actions, drop.actions());
    dbg!(action);
    let drop_clone = drop.clone();
    drop.read_async(
        &[mime_type],
        glib::Priority::DEFAULT,
        gio::Cancellable::NONE,
        move |res| match res {
            Ok((input_stream, mime_type)) => {
                read_all(&input_stream, move |data| {
                    drop_clone.finish(action);
                    if let Some(data) = data {
                        let mime_type_cstr = CString::new(mime_type.as_str()).unwrap();
                        let content = DataTransferContent::new(&mime_type_cstr, &data);
                        send_event(
                            event_handler,
                            DropPerformedEvent {
                                window_id,
                                content,
                                action: action.into(),
                            },
                        );
                    } else {
                        send_event(
                            event_handler,
                            DropPerformedEvent {
                                window_id,
                                content: DataTransferContent::null(),
                                action: DragAndDropAction::None,
                            },
                        );
                    }
                });
            }
            Err(e) => {
                drop_clone.finish(drop_clone.actions());
                warn!("DropStart: failed receiving data offer: {e}");
                send_event(
                    event_handler,
                    DropPerformedEvent {
                        window_id,
                        content: DataTransferContent::null(),
                        action: DragAndDropAction::None,
                    },
                );
            }
        },
    );
    true
}

pub fn set_drag_and_drop_event_handlers(
    widget: &impl IsA<gtk4::Widget>,
    window_id: WindowId,
    event_handler: EventHandler,
    query_drag_and_drop_target: QueryDragAndDropTarget,
) {
    let drop_target = gtk4::DropTargetAsync::new(None, gdk4::DragAction::COPY);
    drop_target.connect_accept(move |drop_target, drop| {
        // let mime_types = drop.formats().mime_types();
        // debug!("drop_target_accept: {mime_types:?}");
        drop_target.set_formats(Some(&drop.formats()));
        drop_target.set_actions(drop.actions());
        true
    });

    // Don't use "drag-enter" because it reports wrong coordinates (0, 0)
    // https://github.com/GNOME/gtk/blob/9d31fd6429e8287766094b8ebaf4d102c2b851ec/gdk/gdkdrop.c#L943

    drop_target.connect_drag_motion(move |_drop_target, drop, x, y| {
        // debug!("DropTarget::drag_motion");
        let mime_type_and_actions = get_drag_offer_actions(query_drag_and_drop_target, drop, x, y, window_id);
        drop.status(mime_type_and_actions.supported_actions, mime_type_and_actions.preferred_action);
        mime_type_and_actions.preferred_action
    });
    drop_target.connect_drag_leave(move |_drop_target, _drop| {
        send_event(event_handler, DragAndDropLeaveEvent { window_id });
    });
    drop_target.connect_drop(move |_drop_target, drop, x, y| {
        debug!("DropTarget::drop");
        handle_drop_start(event_handler, window_id, query_drag_and_drop_target, drop, x, y)
    });
    widget.add_controller(drop_target);
}
