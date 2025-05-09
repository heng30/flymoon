use super::tr::tr;
use crate::{
    slint_generatedAppWindow::{AppWindow, Logic},
    toast_success, toast_warn,
};
use anyhow::{Result, bail};
use slint::ComponentHandle;

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
fn copy_to_clipboard(msg: &str) -> Result<()> {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "linux")] {
            if super::util::is_wayland() {
                if let Ok(_) = copy_to_wayland_clipboard(msg) {
                    return Ok(());
                }
            }
        }
    }

    use clipboard::{ClipboardContext, ClipboardProvider};
    let ctx: Result<ClipboardContext, _> = ClipboardProvider::new();

    match ctx {
        Ok(mut ctx) => match ctx.set_contents(msg.to_string()) {
            Err(e) => bail!("{e:?}"),
            _ => Ok(()),
        },
        Err(e) => bail!("{e:?}"),
    }
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
fn paste_from_clipboard() -> Result<String> {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "linux")] {
            if super::util::is_wayland() {
                if let Ok(text) = paste_from_wayland_clipboard() {
                    return Ok(text);
                }
            }
        }
    }

    use clipboard::{ClipboardContext, ClipboardProvider};
    let ctx: Result<ClipboardContext, _> = ClipboardProvider::new();

    match ctx {
        Ok(mut ctx) => match ctx.get_contents() {
            Err(e) => bail!("{e:?}"),
            Ok(msg) => Ok(msg),
        },
        Err(e) => bail!("{e:?}"),
    }
}

#[cfg(target_os = "android")]
fn copy_to_clipboard(msg: &str) -> Result<()> {
    match terminal_clipboard::set_string(msg) {
        Err(e) => bail!("{e:?}"),
        _ => Ok(()),
    }
}

#[cfg(target_os = "android")]
fn paste_from_clipboard() -> Result<String> {
    match terminal_clipboard::get_string() {
        Err(e) => bail!("{e:?}"),
        Ok(msg) => Ok(msg),
    }
}

#[cfg(target_os = "linux")]
fn copy_to_wayland_clipboard(text: &str) -> Result<()> {
    duct::cmd!("wl-copy", text).run()?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn paste_from_wayland_clipboard() -> Result<String> {
    Ok(duct::cmd!("wl-paste").read()?)
}

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_copy_to_clipboard(move |msg| {
        let ui = ui_handle.unwrap();
        match copy_to_clipboard(&msg) {
            Err(e) => toast_warn!(
                ui,
                format!("{}. {}: {e:?}", tr("Copy failed"), tr("Reason"))
            ),
            _ => toast_success!(ui, tr("Copy success")),
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_paste_from_clipboard(move || {
        let ui = ui_handle.unwrap();
        match paste_from_clipboard() {
            Err(e) => {
                toast_warn!(
                    ui,
                    format!("{}. {}: {e:?}", tr("Paste failed"), tr("Reason"))
                );
                slint::SharedString::default()
            }
            Ok(msg) => msg.into(),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_clipboard() -> Result<()> {
        let msg = "hello world";
        copy_to_clipboard(msg)?;
        let res = paste_from_clipboard()?;

        assert_eq!(msg, res);
        Ok(())
    }
}
