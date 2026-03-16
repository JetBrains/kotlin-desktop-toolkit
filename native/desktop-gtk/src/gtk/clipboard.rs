use crate::gtk::application::send_event;
use crate::gtk::application_api::DataSource;
use crate::gtk::data_transfer::read_all;
use crate::gtk::events::{DataTransferAvailableEvent, DataTransferContent, DataTransferEvent, EventHandler};
use crate::gtk::ffi_return_conversions::TransferDataGetter;
use crate::gtk::mime_types::MimeTypes;
use gdk4::prelude::{ObjectExt, OutputStreamExtManual};
use gdk4::subclass::content_provider::ContentProviderImplExt;
use gdk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{gdk as gdk4, gio, glib};
use log::{debug, warn};
use std::cell::OnceCell;
use std::ffi::CString;
use std::pin::Pin;

#[derive(Default)]
pub struct ClipboardContentProviderImpl {
    pub formats: OnceCell<gdk4::ContentFormats>,
    pub clipboard_type: OnceCell<DataSource>,
    pub transfer_data_getter: OnceCell<TransferDataGetter>,
}

#[glib::object_subclass]
impl gdk4::subclass::prelude::ObjectSubclass for ClipboardContentProviderImpl {
    /// cbindgen:ignore
    const NAME: &'static str = "ClipboardContentProvider";
    type Type = ClipboardContentProvider;
    type ParentType = gdk4::ContentProvider;

    fn new() -> Self {
        Self::default()
    }
}

impl gdk4::subclass::prelude::ObjectImpl for ClipboardContentProviderImpl {}
impl gdk4::subclass::content_provider::ContentProviderImpl for ClipboardContentProviderImpl {
    fn content_changed(&self) {
        debug!("ContentProviderImpl::content_changed");
        self.parent_content_changed();
    }

    fn attach_clipboard(&self, clipboard: &gdk4::Clipboard) {
        debug!("ContentProviderImpl::attach_clipboard");
        self.parent_attach_clipboard(clipboard);
    }

    fn detach_clipboard(&self, clipboard: &gdk4::Clipboard) {
        debug!("ContentProviderImpl::detach_clipboard");
        self.parent_detach_clipboard(clipboard);
    }

    fn formats(&self) -> gdk4::ContentFormats {
        let ret = self.formats.get().unwrap().clone();
        debug!("ContentProviderImpl::formats -> {:?}", ret.mime_types());
        ret
    }

    fn storable_formats(&self) -> gdk4::ContentFormats {
        let ret = self.formats();
        debug!("ContentProviderImpl::storable_formats -> {:?}", ret.mime_types());
        ret
    }

    fn write_mime_type_future(
        &self,
        mime_type: &str,
        stream: &gio::OutputStream,
        io_priority: glib::Priority,
    ) -> Pin<Box<dyn Future<Output = Result<(), glib::Error>> + 'static>> {
        debug!("ContentProviderImpl::write_mime_type_future: {mime_type}");
        let clipboard_type = *self.clipboard_type.get().unwrap();
        if let Some(data) = self.transfer_data_getter.get().unwrap().get(clipboard_type, mime_type) {
            let stream = stream.clone();
            Box::pin(async move {
                match stream.write_future(data, io_priority).await {
                    Ok((_buf, size)) => {
                        debug!("Finished writing clipboard data, size={size}");
                        Ok(())
                    }
                    Err((_buf, e)) => {
                        warn!("Error writing clipboard data: {e}");
                        Err(e)
                    }
                }
            })
        } else {
            Box::pin(async { Ok(()) })
        }
    }

    fn value(&self, type_: glib::Type) -> Result<glib::Value, glib::Error> {
        let ret = self.parent_value(type_);
        debug!("ContentProviderImpl::value -> {ret:?}");
        ret
    }
}

glib::wrapper! {
    pub struct ClipboardContentProvider(ObjectSubclass<ClipboardContentProviderImpl>)
    @extends gdk4::ContentProvider;
}

impl ClipboardContentProvider {
    pub fn new(mime_types: &MimeTypes, transfer_data_getter: TransferDataGetter, clipboard_type: DataSource) -> Self {
        let obj = glib::Object::new::<Self>();
        let imp = obj.imp();
        imp.clipboard_type.set(clipboard_type).unwrap();
        imp.formats.set(gdk4::ContentFormats::new(&mime_types.val)).unwrap();
        imp.transfer_data_getter.get_or_init(|| transfer_data_getter);
        obj
    }
}

pub struct KdtClipboard {
    gdk_clipboard: gdk4::Clipboard,
    clipboard_type: DataSource,
    connect_changed_handler_id: Option<glib::signal::SignalHandlerId>,
    event_handler: EventHandler,
    transfer_data_getter: TransferDataGetter,
    content_provider: Option<ClipboardContentProvider>,
}

impl Drop for KdtClipboard {
    fn drop(&mut self) {
        if let Some(_content_provider) = self.content_provider.take() {
            self.gdk_clipboard.set_content(gdk4::ContentProvider::NONE).unwrap();
        }
        if let Some(handler_id) = self.connect_changed_handler_id.take() {
            self.gdk_clipboard.disconnect(handler_id);
        }
    }
}

impl KdtClipboard {
    pub fn new(
        gdk_clipboard: gdk4::Clipboard,
        clipboard_type: DataSource,
        event_handler: EventHandler,
        transfer_data_getter: TransferDataGetter,
    ) -> Self {
        let connect_changed_handler_id = gdk_clipboard.connect_changed(move |clipboard| {
            let mime_types = CString::new(clipboard.formats().mime_types().join(",")).unwrap();
            send_event(event_handler, DataTransferAvailableEvent::new(clipboard_type, &mime_types));
        });
        Self {
            gdk_clipboard,
            clipboard_type,
            connect_changed_handler_id: Some(connect_changed_handler_id),
            event_handler,
            transfer_data_getter,
            content_provider: None,
        }
    }

    pub fn paste(&self, serial: i32, mime_types: &MimeTypes) {
        debug!("KdtClipboard::paste start");
        let mime_types = &mime_types.val;
        let event_handler = self.event_handler;

        self.gdk_clipboard
            .read_async(mime_types, glib::Priority::DEFAULT, gio::Cancellable::NONE, move |res| match res {
                Ok((input_stream, mime_type)) => {
                    debug!("KdtClipboard::paste: reading {mime_type}");
                    read_all(&input_stream, move |res| {
                        if let Some(data) = res {
                            let mime_type_cstr = CString::new(mime_type.as_str()).unwrap();
                            let content = DataTransferContent::new(&mime_type_cstr, &data);
                            let event = DataTransferEvent { serial, content };
                            send_event(event_handler, event);
                        } else {
                            let event = DataTransferEvent {
                                serial,
                                content: DataTransferContent::null(),
                            };
                            send_event(event_handler, event);
                        }
                    });
                }
                Err(e) => {
                    warn!("KdtClipboard::paste: clipboard read_async error: {e}");
                    let event = DataTransferEvent {
                        serial,
                        content: DataTransferContent::null(),
                    };
                    send_event(event_handler, event);
                }
            });

        debug!("KdtClipboard::paste end");
    }

    pub fn put(&mut self, mime_types: &MimeTypes) -> anyhow::Result<()> {
        if mime_types.val.is_empty() {
            self.gdk_clipboard.set_content(gdk4::ContentProvider::NONE)?;
            self.content_provider = None;
        } else {
            let _old_content_provider = self.content_provider.take();
            self.content_provider = Some(ClipboardContentProvider::new(
                mime_types,
                self.transfer_data_getter,
                self.clipboard_type,
            ));
            self.gdk_clipboard.set_content(self.content_provider.as_ref())?;
        }
        Ok(())
    }

    pub fn mime_types_csv(&self) -> String {
        self.gdk_clipboard.formats().mime_types().join(",")
    }
}
