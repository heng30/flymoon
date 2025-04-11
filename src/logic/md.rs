// use super::chat_session;
use crate::config::cache_dir;
use crate::slint_generatedAppWindow::{
    AppWindow, ChatEntry as UIChatEntry, Logic, MdElement as UIMdElement, Store,
};
use crate::store_current_chat_session_histories;
use cutil::{crypto, http};
use slint::{ComponentHandle, Image, Model, SharedString, VecModel, Weak};

#[macro_export]
macro_rules! store_current_chat_session_histories_md_elems {
    ($entry:expr) => {
        $entry
            .md_elems
            .as_any()
            .downcast_ref::<VecModel<UIMdElement>>()
            .expect("We know we set a VecModel earlier")
    };
}

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_download_image(move |histories_entry_index, index, url| {
            let index = index as usize;
            let histories_entry_index = histories_entry_index as usize;

            let ui = ui_handle.clone();
            tokio::spawn(async move {
                let file_path = cache_dir().join(&format!("{}.png", crypto::hash(&url)));

                if let Ok(true) = std::fs::exists(&file_path) {
                    // FIXME: If I set the image outside the `tokio::spawn`, would panic
                    async_load_image(ui, histories_entry_index, index, url, file_path);
                } else {
                    if let Ok(data) = http::get_bytes(&url, None).await {
                        _ = std::fs::write(&file_path, data);
                        async_load_image(ui, histories_entry_index, index, url, file_path);
                    }
                }
            });
        });

    // let ui_handle = ui.as_weak();
}

fn get_md_entry(ui: &AppWindow, histories_entry_index: usize, index: usize) -> Option<UIMdElement> {
    if let Some(entry) = store_current_chat_session_histories!(ui).row_data(histories_entry_index) {
        if let Some(item) = store_current_chat_session_histories_md_elems!(entry).row_data(index) {
            return Some(item);
        }
    }

    None
}

fn set_md_entry(ui: &AppWindow, histories_entry_index: usize, index: usize, md_entry: UIMdElement) {
    if let Some(entry) = store_current_chat_session_histories!(ui).row_data(histories_entry_index) {
        if store_current_chat_session_histories_md_elems!(entry).row_count() > index {
            store_current_chat_session_histories_md_elems!(entry).set_row_data(index, md_entry);
        }
    }
}

fn async_load_image(
    ui: Weak<AppWindow>,
    histories_entry_index: usize,
    index: usize,
    url: SharedString,
    file_path: std::path::PathBuf,
) {
    _ = slint::invoke_from_event_loop(move || {
        let ui = ui.unwrap();

        if let Some(mut entry) = get_md_entry(&ui, histories_entry_index, index) {
            if entry.img.url != url {
                return;
            }
            if let Ok(img) = Image::load_from_path(&file_path) {
                entry.img.img = img;
                entry.img.is_loaded = true;
                set_md_entry(&ui, histories_entry_index, index, entry);
            } else {
                _ = std::fs::remove_file(&file_path);
            }
        }
    });
}
