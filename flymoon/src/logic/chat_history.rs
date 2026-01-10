use super::chat_session;
use crate::slint_generatedAppWindow::{AppWindow, ChatHistory as UIChatHistory};
use crate::{global_logic, logic_cb};
use slint::{ComponentHandle, Model, VecModel};

#[macro_export]
macro_rules! store_chat_history_entries {
    ($ui:expr) => {
        $crate::global_store!($ui)
            .get_chat_histories()
            .as_any()
            .downcast_ref::<VecModel<UIChatHistory>>()
            .expect("We know we set a VecModel earlier")
    };
}

#[macro_export]
macro_rules! store_chat_history_entries_cache {
    ($ui:expr) => {
        $crate::global_store!($ui)
            .get_chat_histories_cache()
            .as_any()
            .downcast_ref::<VecModel<UIChatHistory>>()
            .expect("We know we set a VecModel earlier")
    };
}

fn chat_history_init(ui: &AppWindow) {
    store_chat_history_entries!(ui).set_vec(vec![]);
    store_chat_history_entries_cache!(ui).set_vec(vec![]);
}

pub fn init(ui: &AppWindow) {
    chat_history_init(ui);

    logic_cb!(chat_histories_init, ui);
    logic_cb!(chat_history_load, ui, uuid);
    logic_cb!(chat_histories_select_all, ui);
    logic_cb!(chat_histories_cancel_select_all, ui);
    logic_cb!(chat_history_toggle_select, ui, index);
    logic_cb!(chat_histories_remove_selected, ui);
    logic_cb!(chat_histories_update_list, ui, text);
}

fn chat_histories_init(ui: &AppWindow) {
    let ui = ui.as_weak();

    tokio::spawn(async move {
        let entries = chat_session::get_all_db_entries().await;

        let entries = entries
            .into_iter()
            .map(|entry| entry.into())
            .rev()
            .collect::<Vec<_>>();

        _ = ui.upgrade_in_event_loop(move |ui| {
            store_chat_history_entries!(ui).set_vec(entries.clone());
            store_chat_history_entries_cache!(ui).set_vec(entries);
        });
    });
}

fn chat_history_load(ui: &AppWindow, uuid: slint::SharedString) {
    global_logic!(ui).invoke_load_chat_session(uuid);
}

fn chat_histories_select_all(ui: &AppWindow) {
    for (index, mut entry) in store_chat_history_entries!(ui).iter().enumerate() {
        entry.checked = true;
        store_chat_history_entries!(ui).set_row_data(index, entry);
    }
}

fn chat_histories_cancel_select_all(ui: &AppWindow) {
    for (index, mut entry) in store_chat_history_entries!(ui).iter().enumerate() {
        entry.checked = false;
        store_chat_history_entries!(ui).set_row_data(index, entry);
    }
}

fn chat_history_toggle_select(ui: &AppWindow, index: i32) {
    let index = index as usize;

    let mut entry = store_chat_history_entries!(ui).row_data(index).unwrap();
    entry.checked = !entry.checked;
    store_chat_history_entries!(ui).set_row_data(index, entry);
}

fn chat_histories_remove_selected(ui: &AppWindow) {
    let mut remove_indexs = vec![];
    let mut remove_uuids = vec![];

    for (index, entry) in store_chat_history_entries!(ui).iter().enumerate() {
        if entry.checked {
            remove_indexs.push(index);
            remove_uuids.push(entry.uuid.clone());
            chat_session::delete_db_entry(ui, entry.uuid);
        }
    }

    for index in remove_indexs.into_iter().rev() {
        store_chat_history_entries!(ui).remove(index);
    }

    for uuid in remove_uuids.into_iter().rev() {
        for (index, entry) in store_chat_history_entries_cache!(ui).iter().enumerate() {
            if entry.uuid == uuid {
                store_chat_history_entries_cache!(ui).remove(index);
                break;
            }
        }
    }
}

fn chat_histories_update_list(ui: &AppWindow, text: slint::SharedString) {
    if text.is_empty() {
        let entries = store_chat_history_entries_cache!(ui)
            .iter()
            .collect::<Vec<_>>();

        store_chat_history_entries!(ui).set_vec(entries);
        return;
    }

    let entries = store_chat_history_entries_cache!(ui)
        .iter()
        .filter(|entry| {
            entry
                .summary
                .to_lowercase()
                .contains(text.to_lowercase().as_str())
        })
        .collect::<Vec<_>>();

    store_chat_history_entries!(ui).set_vec(entries);
}
