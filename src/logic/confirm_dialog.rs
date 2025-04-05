use crate::slint_generatedAppWindow::{AppWindow, Logic, Util};
use slint::ComponentHandle;

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    ui.global::<Util>()
        .on_handle_confirm_dialog(move |handle_type, user_data| {
            let ui = ui_handle.unwrap();

            #[allow(clippy::single_match)]
            match handle_type.as_str() {
                "remove-all-cache" => {
                    ui.global::<Logic>().invoke_remove_all_cache();
                }
                "close-window" => {
                    ui.global::<Util>().invoke_close_window();
                }
                "prompt-delete" => {
                    ui.global::<Logic>().invoke_prompt_delete(user_data);
                }
                "chat-histories-remove-selected" => {
                    ui.global::<Logic>().invoke_chat_histories_remove_selected();
                }
                "retry-question" => {
                    ui.global::<Logic>()
                        .invoke_retry_question(user_data.parse::<i32>().unwrap());
                }
                "edit-question" => {
                    ui.global::<Logic>()
                        .invoke_toggle_edit_question(user_data.parse::<i32>().unwrap());
                }
                "remove-question" => {
                    ui.global::<Logic>()
                        .invoke_remove_question(user_data.parse::<i32>().unwrap());
                }
                "copy-question" => {
                    ui.global::<Logic>().invoke_copy_to_clipboard(user_data);
                }
                _ => (),
            }
        });
}
