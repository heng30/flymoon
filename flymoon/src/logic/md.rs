use crate::global_logic;
use crate::slint_generatedAppWindow::{
    AppWindow, ChatEntry as UIChatEntry, MdCodeBlock as UIMdCodeBlock, MdElement as UIMdElement,
    MdElementType as UIMdElementType, MdHeading as UIMdHeading, MdListItem as UIMdListItem,
    MdMath as UIMdMath, MdTable as UIMdTable,
};
use crate::{config::cache_dir, store_current_chat_session_histories};
use cutil::crypto;
use dummy_markdown::{self, MdCodeBlock, MdElement, MdElementType, MdHeading, MdListItem, MdTable};
use once_cell::sync::OnceCell;
use slint::{ComponentHandle, Image, Model, ModelRc, SharedString, VecModel, Weak};

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

impl From<MdCodeBlock> for UIMdCodeBlock {
    fn from(code_block: MdCodeBlock) -> Self {
        UIMdCodeBlock {
            lang: code_block.lang.into(),
            code: code_block.code.trim().into(),
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
            code_block: entry.code_block.into(),
            list_item: entry.list_item.into(),
            heading: entry.heading.into(),
            table: entry.table.into(),
            ..Default::default()
        }
    }
}

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    global_logic!(ui).on_render_formula_svg(
        move |histories_entry_index, index, formula: slint::SharedString| {
            let index = index as usize;
            let histories_entry_index = histories_entry_index as usize;

            let ui = ui_handle.clone();
            tokio::spawn(async move {
                let file_path = cache_dir().join(format!("{}.svg", crypto::hash(&formula)));

                if let Ok(true) = std::fs::exists(&file_path) {
                    async_load_math(ui, histories_entry_index, index, formula.clone(), file_path);
                } else {
                    let formula_str = formula.to_string();
                    match duct::cmd!("latex-image", "-f", &formula_str, "-o", &file_path).read() {
                        Err(e) => {
                            log::warn!("latex-image can't render: `{}`. error: {}", formula_str, e)
                        }
                        Ok(output) => {
                            log::debug!("{output:?}");

                            async_load_math(
                                ui,
                                histories_entry_index,
                                index,
                                formula.clone(),
                                file_path,
                            );
                        }
                    }
                }
            });
        },
    );
}

pub fn parse_stream_bot_text(ui: &AppWindow) {
    let rows = store_current_chat_session_histories!(ui).row_count();
    if rows == 0 {
        return;
    }
    let last_entry = store_current_chat_session_histories!(ui)
        .row_data(rows - 1)
        .unwrap();

    let bot_text = last_entry.bot.trim();
    if bot_text.is_empty() {
        return;
    }

    let (md_elems, _) = dummy_markdown::parser::run(bot_text, can_parse_math());

    // update Markdown elements
    let rows = store_current_chat_session_histories_md_elems!(last_entry).row_count();

    if rows == 0 || rows > md_elems.len() {
        let elems = md_elems
            .into_iter()
            .map(|item| item.into())
            .collect::<Vec<_>>();
        store_current_chat_session_histories_md_elems!(last_entry).set_vec(elems);
    } else {
        let mut insert_row_index = rows;
        let mut offset = md_elems.len() - rows;
        let ui_md_elems = store_current_chat_session_histories_md_elems!(last_entry)
            .iter()
            .collect::<Vec<UIMdElement>>();

        // find the first unmatched element
        let mut diff_row_index = None;
        for (index, elems) in ui_md_elems.iter().enumerate() {
            if <dummy_markdown::MdElementType as Into<UIMdElementType>>::into(md_elems[index].ty)
                != elems.ty
            {
                diff_row_index = Some(index);
                break;
            }
        }

        // remove all element after unmatched element and unmatched element itself
        if let Some(index) = diff_row_index {
            let remove_counts = rows - index;

            for _ in 0..remove_counts {
                store_current_chat_session_histories_md_elems!(last_entry).remove(index);
            }

            offset += remove_counts;
            insert_row_index -= remove_counts;
        } else {
            // should not update it, because it has been verified.
            if !(md_elems[rows - 1].ty == MdElementType::Math
                || md_elems[rows - 1].ty == MdElementType::ImageUrl)
            {
                store_current_chat_session_histories_md_elems!(last_entry)
                    .set_row_data(rows - 1, md_elems[rows - 1].clone().into());
            }
        }

        for i in 0..offset {
            store_current_chat_session_histories_md_elems!(last_entry)
                .push(md_elems[insert_row_index + i].clone().into());
        }
    }
}

pub fn parse_last_history_bot_text(ui: &AppWindow) {
    let rows = store_current_chat_session_histories!(ui).row_count();
    if rows == 0 {
        return;
    }

    let entry = store_current_chat_session_histories!(ui)
        .row_data(rows - 1)
        .unwrap();

    if entry.bot.trim().is_empty() {
        return;
    }

    let (md_elems, _) = dummy_markdown::parser::run(&entry.bot, can_parse_math());

    let elems = md_elems
        .into_iter()
        .map(|item| item.into())
        .collect::<Vec<_>>();
    store_current_chat_session_histories_md_elems!(entry).set_vec(elems);
}

pub fn parse_histories_bot_text(ui: &AppWindow) {
    for entry in store_current_chat_session_histories!(ui).iter() {
        if entry.bot.trim().is_empty() {
            continue;
        }

        let (md_elems, _) = dummy_markdown::parser::run(&entry.bot, can_parse_math());

        let elems = md_elems
            .into_iter()
            .map(|item| item.into())
            .collect::<Vec<_>>();
        store_current_chat_session_histories_md_elems!(entry).set_vec(elems);
    }
}

fn get_md_entry(ui: &AppWindow, histories_entry_index: usize, index: usize) -> Option<UIMdElement> {
    if let Some(entry) = store_current_chat_session_histories!(ui).row_data(histories_entry_index)
        && let Some(item) = store_current_chat_session_histories_md_elems!(entry).row_data(index)
    {
        return Some(item);
    }

    None
}

fn set_md_entry(ui: &AppWindow, histories_entry_index: usize, index: usize, md_entry: UIMdElement) {
    if let Some(entry) = store_current_chat_session_histories!(ui).row_data(histories_entry_index)
        && store_current_chat_session_histories_md_elems!(entry).row_count() > index
    {
        store_current_chat_session_histories_md_elems!(entry).set_row_data(index, md_entry);
    }
}

fn async_load_math(
    ui: Weak<AppWindow>,
    histories_entry_index: usize,
    index: usize,
    formula: SharedString,
    file_path: std::path::PathBuf,
) {
    _ = slint::invoke_from_event_loop(move || {
        let ui = ui.unwrap();

        if let Some(mut entry) = get_md_entry(&ui, histories_entry_index, index) {
            if entry.math.formula != formula {
                return;
            }
            if let Ok(img) = Image::load_from_path(&file_path) {
                entry.math.img = img;
                entry.math.is_loaded = true;
                set_md_entry(&ui, histories_entry_index, index, entry);
            } else {
                _ = std::fs::remove_file(&file_path);
            }
        }
    });
}

fn can_parse_math() -> bool {
    static CACHE: OnceCell<bool> = OnceCell::new();
    *CACHE.get_or_init(|| duct::cmd!("latex-image", "--version").run().is_ok())
}
