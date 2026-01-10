use crate::global_logic;
use crate::slint_generatedAppWindow::{AppWindow, PopupActionSetting};
use slint::ComponentHandle;

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    ui.global::<PopupActionSetting>()
        .on_action(move |action, user_data| {
            let ui = ui_handle.unwrap();

            #[allow(clippy::single_match)]
            match action.as_str() {
                "remove-all-cache" => {
                    global_logic!(ui).invoke_remove_all_cache();
                }
                "retry-question" => {
                    global_logic!(ui).invoke_retry_question(
                        user_data.parse::<i32>().unwrap(),
                        Default::default(),
                    );
                }
                "edit-question" => {
                    global_logic!(ui)
                        .invoke_toggle_edit_question(user_data.parse::<i32>().unwrap());
                }
                "remove-question" => {
                    global_logic!(ui)
                        .invoke_remove_question(user_data.parse::<i32>().unwrap());
                }
                "copy-question" => {
                    global_logic!(ui).invoke_copy_to_clipboard(user_data);
                }
                _ => (),
            }
        });
}
