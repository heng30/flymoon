use crate::slint_generatedAppWindow::{
    AppWindow, ChatEntry as UIChatEntry, Logic, MdElement as UIMdElement,
    MdElementType as UIMdElementType, MdHeading as UIMdHeading, MdImage as UIMdImage,
    MdListItem as UIMdListItem, MdMath as UIMdMath, MdTable as UIMdTable, MdUrl as UIMdUrl, Store,
};
use crate::{config::cache_dir, store_current_chat_session_histories};
use cutil::{crypto, http};
use dummy_markdown::{self, MdElement, MdElementType, MdHeading, MdListItem, MdTable, MdUrl};
use once_cell::sync::Lazy;
use slint::{ComponentHandle, Image, Model, ModelRc, SharedString, VecModel, Weak};
use std::{collections::HashMap, sync::Mutex};

struct DownloadImageCache {
    try_times: u32,
    is_loading: bool,
}

static DOWNLOAD_IMAGE_CACHE: Lazy<Mutex<HashMap<String, DownloadImageCache>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[macro_export]
macro_rules! store_current_chat_session_histories_md_elems {
    ($entry:expr) => {
        $entry
            .md_elems
            .as_any()
            .downcast_ref::<VecModel<UIMdElement>>()
            .expect("We know we set a VecModel earlier")
    };
}

#[macro_export]
macro_rules! store_current_chat_session_histories_link_urls {
    ($entry:expr) => {
        $entry
            .link_urls
            .as_any()
            .downcast_ref::<VecModel<UIMdUrl>>()
            .expect("We know we set a VecModel earlier")
    };
}

impl From<MdElementType> for UIMdElementType {
    fn from(ty: MdElementType) -> Self {
        match ty {
            MdElementType::Text => UIMdElementType::Text,
            MdElementType::Math => UIMdElementType::Math,
            MdElementType::ImageUrl => UIMdElementType::Image,
            MdElementType::ListItem => UIMdElementType::ListItem,
            MdElementType::Heading => UIMdElementType::Heading,
            MdElementType::CodeBlock => UIMdElementType::CodeBlock,
            MdElementType::Table => UIMdElementType::Table,
            _ => unreachable!(),
        }
    }
}

impl From<MdUrl> for UIMdUrl {
    fn from(entry: MdUrl) -> Self {
        UIMdUrl {
            text: entry.text.into(),
            url: entry.url.into(),
        }
    }
}

impl From<MdHeading> for UIMdHeading {
    fn from(entry: MdHeading) -> Self {
        UIMdHeading {
            level: entry.level,
            text: entry.text.into(),
        }
    }
}

impl From<MdListItem> for UIMdListItem {
    fn from(entry: MdListItem) -> Self {
        UIMdListItem {
            level: entry.level,
            text: entry.text.into(),
        }
    }
}

impl From<String> for UIMdMath {
    fn from(formula: String) -> Self {
        UIMdMath {
            formula: formula.into(),
            ..Default::default()
        }
    }
}

impl From<String> for UIMdImage {
    fn from(url: String) -> Self {
        UIMdImage {
            url: url.into(),
            ..Default::default()
        }
    }
}

impl From<MdTable> for UIMdTable {
    fn from(table: MdTable) -> Self {
        UIMdTable {
            head: ModelRc::new(
                table
                    .head
                    .into_iter()
                    .map(Into::into)
                    .collect::<VecModel<SharedString>>(),
            ),

            rows: ModelRc::new(
                table
                    .rows
                    .into_iter()
                    .map(|row| {
                        ModelRc::new(
                            row.into_iter()
                                .map(Into::into)
                                .collect::<VecModel<SharedString>>(),
                        )
                    })
                    .collect::<VecModel<_>>(),
            ),
        }
    }
}

impl From<MdElement> for UIMdElement {
    fn from(entry: MdElement) -> Self {
        UIMdElement {
            ty: entry.ty.into(),
            text: entry.text.into(),
            math: entry.math.into(),
            code_block: entry.code_block.trim().to_string().into(),
            list_item: entry.list_item.into(),
            img: entry.image_url.into(),
            heading: entry.heading.into(),
            table: entry.table.into(),
        }
    }
}

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_download_image(move |histories_entry_index, index, url| {
            let index = index as usize;
            let histories_entry_index = histories_entry_index as usize;

            let ui = ui_handle.clone();
            tokio::spawn(async move {
                let file_path = cache_dir().join(&format!("{}.png", crypto::hash(&url)));

                if let Ok(true) = std::fs::exists(&file_path) {
                    // FIXME: If I set the image outside the `tokio::spawn`, would panic
                    async_load_image(ui, histories_entry_index, index, url, file_path);
                } else {
                    {
                        let mut cache = DOWNLOAD_IMAGE_CACHE.lock().unwrap();
                        if let Some(entry) = cache.get_mut(url.as_str()) {
                            if entry.is_loading || entry.try_times >= 3 {
                                return;
                            }

                            entry.try_times += 1;
                            entry.is_loading = true;
                        }
                    }

                    if let Ok(data) = http::get_bytes(&url, None).await {
                        _ = std::fs::write(&file_path, data);
                        async_load_image(ui, histories_entry_index, index, url.clone(), file_path);
                    }

                    {
                        let mut cache = DOWNLOAD_IMAGE_CACHE.lock().unwrap();
                        if let Some(entry) = cache.get_mut(url.as_str()) {
                            entry.is_loading = false;
                        }
                    }
                }
            });
        });

    // TODO
    ui.global::<Logic>()
        .on_render_formula_svg(move |histories_entry_index, index, formula| {});
}

pub fn need_parse_stream_bot_text(ui: &AppWindow) -> bool {
    let rows = store_current_chat_session_histories!(ui).row_count();
    if rows == 0 {
        return false;
    }

    let last_entry = store_current_chat_session_histories!(ui)
        .row_data(rows - 1)
        .unwrap();

    last_entry.bot.chars().last().map_or(false, |c| c == '\n')
}

pub fn parse_stream_bot_text(ui: &AppWindow) {
    let rows = store_current_chat_session_histories!(ui).row_count();
    if rows == 0 {
        return;
    }
    let last_index = rows - 1;

    let last_entry = store_current_chat_session_histories!(ui)
        .row_data(last_index)
        .unwrap();

    if last_entry.bot.trim().is_empty() {
        return;
    }

    let is_end_with_newline = last_entry.bot.ends_with('\n');
    let bot_text = last_entry.bot.trim();

    let ((md_elems, _link_urls), _unfinished_text) = {
        if is_end_with_newline {
            (dummy_markdown::parser::run(bot_text), "")
        } else {
            if let Some((before, after)) = bot_text.rsplit_once('\n') {
                (dummy_markdown::parser::run(before), after)
            } else {
                (dummy_markdown::parser::run(bot_text), "")
            }
        }
    };

    // update Markdown elements
    let rows = store_current_chat_session_histories_md_elems!(last_entry).row_count();

    if rows == 0 || rows > md_elems.len() {
        let elems = md_elems
            .into_iter()
            .map(|item| item.into())
            .collect::<Vec<_>>();
        store_current_chat_session_histories_md_elems!(last_entry).set_vec(elems);
    } else {
        let offset = md_elems.len() - rows;

        store_current_chat_session_histories_md_elems!(last_entry)
            .set_row_data(rows - 1, md_elems[rows - 1].clone().into());

        for i in 0..offset {
            store_current_chat_session_histories_md_elems!(last_entry)
                .push(md_elems[rows + i].clone().into());
        }
    }
}

pub fn parse_last_history_bot_text(ui: &AppWindow) {
    let rows = store_current_chat_session_histories!(ui).row_count();
    if rows == 0 {
        return;
    }
    let last_index = rows - 1;

    let entry = store_current_chat_session_histories!(ui)
        .row_data(last_index)
        .unwrap();

    if entry.bot.trim().is_empty() {
        return;
    }

    let (md_elems, link_urls) = dummy_markdown::parser::run(&entry.bot);

    let elems = md_elems
        .into_iter()
        .map(|item| item.into())
        .collect::<Vec<_>>();
    store_current_chat_session_histories_md_elems!(entry).set_vec(elems);

    let urls = link_urls
        .into_iter()
        .map(|item| item.into())
        .collect::<Vec<_>>();
    store_current_chat_session_histories_link_urls!(entry).set_vec(urls);
}

pub fn parse_histories_bot_text(ui: &AppWindow) {
    for entry in store_current_chat_session_histories!(ui).iter() {
        if entry.bot.trim().is_empty() {
            continue;
        }

        let (md_elems, link_urls) = dummy_markdown::parser::run(&entry.bot);

        let elems = md_elems
            .into_iter()
            .map(|item| item.into())
            .collect::<Vec<_>>();
        store_current_chat_session_histories_md_elems!(entry).set_vec(elems);

        let urls = link_urls
            .into_iter()
            .map(|item| item.into())
            .collect::<Vec<_>>();
        store_current_chat_session_histories_link_urls!(entry).set_vec(urls);
    }
}

fn get_md_entry(ui: &AppWindow, histories_entry_index: usize, index: usize) -> Option<UIMdElement> {
    if let Some(entry) = store_current_chat_session_histories!(ui).row_data(histories_entry_index) {
        if let Some(item) = store_current_chat_session_histories_md_elems!(entry).row_data(index) {
            return Some(item);
        }
    }

    None
}

fn set_md_entry(ui: &AppWindow, histories_entry_index: usize, index: usize, md_entry: UIMdElement) {
    if let Some(entry) = store_current_chat_session_histories!(ui).row_data(histories_entry_index) {
        if store_current_chat_session_histories_md_elems!(entry).row_count() > index {
            store_current_chat_session_histories_md_elems!(entry).set_row_data(index, md_entry);
        }
    }
}

fn async_load_image(
    ui: Weak<AppWindow>,
    histories_entry_index: usize,
    index: usize,
    url: SharedString,
    file_path: std::path::PathBuf,
) {
    _ = slint::invoke_from_event_loop(move || {
        let ui = ui.unwrap();

        if let Some(mut entry) = get_md_entry(&ui, histories_entry_index, index) {
            if entry.img.url != url {
                return;
            }
            if let Ok(img) = Image::load_from_path(&file_path) {
                entry.img.img = img;
                entry.img.is_loaded = true;
                set_md_entry(&ui, histories_entry_index, index, entry);
            } else {
                _ = std::fs::remove_file(&file_path);
            }
        }
    });
}
