use crate::slint_generatedAppWindow::AppWindow;

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "android"
))]
mod about;

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "android"
))]
mod clipboard;

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "android"
))]
mod util;

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "android"
))]
mod setting;

mod confirm_dialog;
mod popup_action;
mod toast;

#[allow(unused)]
mod tr;

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
mod prompt;

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
mod chat_history;

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
mod chat_session;

pub fn init(ui: &AppWindow) {
    #[cfg(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "macos",
        target_os = "android",
    ))]
    {
        util::init(ui);
        clipboard::init(ui);
        about::init(ui);
        setting::init(ui);
    }

    toast::init(ui);
    confirm_dialog::init(ui);
    popup_action::init(ui);

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
        prompt::init(ui);
        chat_history::init(ui);
        chat_session::init(ui);
    }
}
