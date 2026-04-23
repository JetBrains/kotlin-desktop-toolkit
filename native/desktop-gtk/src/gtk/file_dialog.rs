use crate::gtk::file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams};
use anyhow::{Context, anyhow};
use gtk4::gio;
use gtk4::prelude::{Cast, FileChooserExt, FileChooserExtManual, FileExt, ListModelExt, NativeDialogExt, NativeDialogExtManual, ObjectExt};
use log::debug;

impl OpenFileDialogParams {
    const fn get_action(&self) -> gtk4::FileChooserAction {
        if self.select_directories {
            gtk4::FileChooserAction::SelectFolder
        } else {
            gtk4::FileChooserAction::Open
        }
    }

    fn apply(&self, file_chooser: &gtk4::FileChooserNative) {
        file_chooser.set_select_multiple(self.allows_multiple_selection);
    }
}

impl SaveFileDialogParams<'_> {
    fn apply(&self, file_chooser: &gtk4::FileChooserNative) -> anyhow::Result<()> {
        if let Some(name_field_string_value) = self.get_name_field_string_value()? {
            file_chooser.set_current_name(name_field_string_value);
        }
        Ok(())
    }
}

impl CommonFileDialogParams<'_> {
    fn create_file_chooser(&self, action: gtk4::FileChooserAction, parent: &gtk4::Window) -> anyhow::Result<gtk4::FileChooserNative> {
        let file_chooser = gtk4::FileChooserNative::new(self.get_title()?, Some(parent), action, self.get_accept_label()?, None);
        file_chooser.set_modal(self.modal);
        if let Some(current_folder) = self.get_current_folder()? {
            let file = gio::File::for_path(current_folder);
            file_chooser.set_current_folder(Some(&file))?;
        }

        Ok(file_chooser)
    }

    pub fn create_open_request(
        &self,
        open_params: &OpenFileDialogParams,
        parent: &gtk4::Window,
    ) -> anyhow::Result<gtk4::FileChooserNative> {
        let file_chooser = self.create_file_chooser(open_params.get_action(), parent)?;
        open_params.apply(&file_chooser);
        Ok(file_chooser)
    }

    pub fn create_save_request(
        &self,
        save_params: &SaveFileDialogParams,
        parent: &gtk4::Window,
    ) -> anyhow::Result<gtk4::FileChooserNative> {
        let file_chooser = self.create_file_chooser(gtk4::FileChooserAction::Save, parent)?;
        save_params.apply(&file_chooser)?;
        Ok(file_chooser)
    }
}

fn convert_file_chooser_response(file_chooser: &gtk4::FileChooserNative, response: gtk4::ResponseType) -> anyhow::Result<Option<String>> {
    if response == gtk4::ResponseType::Accept {
        let mut acc = String::new();

        let files = file_chooser.files();
        let n_files = files.n_items();
        for i in 0..n_files {
            let e = files.item(i).with_context(|| format!("List element {i} missing"))?;
            let file: gio::File = e.downcast().map_err(|e| anyhow!("Cannot downcast to gio::File: {e:?}"))?;
            let path = file.path().with_context(|| format!("Missing path for file {file:?}"))?;
            #[allow(clippy::unnecessary_debug_formatting)]
            acc.push_str(path.to_str().with_context(|| format!("Cannot convert to str: {path:?}"))?);
            acc.push_str("\r\n");
        }
        Ok(Some(acc))
    } else {
        Ok(None)
    }
}

pub fn show_file_dialog_impl(file_chooser: &gtk4::FileChooserNative, callback: impl Fn(anyhow::Result<Option<String>>) + 'static) {
    file_chooser.add_weak_ref_notify_local(|| {
        debug!("FileChooserNative destroyed");
    });
    // With `GDK_DEBUG=no-portals`, FileChooserNative object is destroyed at the end of this function, so this is a workaround.
    let clone = file_chooser.clone();
    file_chooser.run_async(move |file_chooser, response_type| {
        debug!("FileChooserNative callback result: {response_type}");
        let result = convert_file_chooser_response(file_chooser, response_type);
        callback(result);
        drop(clone);
    });
}
