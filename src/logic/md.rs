use super::chat_session;
use crate::slint_generatedAppWindow::{AppWindow, ChatHistory as UIChatHistory, Logic, Store};
use cutil::http;
use image::Rgb;
use once_cell::sync::Lazy;
use slint::{ComponentHandle, Image, Model, Rgb8Pixel, SharedPixelBuffer, VecModel};
use std::{collections::HashMap, sync::Mutex};

static IMAGE_CACHE: Lazy<Mutex<HashMap<String, ImageCache>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

struct ImageCache {
    try_times: u8,
    is_loading: bool,
    data: Vec<u8>,
}


pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_is_image_downloaded(move |url| {
        let ui = ui_handle.unwrap();

        false
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_download_image(move |url| {
        let ui = ui_handle.unwrap();
        // match QrCode::new(text) {
        //     Ok(code) => {
        //         let qrc = code.render::<Rgb<u8>>().build();

        //         let buffer = SharedPixelBuffer::<Rgb8Pixel>::clone_from_slice(
        //             qrc.as_raw(),
        //             qrc.width(),
        //             qrc.height(),
        //         );
        //         Image::from_rgb8(buffer)
        //     }
        //     _ => ui.global::<Icons>().get_no_data(),
        // }
        Default::default()
    });


}
