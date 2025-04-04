// use super::{toast, tr::tr};
// use crate::db;
// use crate::db::def::ChatSession;
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

// impl From<ChatSession> for UIChatHistoryEntry {
//     fn from(entry: ChatHistoryEntry) -> Self {
//         UIChatHistoryEntry {
//             uuid: entry.uuid.into(),
//             time: entry.time.into(),
//             summary: entry.history.into(),
//         }
//     }
// }
//

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_chat_history_load(move |uuid| {
        let ui = ui_handle.unwrap();

        todo!();
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

            todo!();
            // for (index, entry) in store_chat_history_entries!(ui).iter().enumerate() {
            //     entry.checked = false;
            //     store_chat_history_entries!(ui).set_row_data(index, entry);
            // }
        });
}
