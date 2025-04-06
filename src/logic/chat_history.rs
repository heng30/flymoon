use super::chat_session;
use crate::slint_generatedAppWindow::{AppWindow, ChatHistory as UIChatHistory, Logic, Store};
use slint::{ComponentHandle, Model, VecModel};

#[macro_export]
macro_rules! store_chat_history_entries {
    ($ui:expr) => {
        $ui.global::<Store>()
            .get_chat_histories()
            .as_any()
            .downcast_ref::<VecModel<UIChatHistory>>()
            .expect("We know we set a VecModel earlier")
    };
}

#[macro_export]
macro_rules! store_chat_history_entries_cache {
    ($ui:expr) => {
        $ui.global::<Store>()
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

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_chat_histories_init(move || {
        let ui = ui_handle.clone();

        tokio::spawn(async move {
            let entries = chat_session::get_from_db().await;

            let entries = entries
                .into_iter()
                .map(|entry| entry.into())
                .rev()
                .collect::<Vec<_>>();

            _ = slint::invoke_from_event_loop(move || {
                store_chat_history_entries!(ui.unwrap()).set_vec(entries.clone());
                store_chat_history_entries_cache!(ui.unwrap()).set_vec(entries);
            });
        });
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_chat_history_load(move |uuid| {
        let ui = ui_handle.unwrap();
        ui.global::<Logic>().invoke_load_chat_session(uuid);
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_chat_histories_select_all(move || {
        let ui = ui_handle.unwrap();

        for (index, mut entry) in store_chat_history_entries!(ui).iter().enumerate() {
            entry.checked = true;
            store_chat_history_entries!(ui).set_row_data(index, entry);
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_chat_histories_cancel_select_all(move || {
            let ui = ui_handle.unwrap();

            for (index, mut entry) in store_chat_history_entries!(ui).iter().enumerate() {
                entry.checked = false;
                store_chat_history_entries!(ui).set_row_data(index, entry);
            }
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_chat_histories_remove_selected(move || {
            let ui = ui_handle.unwrap();
            let mut remove_indexs = vec![];

            for (index, entry) in store_chat_history_entries!(ui).iter().enumerate() {
                if entry.checked {
                    remove_indexs.push(index);
                    chat_session::delete_db_entry(&ui, entry.uuid);
                }
            }

            for index in remove_indexs.into_iter().rev() {
                store_chat_history_entries!(ui).remove(index);
            }
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_chat_histories_update_list(move |text| {
            let ui = ui_handle.unwrap();

            if text.is_empty() {
                let entries = store_chat_history_entries_cache!(ui)
                    .iter()
                    .collect::<Vec<_>>();

                store_chat_history_entries!(ui).set_vec(entries);
                return;
            }

            let entries = store_chat_history_entries!(ui)
                .iter()
                .filter(|entry| {
                    entry
                        .summary
                        .to_lowercase()
                        .contains(text.to_lowercase().as_str())
                })
                .map(|entry| entry.into())
                .collect::<Vec<_>>();

            store_chat_history_entries!(ui).set_vec(entries);
        });
}
