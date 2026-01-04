slint::include_modules!();

#[macro_use]
extern crate derivative;

mod config;
mod version;

#[cfg(feature = "database")]
mod db;

mod logic;

pub fn init_logger() {
    use std::io::Write;

    env_logger::builder()
        .format(|buf, record| {
            let style = buf.default_level_style(record.level());
            let ts = cutil::time::local_now("%H:%M:%S");

            writeln!(
                buf,
                "[{} {style}{}{style:#} {}::{} {}] {}",
                ts,
                record.level(),
                record
                    .module_path()
                    .unwrap_or("None")
                    .split("::")
                    .next()
                    .unwrap_or("None"),
                record
                    .file()
                    .unwrap_or("None")
                    .split('/')
                    .next_back()
                    .unwrap_or("None"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();
}

async fn ui_before() {
    init_logger();
    config::init();

    #[cfg(feature = "database")]
    db::init(config::all().db_path.to_str().expect("invalid db path")).await;
}

fn ui_after(ui: &AppWindow) {
    logic::init(ui);
}

pub async fn desktop_main() {
    log::debug!("start...");

    ui_before().await;
    let ui = AppWindow::new().unwrap();
    ui_after(&ui);

    global_util!(ui).invoke_set_window_center();

    ui.run().unwrap();

    log::debug!("exit...");
}
