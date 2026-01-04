use crate::slint_generatedAppWindow::AppWindow;

mod about;
mod clipboard;
mod util;
mod setting;

mod confirm_dialog;
mod popup_action;
mod toast;

#[allow(unused)]
mod tr;

mod chat_history;
mod chat_session;
mod md;

pub fn init(ui: &AppWindow) {
    util::init(ui);
    clipboard::init(ui);
    about::init(ui);
    setting::init(ui);

    toast::init(ui);
    confirm_dialog::init(ui);
    popup_action::init(ui);

    chat_history::init(ui);
    chat_session::init(ui);
    md::init(ui);
}
