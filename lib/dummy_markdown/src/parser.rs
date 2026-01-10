use super::*;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, Default)]
struct GenerateMdElemUserData {
    list_level: i32,
    link_urls: Vec<MdUrl>,
}

pub fn run(doc: &str, parser_math: bool) -> (Vec<MdElement>, Vec<MdUrl>) {
    let doc = if !parser_math {
        doc.to_string()
    } else {
        preprocess_math(doc)
    };

    let (ui_elems, user_data) = parse_text(&doc);
    (ui_elems, user_data.link_urls)
}

fn parse_text(text: &str) -> (Vec<MdElement>, GenerateMdElemUserData) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);

    let mut parser = Parser::new_ext(text, options);
    let mut elems = parse_events(&mut parser);
    log::trace!("{:#?}", elems);

    let mut ui_elems = vec![];
    let mut user_data = GenerateMdElemUserData::default();
    process_elements(&mut elems, &mut ui_elems, &mut user_data);

    (ui_elems, user_data)
}

fn heading_level_from(level: &HeadingLevel) -> i32 {
    match *level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn parse_events(parser: &mut Parser<'_>) -> Vec<MdElement> {
    let mut elems: Vec<MdElement> = vec![];

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::Paragraph) => elems.push(MdElement::Paragraph(parse_events(parser))),
            Event::End(TagEnd::Paragraph) => return elems,
            Event::Text(text) => elems.push(MdElement::Text(text.into_string())),
            Event::Code(code) => {
                let code_str = code.into_string();
                // Check if this is inline math (starts with "math:")
                if let Some(formula) = code_str.strip_prefix("math:") {
                    elems.push(MdElement::Math(formula.to_string()));
                } else {
                    elems.push(MdElement::Text(code_str));
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    _ => String::default(),
                };
                let code_contents = parse_events(parser);
                let code = extract_text(&code_contents);

                // Check if this is a math code block
                if lang == "math" {
                    elems.push(MdElement::Math(code));
                } else {
                    elems.push(MdElement::CodeBlock(MdCodeBlock { lang, code }));
                }
            }
            Event::End(TagEnd::CodeBlock) => return elems,
            Event::Start(Tag::Link { dest_url, .. }) => {
                let link_items = parse_events(parser);
                elems.push(MdElement::Link {
                    text: link_items,
                    url: dest_url.into_string(),
                });
            }
            Event::End(TagEnd::Link) => return elems,
            Event::Start(Tag::Image { dest_url, .. }) => {
                elems.push(MdElement::ImageUrl(dest_url.into_string()))
            }
            Event::End(TagEnd::Image) => return elems,
            Event::Start(Tag::Heading { level, .. }) => {
                let heading_items = parse_events(parser);
                let text = extract_text(&heading_items);
                elems.push(MdElement::Heading(MdHeading {
                    level: heading_level_from(&level),
                    text,
                }));
            }
            Event::End(TagEnd::Heading(_level)) => return elems,
            Event::Start(Tag::List(_)) => elems.push(MdElement::List(parse_events(parser))),
            Event::End(TagEnd::List(_)) => return elems,
            Event::Start(Tag::Item) => elems.push(MdElement::ListItem(parse_events(parser))),
            Event::End(TagEnd::Item) => return elems,
            Event::Start(Tag::Table(_)) => elems.push(MdElement::List(parse_events(parser))),
            Event::End(TagEnd::Table) => return elems,
            Event::Start(Tag::TableHead) => elems.push(MdElement::List(parse_events(parser))),
            Event::End(TagEnd::TableHead) => return elems,
            Event::Start(Tag::TableRow) => elems.push(MdElement::List(parse_events(parser))),
            Event::End(TagEnd::TableRow) => return elems,
            Event::Start(Tag::TableCell) => elems.push(MdElement::Paragraph(parse_events(parser))),
            Event::End(TagEnd::TableCell) => return elems,
            _ => (),
        }
    }

    elems
}

fn extract_text(elems: &[MdElement]) -> String {
    elems
        .iter()
        .filter_map(|e| match e {
            MdElement::Text(s) => Some(s.as_str()),
            _ => None,
        })
        .collect()
}

fn process_elements(
    elems: &mut [MdElement],
    ui_elems: &mut Vec<MdElement>,
    user_data: &mut GenerateMdElemUserData,
) {
    for elem in elems.iter_mut() {
        match elem {
            MdElement::Text(text) => ui_elems.push(MdElement::Text(text.clone())),
            MdElement::Math(_) => ui_elems.push(elem.clone()),
            MdElement::ImageUrl(_) => ui_elems.push(elem.clone()),
            MdElement::FlatListItem(_) => ui_elems.push(elem.clone()),
            MdElement::Link { text, url } => {
                let link_text = extract_text(text);
                if !link_text.is_empty() {
                    ui_elems.push(MdElement::Text(link_text.clone()));
                }
                user_data.link_urls.push(MdUrl {
                    text: link_text,
                    url: url.clone(),
                });
            }
            MdElement::Heading(_) | MdElement::CodeBlock(_) | MdElement::Table(_) => {
                ui_elems.push(elem.clone())
            }
            MdElement::Paragraph(children) => {
                let mut text_buffer = String::new();
                for child in children.iter() {
                    match child {
                        MdElement::Text(s) => {
                            text_buffer.push_str(s);
                        }
                        _ => {
                            if !text_buffer.is_empty() {
                                ui_elems.push(MdElement::Text(text_buffer.clone()));
                                text_buffer.clear();
                            }
                            let mut child_clone = child.clone();
                            process_elements(
                                std::slice::from_mut(&mut child_clone),
                                ui_elems,
                                user_data,
                            );
                        }
                    }
                }
                if !text_buffer.is_empty() {
                    ui_elems.push(MdElement::Text(text_buffer));
                }
            }
            MdElement::List(children) => {
                user_data.list_level += 1;
                process_elements(children, ui_elems, user_data);
                user_data.list_level -= 1;
            }
            MdElement::ListItem(children) => {
                let mut item_text = String::new();
                let mut code_block = None;

                for child in children.iter() {
                    match child {
                        MdElement::Text(s) => {
                            item_text.push_str(s);
                        }
                        MdElement::Link { text, url } => {
                            let link_text = extract_text(text);
                            item_text.push_str(&link_text);

                            user_data.link_urls.push(MdUrl {
                                text: link_text,
                                url: url.clone(),
                            });
                        }
                        MdElement::Paragraph(para_children) => {
                            for para_child in para_children.iter() {
                                match para_child {
                                    MdElement::Text(s) => item_text.push_str(s),
                                    MdElement::Link { text, url } => {
                                        let link_text = extract_text(text);
                                        item_text.push_str(&link_text);
                                        user_data.link_urls.push(MdUrl {
                                            text: link_text,
                                            url: url.clone(),
                                        });
                                    }
                                    _ => (),
                                }
                            }
                        }
                        MdElement::ListItem(_) => {
                            if !item_text.is_empty() {
                                ui_elems.push(MdElement::FlatListItem(MdListItem {
                                    level: user_data.list_level,
                                    text: item_text.clone(),
                                }));
                                item_text.clear();
                            }

                            let mut child_clone = child.clone();
                            process_elements(
                                std::slice::from_mut(&mut child_clone),
                                ui_elems,
                                user_data,
                            );
                        }
                        MdElement::CodeBlock(cb) => {
                            code_block = Some(cb.clone());
                        }
                        _ => {
                            if !item_text.is_empty() {
                                ui_elems.push(MdElement::FlatListItem(MdListItem {
                                    level: user_data.list_level,
                                    text: item_text.clone(),
                                }));
                                item_text.clear();
                            }
                            let mut child_clone = child.clone();
                            process_elements(
                                std::slice::from_mut(&mut child_clone),
                                ui_elems,
                                user_data,
                            );
                        }
                    }
                }

                if !item_text.is_empty() {
                    ui_elems.push(MdElement::FlatListItem(MdListItem {
                        level: user_data.list_level,
                        text: item_text,
                    }));
                }

                if let Some(cb) = code_block {
                    ui_elems.push(MdElement::CodeBlock(cb));
                }
            }
        }
    }
}

/// Preprocess math formulas by replacing delimiters with code blocks
/// Supports: \(...\), \[...\], $...$, $$...$$
/// - Block math (\[...\], $$...$$) -> ```math\n...\n```
/// - Inline math (\(...\), $...$) -> `...` (will be detected as math in parse_events)
fn preprocess_math(text: &str) -> String {
    let mut result = text.to_string();

    // Replace in a specific order to avoid overlapping issues:
    // 1. First replace longer delimiters ($$...$$ and \[...\])
    // 2. Then replace shorter delimiters ($...$ and \(...\))

    // Replace \[...\] with fenced code blocks
    replace_delimited(&mut result, "\\[", "\\]", |formula| {
        format!("```math\n{formula}\n```\n")
    });

    // Replace $$...$$ with fenced code blocks
    replace_delimited(&mut result, "$$", "$$", |formula| {
        format!("```math\n{formula}\n```\n")
    });

    // Replace \(...\) with inline code
    replace_delimited(&mut result, "\\(", "\\)", |formula| {
        format!("`math:{formula}`")
    });

    // Replace $...$ with inline code (but not part of $$...$$)
    // Note: $$...$$ has already been replaced, so we can safely replace $...$
    replace_delimited(&mut result, "$", "$", |formula| format!("`math:{formula}`"));

    result
}

// Helper to find and replace a delimited pattern
// Only replaces if both start and end delimiters are found
fn replace_delimited(
    text: &mut String,
    start_delim: &str,
    end_delim: &str,
    replacement_template: impl Fn(&str) -> String,
) {
    let mut search_start = 0;
    while search_start < text.len() {
        // Find start delimiter
        if let Some(start_pos) = text[search_start..].find(start_delim) {
            let start_pos = search_start + start_pos;
            let after_start = start_pos + start_delim.len();

            // Find end delimiter (only after the start)
            if let Some(end_pos) = text[after_start..].find(end_delim) {
                let end_pos = after_start + end_pos;

                // Extract and replace
                let content = text[after_start..end_pos].trim().to_string();
                let replacement = replacement_template(&content);
                text.replace_range(start_pos..end_pos + end_delim.len(), &replacement);

                // Move past the replacement
                search_start = start_pos + replacement.len();
            } else {
                // No matching end delimiter, skip this start delimiter
                search_start = after_start;
            }
        } else {
            // No more start delimiters found
            break;
        }
    }
}
