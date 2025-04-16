#![windows_subsystem = "windows"]

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[tokio::main]
async fn main() {
    extern crate flymoon;
    flymoon::desktop_main().await;
}
