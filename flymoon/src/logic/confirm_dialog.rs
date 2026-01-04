use crate::{global_logic, global_util};
use crate::slint_generatedAppWindow::AppWindow;
use slint::ComponentHandle;

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    global_util!(ui)
        .on_handle_confirm_dialog(move |handle_type, _user_data| {
            let ui = ui_handle.unwrap();

            #[allow(clippy::single_match)]
            match handle_type.as_str() {
                "remove-all-cache" => {
                    global_logic!(ui).invoke_remove_all_cache();
                }
                "close-window" => {
                    global_util!(ui).invoke_close_window();
                }
                "chat-histories-remove-selected" => {
                    global_logic!(ui).invoke_chat_histories_remove_selected();
                }
                _ => (),
            }
        });
}
