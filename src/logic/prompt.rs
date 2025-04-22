use super::{toast, tr::tr};
use crate::db;
use crate::db::def::{PROMPT_TABLE, PromptEntry};
use crate::slint_generatedAppWindow::{AppWindow, Logic, PromptEntry as UIPromptEntry, Store};
use slint::{ComponentHandle, Model, SharedString, VecModel, Weak};
use uuid::Uuid;

#[macro_export]
macro_rules! store_prompt_entries {
    ($ui:expr) => {
        $ui.global::<Store>()
            .get_prompt_entries()
            .as_any()
            .downcast_ref::<VecModel<UIPromptEntry>>()
            .expect("We know we set a VecModel earlier")
    };
}

#[macro_export]
macro_rules! store_input_prompt_list_entries {
    ($ui:expr) => {
        $ui.global::<Store>()
            .get_input_prompt_list_entries()
            .as_any()
            .downcast_ref::<VecModel<UIPromptEntry>>()
            .expect("We know we set a VecModel earlier")
    };
}

async fn get_from_db() -> Vec<UIPromptEntry> {
    let entries = match db::entry::select_all(PROMPT_TABLE).await {
        Ok(items) => items
            .into_iter()
            .filter_map(|item| serde_json::from_str::<PromptEntry>(&item.data).ok())
            .map(|item| item.into())
            .collect(),
        Err(e) => {
            log::warn!("{:?}", e);
            vec![]
        }
    };

    entries
}

fn prompt_init(ui: Weak<AppWindow>) {
    tokio::spawn(async move {
        let entries = get_from_db().await;

        let _ = slint::invoke_from_event_loop(move || {
            store_prompt_entries!(ui.unwrap()).set_vec(entries);
        });
    });
}

pub fn init(ui: &AppWindow) {
    prompt_init(ui.as_weak());

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_prompt_update(move |mut entry| {
        let ui = ui_handle.unwrap();

        if entry.uuid.is_empty() {
            entry.uuid = Uuid::new_v4().to_string().into();
            add_entry(&ui, entry);
        } else {
            update_entry(&ui, entry);
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_prompt_delete(move |uuid| {
        let ui = ui_handle.unwrap();

        for (index, entry) in store_prompt_entries!(ui).iter().enumerate() {
            if entry.uuid != uuid {
                continue;
            }

            store_prompt_entries!(ui).remove(index);
            delete_entry(&ui, uuid);
            return;
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_prompt_popup(move |text| {
        let ui = ui_handle.unwrap();

        if text.is_empty() || text.contains(' ') {
            store_input_prompt_list_entries!(ui).set_vec(vec![]);
            return;
        }

        let mut shortcuts = vec![];
        for entry in store_prompt_entries!(ui).iter() {
            let shortcut = format!("/{}", entry.shortcut);

            if shortcut.starts_with(text.as_str()) {
                shortcuts.push(entry);
            }
        }

        store_input_prompt_list_entries!(ui).set_vec(shortcuts);
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_prompt_popup_clear(move |text| {
        let ui = ui_handle.unwrap();

        if text.is_empty() || text.contains(' ') {
            store_input_prompt_list_entries!(ui).set_vec(vec![]);
        }
    });
}

fn add_entry(ui: &AppWindow, entry_ui: UIPromptEntry) {
    let entry_db: PromptEntry = entry_ui.clone().into();
    store_prompt_entries!(ui).push(entry_ui);

    let ui = ui.as_weak();
    tokio::spawn(async move {
        let data = serde_json::to_string(&entry_db).unwrap();
        match db::entry::insert(PROMPT_TABLE, &entry_db.uuid, &data).await {
            Err(e) => toast::async_toast_warn(
                ui,
                format!("{}. {}: {e:?}", tr("Add entry failed"), tr("Reason")),
            ),
            _ => toast::async_toast_success(ui, tr("Add entry successfully")),
        }
    });
}

fn update_entry(ui: &AppWindow, entry_ui: UIPromptEntry) {
    let entry_db: PromptEntry = entry_ui.clone().into();

    for (index, entry) in store_prompt_entries!(ui).iter().enumerate() {
        if entry.uuid != entry_ui.uuid {
            continue;
        }

        store_prompt_entries!(ui).set_row_data(index, entry_ui);
        break;
    }

    let ui = ui.as_weak();
    tokio::spawn(async move {
        let data = serde_json::to_string(&entry_db).unwrap();
        match db::entry::update(PROMPT_TABLE, &entry_db.uuid, &data).await {
            Err(e) => toast::async_toast_warn(
                ui,
                format!("{}. {}: {e:?}", tr("Update entry failed"), tr("Reason")),
            ),
            _ => toast::async_toast_success(ui, tr("Update entry successfully")),
        }
    });
}

fn delete_entry(ui: &AppWindow, uuid: SharedString) {
    let ui = ui.as_weak();
    tokio::spawn(async move {
        match db::entry::delete(PROMPT_TABLE, uuid.as_str()).await {
            Err(e) => toast::async_toast_warn(
                ui,
                format!("{}. {}: {e:?}", tr("Remove entry failed"), tr("Reason")),
            ),
            _ => toast::async_toast_success(ui, tr("Remove entry successfully")),
        }
    });
}
