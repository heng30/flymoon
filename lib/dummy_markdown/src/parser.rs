use super::*;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use regex::Regex;

#[derive(Debug, Clone)]
enum MarkdownElement {
    Text(String),
    Math(String),
    Url(String),
    List(Vec<MarkdownElement>),
    ListItem(Vec<MarkdownElement>),
    Link(Vec<MarkdownElement>),
    Image(Vec<MarkdownElement>),
    CodeBlock(Vec<MarkdownElement>),
    Paragraph(Vec<MarkdownElement>),

    TableHead(Vec<MarkdownElement>),
    TableRow(Vec<MarkdownElement>),
    TableCell(Vec<MarkdownElement>),
    Table(Vec<MarkdownElement>),

    Heading {
        level: HeadingLevel,
        elems: Vec<MarkdownElement>,
    },
}

#[derive(Debug, Clone, Default)]
struct GenerateMdElemUserData {
    list_level: i32,
    link_urls: Vec<MdUrl>,
}

pub fn run(doc: &str, parser_math: bool) -> (Vec<MdElement>, Vec<MdUrl>) {
    let mut items = vec![];
    let mut link_urls = vec![];

    if parser_math {
        for item in split_text_and_latex(doc) {
            match item {
                MarkdownElement::Text(text) => {
                    let mut options = Options::empty();
                    options.insert(Options::ENABLE_TABLES);

                    let mut parser = Parser::new_ext(&text, options);
                    let mut elems = parse_events(&mut parser);
                    log::trace!("{:#?}", elems);
                    // println!("{:#?}", elems);

                    let mut ui_elems = vec![];
                    let mut elems_iter = elems.iter_mut();
                    let elems_iter_ref: &mut dyn Iterator<Item = &mut MarkdownElement> =
                        &mut elems_iter;
                    let mut user_data = GenerateMdElemUserData::default();

                    generate_ui_elements(elems_iter_ref, &mut ui_elems, &mut user_data);

                    items.extend(ui_elems.into_iter());
                    link_urls.extend(user_data.link_urls.into_iter());
                }
                MarkdownElement::Math(formula) => items.push(MdElement {
                    ty: MdElementType::Math,
                    math: formula.into(),
                    ..Default::default()
                }),
                _ => unreachable!(),
            }
        }
    } else {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);

        let mut parser = Parser::new_ext(doc, options);
        let mut elems = parse_events(&mut parser);
        log::trace!("{:#?}", elems);
        // println!("{:#?}", elems);

        let mut ui_elems = vec![];
        let mut elems_iter = elems.iter_mut();
        let elems_iter_ref: &mut dyn Iterator<Item = &mut MarkdownElement> = &mut elems_iter;
        let mut user_data = GenerateMdElemUserData::default();

        generate_ui_elements(elems_iter_ref, &mut ui_elems, &mut user_data);

        items.extend(ui_elems.into_iter());
        link_urls.extend(user_data.link_urls.into_iter());
    }

    (items, link_urls)
}

fn heading_level_from(level: &HeadingLevel) -> i32 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn parse_events(parser: &mut Parser<'_>) -> Vec<MarkdownElement> {
    let mut elems: Vec<MarkdownElement> = vec![];

    while let Some(event) = parser.next() {
        let items = &mut elems;

        // println!("{event:?}");

        match event {
            Event::Start(Tag::Paragraph) => {
                items.push(MarkdownElement::Paragraph(parse_events(parser)));
            }
            Event::End(TagEnd::Paragraph) => {
                return elems;
            }
            Event::Text(text) => {
                items.push(MarkdownElement::Text(text.into_string()));
            }
            Event::Code(code) => {
                items.push(MarkdownElement::Text(code.into_string()));
            }
            Event::Start(Tag::CodeBlock(_kind)) => {
                items.push(MarkdownElement::CodeBlock(parse_events(parser)));
            }
            Event::End(TagEnd::CodeBlock) => {
                return elems;
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                let mut link_items = parse_events(parser);
                link_items.push(MarkdownElement::Url(dest_url.into_string()));
                items.push(MarkdownElement::Link(link_items));
            }
            Event::End(TagEnd::Link) => {
                return elems;
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                let mut img_items = parse_events(parser);
                img_items.push(MarkdownElement::Url(dest_url.into_string()));
                items.push(MarkdownElement::Image(img_items));
            }
            Event::End(TagEnd::Image) => {
                return elems;
            }
            Event::Start(Tag::Heading { level, .. }) => {
                items.push(MarkdownElement::Heading {
                    level,
                    elems: parse_events(parser),
                });
            }
            Event::End(TagEnd::Heading(_level)) => {
                return elems;
            }
            Event::Start(Tag::List(_)) => {
                items.push(MarkdownElement::List(parse_events(parser)));
            }
            Event::End(TagEnd::List(_)) => {
                return elems;
            }
            Event::Start(Tag::Item) => {
                items.push(MarkdownElement::ListItem(parse_events(parser)));
            }
            Event::End(TagEnd::Item) => {
                return elems;
            }
            Event::Start(Tag::Table(_)) => {
                items.push(MarkdownElement::Table(parse_events(parser)));
            }
            Event::End(TagEnd::Table) => {
                return elems;
            }
            Event::Start(Tag::TableHead) => {
                items.push(MarkdownElement::TableHead(parse_events(parser)));
            }
            Event::End(TagEnd::TableHead) => {
                return elems;
            }
            Event::Start(Tag::TableRow) => {
                items.push(MarkdownElement::TableRow(parse_events(parser)));
            }
            Event::End(TagEnd::TableRow) => {
                return elems;
            }
            Event::Start(Tag::TableCell) => {
                items.push(MarkdownElement::TableCell(parse_events(parser)));
            }
            Event::End(TagEnd::TableCell) => {
                return elems;
            }
            _ => {}
        }
    }

    elems
}

fn generate_ui_elements(
    elems_iter: &mut dyn Iterator<Item = &mut MarkdownElement>,
    ui_elems: &mut Vec<MdElement>,
    user_data: &mut GenerateMdElemUserData,
) {
    while let Some(elem) = elems_iter.next() {
        match elem {
            MarkdownElement::Text(text) => {
                ui_elems.push(MdElement {
                    ty: MdElementType::Text,
                    text: text.clone().into(),
                    ..Default::default()
                });
            }
            MarkdownElement::Link(elems) => {
                if elems.len() != 2 {
                    continue;
                }

                let mut ui_url = MdUrl::default();
                for item in elems.iter() {
                    match item {
                        MarkdownElement::Text(text) => {
                            ui_url.text = text.clone();

                            ui_elems.push(MdElement {
                                ty: MdElementType::Text,
                                text: text.clone().into(),
                                ..Default::default()
                            });
                        }
                        MarkdownElement::Url(url) => {
                            ui_url.url = url.clone();
                        }
                        _ => (),
                    }
                }

                user_data.link_urls.push(ui_url);
            }
            MarkdownElement::Image(elems) => {
                for item in elems.iter() {
                    match item {
                        MarkdownElement::Url(url) => {
                            ui_elems.push(MdElement {
                                ty: MdElementType::ImageUrl,
                                image_url: url.clone().into(),
                                ..Default::default()
                            });
                        }
                        _ => (),
                    }
                }
            }
            MarkdownElement::CodeBlock(elems) => {
                let mut code = String::default();

                for item in elems.iter() {
                    match item {
                        MarkdownElement::Text(text) => {
                            code.push_str(&text);
                        }
                        _ => (),
                    }
                }

                ui_elems.push(MdElement {
                    ty: MdElementType::CodeBlock,
                    code_block: code.into(),
                    ..Default::default()
                });
            }
            MarkdownElement::Heading { level, elems } => {
                let mut heading_elems = vec![];
                let mut elems_iter = elems.iter_mut();
                let mut elems_iter_ref: &mut dyn Iterator<Item = &mut MarkdownElement> =
                    &mut elems_iter;

                generate_ui_elements(&mut elems_iter_ref, &mut heading_elems, user_data);
                let mut heading_text = String::default();
                for item in heading_elems.iter() {
                    if item.ty == MdElementType::Text {
                        heading_text.push_str(&item.text);
                    }
                }

                ui_elems.push(MdElement {
                    ty: MdElementType::Heading,
                    heading: MdHeading {
                        level: heading_level_from(level),
                        text: heading_text,
                    },
                    ..Default::default()
                });
            }
            MarkdownElement::Paragraph(elems) => {
                let mut paragraph_elems = vec![];
                let mut elems_iter = elems.iter_mut();
                let mut elems_iter_ref: &mut dyn Iterator<Item = &mut MarkdownElement> =
                    &mut elems_iter;

                generate_ui_elements(&mut elems_iter_ref, &mut paragraph_elems, user_data);

                let mut text = String::default();
                for item in paragraph_elems.into_iter() {
                    if item.ty == MdElementType::Text {
                        text.push_str(&item.text);
                    } else {
                        if !text.is_empty() {
                            ui_elems.push(MdElement {
                                ty: MdElementType::Text,
                                text: text.clone(),
                                ..Default::default()
                            });
                            text.clear();
                        }
                        ui_elems.push(item);
                    }
                }

                if !text.is_empty() {
                    ui_elems.push(MdElement {
                        ty: MdElementType::Text,
                        text,
                        ..Default::default()
                    });
                }
            }
            MarkdownElement::List(elems) => {
                let mut list_elems = vec![];
                let mut elems_iter = elems.iter_mut();
                let mut elems_iter_ref: &mut dyn Iterator<Item = &mut MarkdownElement> =
                    &mut elems_iter;

                user_data.list_level += 1;
                generate_ui_elements(&mut elems_iter_ref, &mut list_elems, user_data);
                user_data.list_level -= 1;

                ui_elems.extend(list_elems);
            }
            MarkdownElement::ListItem(elems) => {
                let mut list_item_elems = vec![];
                let mut elems_iter = elems.iter_mut();
                let mut elems_iter_ref: &mut dyn Iterator<Item = &mut MarkdownElement> =
                    &mut elems_iter;

                generate_ui_elements(&mut elems_iter_ref, &mut list_item_elems, user_data);

                let mut list_item_text = String::default();
                let mut code_block = String::default();

                for item in list_item_elems.iter() {
                    if item.ty == MdElementType::Text {
                        list_item_text.push_str(&item.text);
                    } else if item.ty == MdElementType::ListItem {
                        if !list_item_text.is_empty() {
                            ui_elems.push(MdElement {
                                ty: MdElementType::ListItem,
                                list_item: MdListItem {
                                    level: user_data.list_level,
                                    text: list_item_text.clone(),
                                },
                                ..Default::default()
                            });
                            list_item_text.clear();
                        }

                        ui_elems.push(item.clone());
                    } else if item.ty == MdElementType::CodeBlock {
                        code_block.push_str(&item.code_block);
                    }
                }

                if !list_item_text.is_empty() {
                    ui_elems.push(MdElement {
                        ty: MdElementType::ListItem,
                        list_item: MdListItem {
                            level: user_data.list_level,
                            text: list_item_text,
                        },
                        ..Default::default()
                    });
                }

                if !code_block.is_empty() {
                    ui_elems.push(MdElement {
                        ty: MdElementType::CodeBlock,
                        code_block: code_block,
                        ..Default::default()
                    });
                }
            }
            MarkdownElement::Table(elems) => {
                let mut table_item_elems = vec![];
                let mut elems_iter = elems.iter_mut();
                let mut elems_iter_ref: &mut dyn Iterator<Item = &mut MarkdownElement> =
                    &mut elems_iter;

                generate_ui_elements(&mut elems_iter_ref, &mut table_item_elems, user_data);

                let (mut head, mut rows) = (vec![], vec![]);

                for item in table_item_elems.into_iter() {
                    if item.ty == MdElementType::TableHead {
                        head.extend(item.table_head);
                    } else if item.ty == MdElementType::TableRow {
                        rows.push(item.table_row);
                    }
                }

                ui_elems.push(MdElement {
                    ty: MdElementType::Table,
                    table: MdTable { head, rows },
                    ..Default::default()
                });
            }
            MarkdownElement::TableHead(elems) => {
                let mut table_head_item_elems = vec![];
                let mut elems_iter = elems.iter_mut();
                let mut elems_iter_ref: &mut dyn Iterator<Item = &mut MarkdownElement> =
                    &mut elems_iter;

                generate_ui_elements(&mut elems_iter_ref, &mut table_head_item_elems, user_data);

                let table_head = table_head_item_elems
                    .into_iter()
                    .filter(|item| item.ty == MdElementType::TableCell)
                    .map(|item| item.table_cell)
                    .collect::<Vec<String>>();

                ui_elems.push(MdElement {
                    ty: MdElementType::TableHead,
                    table_head,
                    ..Default::default()
                });
            }
            MarkdownElement::TableRow(elems) => {
                let mut table_row_item_elems = vec![];
                let mut elems_iter = elems.iter_mut();
                let mut elems_iter_ref: &mut dyn Iterator<Item = &mut MarkdownElement> =
                    &mut elems_iter;

                generate_ui_elements(&mut elems_iter_ref, &mut table_row_item_elems, user_data);

                let table_row = table_row_item_elems
                    .into_iter()
                    .filter(|item| item.ty == MdElementType::TableCell)
                    .map(|item| item.table_cell)
                    .collect::<Vec<String>>();

                ui_elems.push(MdElement {
                    ty: MdElementType::TableRow,
                    table_row,
                    ..Default::default()
                });
            }
            MarkdownElement::TableCell(elems) => {
                let mut table_cell_item_elems = vec![];
                let mut elems_iter = elems.iter_mut();
                let mut elems_iter_ref: &mut dyn Iterator<Item = &mut MarkdownElement> =
                    &mut elems_iter;

                generate_ui_elements(&mut elems_iter_ref, &mut table_cell_item_elems, user_data);

                let mut text = String::default();
                for item in table_cell_item_elems.into_iter() {
                    if item.ty == MdElementType::Text {
                        text.push_str(&item.text);
                    }
                }

                ui_elems.push(MdElement {
                    ty: MdElementType::TableCell,
                    table_cell: text,
                    ..Default::default()
                });
            }
            _ => (),
        }
    }
}

fn split_text_and_latex(text: &str) -> Vec<MarkdownElement> {
    let mut last_end = 0;
    let mut items = vec![];
    let re = Regex::new(r"(?s)\\\((.*?)\\\)|\\\[(.*?)\\\]").unwrap();

    for mat in re.find_iter(text) {
        let start = mat.start();

        // normal text
        if last_end < start {
            items.push(MarkdownElement::Text(text[last_end..start].to_string()));
        }

        // LaTeX
        let formula = &text[mat.start()..mat.end()];
        let formula = formula
            .trim_start_matches("\\(")
            .trim_end_matches("\\)")
            .trim()
            .to_string();

        let formula = formula
            .trim_start_matches("\\[")
            .trim_end_matches("\\]")
            .trim()
            .to_string();

        items.push(MarkdownElement::Math(formula));

        last_end = mat.end();
    }

    // last normal text if exist
    if last_end < text.len() {
        items.push(MarkdownElement::Text(text[last_end..].to_string()));
    }

    items
}
