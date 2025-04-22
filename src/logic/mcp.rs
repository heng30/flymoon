use super::{toast, tr::tr};
use crate::slint_generatedAppWindow::{
    AppWindow, Logic, MCPEntry as UIMCPEntry, MCPServerStatus as UIMCPServerStatus,
    PromptEntry as UIPromptEntry, Store,
};
use crate::{
    db::{
        self,
        def::{MCP_TABLE, MCPEntry},
    },
    store_input_prompt_list_entries, toast_success, toast_warn,
};
use anyhow::Result;
use slint::{ComponentHandle, Model, SharedString, VecModel, Weak};
use uuid::Uuid;

#[macro_export]
macro_rules! store_mcp_entries {
    ($ui:expr) => {
        $ui.global::<Store>()
            .get_mcp_entries()
            .as_any()
            .downcast_ref::<VecModel<UIMCPEntry>>()
            .expect("We know we set a VecModel earlier")
    };
}

async fn get_from_db() -> Vec<UIMCPEntry> {
    let entries = match db::entry::select_all(MCP_TABLE).await {
        Ok(items) => items
            .into_iter()
            .filter_map(|item| serde_json::from_str::<MCPEntry>(&item.data).ok())
            .map(|item| item.into())
            .collect(),
        Err(e) => {
            log::warn!("{:?}", e);
            vec![]
        }
    };

    entries
}

fn mcp_init(ui: Weak<AppWindow>) {
    tokio::spawn(async move {
        let entries = get_from_db().await;

        let _ = slint::invoke_from_event_loop(move || {
            store_mcp_entries!(ui.unwrap()).set_vec(entries);
        });
    });
}

pub fn init(ui: &AppWindow) {
    mcp_init(ui.as_weak());

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_mcp_update(move |mut entry| {
        let ui = ui_handle.unwrap();

        if entry.uuid.is_empty() {
            entry.uuid = Uuid::new_v4().to_string().into();
            add_entry(&ui, entry);
        } else {
            update_entry(&ui, entry);
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_mcp_delete(move |uuid| {
        let ui = ui_handle.unwrap();

        for (index, entry) in store_mcp_entries!(ui).iter().enumerate() {
            if entry.uuid != uuid {
                continue;
            }

            store_mcp_entries!(ui).remove(index);
            delete_entry(&ui, uuid);
            return;
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_mcp_popup(move |text| {
        let ui = ui_handle.unwrap();

        if text.is_empty() || text.contains(' ') {
            store_input_prompt_list_entries!(ui).set_vec(vec![]);
            return;
        }

        let mut shortcuts = vec![];
        for entry in store_mcp_entries!(ui).iter() {
            let shortcut = format!("@{}", entry.shortcut);

            if shortcut.starts_with(text.as_str()) {
                shortcuts.push(entry.into());
            }
        }

        store_input_prompt_list_entries!(ui).set_vec(shortcuts);
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_start_mcp_server(move |index| {
        start_mcp_client(ui_handle.clone(), index as usize);
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_stop_mcp_server(move |index| {
        stop_mcp_client(ui_handle.clone(), index as usize);
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_restart_mcp_server(move |index| {
        restart_mcp_client(ui_handle.clone(), index as usize);
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_update_all_mcp_server_status(move || {
            update_all_mcp_server_status(ui_handle.clone());
        });
}

fn add_entry(ui: &AppWindow, entry_ui: UIMCPEntry) {
    let entry_db: MCPEntry = entry_ui.clone().into();
    store_mcp_entries!(ui).push(entry_ui);

    let ui = ui.as_weak();
    tokio::spawn(async move {
        let data = serde_json::to_string(&entry_db).unwrap();
        match db::entry::insert(MCP_TABLE, &entry_db.uuid, &data).await {
            Err(e) => toast::async_toast_warn(
                ui,
                format!("{}. {}: {e:?}", tr("Add entry failed"), tr("Reason")),
            ),
            _ => toast::async_toast_success(ui, tr("Add entry successfully")),
        }
    });
}

fn update_entry(ui: &AppWindow, entry_ui: UIMCPEntry) {
    let entry_db: MCPEntry = entry_ui.clone().into();

    for (index, entry) in store_mcp_entries!(ui).iter().enumerate() {
        if entry.uuid != entry_ui.uuid {
            continue;
        }

        store_mcp_entries!(ui).set_row_data(index, entry_ui);
        break;
    }

    let ui = ui.as_weak();
    tokio::spawn(async move {
        let data = serde_json::to_string(&entry_db).unwrap();
        match db::entry::update(MCP_TABLE, &entry_db.uuid, &data).await {
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
        match db::entry::delete(MCP_TABLE, uuid.as_str()).await {
            Err(e) => toast::async_toast_warn(
                ui,
                format!("{}. {}: {e:?}", tr("Remove entry failed"), tr("Reason")),
            ),
            _ => toast::async_toast_success(ui, tr("Remove entry successfully")),
        }
    });
}

fn start_mcp_client(ui: Weak<AppWindow>, index: usize) {
    let entry = store_mcp_entries!(ui.clone().unwrap())
        .row_data(index)
        .unwrap();

    tokio::spawn(async move {
        match mcp::create_mcp_client(&entry.config).await {
            Err(e) => {
                _ = slint::invoke_from_event_loop(move || {
                    let ui = ui.unwrap();

                    if let Some(mut entry) = store_mcp_entries!(ui).row_data(index) {
                        entry.status = UIMCPServerStatus::Running;
                        store_mcp_entries!(ui).set_row_data(index, entry);
                    }

                    toast_warn!(
                        ui,
                        format!("{}. {}: {e:?}", tr("Start server failed"), tr("Reason"))
                    );
                });
            }
            _ => {
                _ = slint::invoke_from_event_loop(move || {
                    let ui = ui.unwrap();

                    if let Some(mut entry) = store_mcp_entries!(ui).row_data(index) {
                        entry.status = UIMCPServerStatus::Running;
                        store_mcp_entries!(ui).set_row_data(index, entry);
                    }

                    toast_success!(ui, tr("Start server successfully"));
                });
            }
        }
    });
}

fn stop_mcp_client(ui: Weak<AppWindow>, index: usize) {
    let entry = store_mcp_entries!(ui.clone().unwrap())
        .row_data(index)
        .unwrap();

    tokio::spawn(async move {
        match stop_mcp_client_inner(&entry.config).await {
            Err(e) => {
                _ = slint::invoke_from_event_loop(move || {
                    let ui = ui.unwrap();

                    if let Some(mut entry) = store_mcp_entries!(ui).row_data(index) {
                        entry.status = UIMCPServerStatus::Failed;
                        store_mcp_entries!(ui).set_row_data(index, entry);
                    }

                    toast_warn!(
                        ui,
                        format!("{}. {}: {e:?}", tr("Stop server failed"), tr("Reason"))
                    );
                });
            }
            _ => {
                _ = slint::invoke_from_event_loop(move || {
                    let ui = ui.unwrap();

                    if let Some(mut entry) = store_mcp_entries!(ui).row_data(index) {
                        entry.status = UIMCPServerStatus::None;
                        store_mcp_entries!(ui).set_row_data(index, entry);
                    }

                    toast_success!(ui, tr("Stop server successfully"));
                });
            }
        }
    });
}

fn restart_mcp_client(ui: Weak<AppWindow>, index: usize) {
    let entry = store_mcp_entries!(ui.clone().unwrap())
        .row_data(index)
        .unwrap();

    tokio::spawn(async move {
        match restart_mcp_client_inner(&entry.config).await {
            Err(e) => {
                _ = slint::invoke_from_event_loop(move || {
                    let ui = ui.unwrap();

                    if let Some(mut entry) = store_mcp_entries!(ui).row_data(index) {
                        entry.status = UIMCPServerStatus::Running;
                        store_mcp_entries!(ui).set_row_data(index, entry);
                    }

                    toast_warn!(
                        ui,
                        format!("{}. {}: {e:?}", tr("Restart server failed"), tr("Reason"))
                    );
                });
            }
            _ => {
                _ = slint::invoke_from_event_loop(move || {
                    let ui = ui.unwrap();

                    if let Some(mut entry) = store_mcp_entries!(ui).row_data(index) {
                        entry.status = UIMCPServerStatus::Running;
                        store_mcp_entries!(ui).set_row_data(index, entry);
                    }

                    toast_success!(ui, tr("Restart server successfully"));
                });
            }
        }
    });
}


fn update_all_mcp_server_status(ui: Weak<AppWindow) {
    todo!();
}

async fn stop_mcp_client_inner(config: &str) -> Result<()> {
    let server_name = mcp::mcp_server_name_from_config(config)?;
    mcp::cancel_mcp_client(&server_name).await?;
    Ok(())
}

async fn restart_mcp_client_inner(config: &str) -> Result<()> {
    let server_name = mcp::mcp_server_name_from_config(config)?;
    mcp::cancel_mcp_client(&server_name).await?;
    mcp::create_mcp_client(config).await?;
    Ok(())
}
