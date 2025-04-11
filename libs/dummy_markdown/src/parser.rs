use super::*;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
enum MarkdownElement {
    Text(String),
    Url(String),
    List(Vec<MarkdownElement>),
    ListItem(Vec<MarkdownElement>),
    Link(Vec<MarkdownElement>),
    Image(Vec<MarkdownElement>),
    CodeBlock(Vec<MarkdownElement>),
    Paragraph(Vec<MarkdownElement>),

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

pub fn run(doc: &str) -> (Vec<MdElement>, Vec<MdUrl>) {
    let options = Options::empty();
    let mut parser = Parser::new_ext(doc, options);

    let mut elems = parse_events(&mut parser);
    log::trace!("{:#?}", elems);

    let mut ui_elems = vec![];
    let mut elems_iter = elems.iter_mut();
    let elems_iter_ref: &mut dyn Iterator<Item = &mut MarkdownElement> = &mut elems_iter;
    let mut user_data = GenerateMdElemUserData::default();
    generate_ui_elements(elems_iter_ref, &mut ui_elems, &mut user_data);

    (ui_elems, user_data.link_urls)
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
                for item in elems.iter() {
                    match item {
                        MarkdownElement::Text(text) => {
                            ui_elems.push(MdElement {
                                ty: MdElementType::CodeBlock,
                                code_block: text.clone().into(),
                                ..Default::default()
                            });
                        }
                        _ => (),
                    }
                }
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
                ui_elems.extend(paragraph_elems);
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
                for item in list_item_elems.iter() {
                    if item.ty == MdElementType::Text {
                        list_item_text.push_str(&item.text);
                    }
                    if item.ty == MdElementType::ListItem {
                        ui_elems.push(item.clone());
                    }
                }

                ui_elems.push(MdElement {
                    ty: MdElementType::ListItem,
                    list_item: MdListItem {
                        level: user_data.list_level,
                        text: list_item_text,
                    },
                    ..Default::default()
                });
            }
            _ => (),
        }
    }
}
