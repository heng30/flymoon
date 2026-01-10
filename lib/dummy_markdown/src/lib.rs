pub mod parser;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum MdElementType {
    #[default]
    Text,
    Math,
    ImageUrl,
    ListItem,
    Heading,
    CodeBlock,
    Table,
}

#[derive(Debug, Clone, Default)]
pub struct MdUrl {
    pub text: String,
    pub url: String,
}

#[derive(Debug, Clone, Default)]
pub struct MdHeading {
    pub level: i32,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct MdListItem {
    pub level: i32,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct MdTable {
    pub head: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct MdCodeBlock {
    pub lang: String,
    pub code: String,
}

#[derive(Debug, Clone)]
pub enum MdElement {
    Text(String),
    Math(String),
    ImageUrl(String),

    Paragraph(Vec<MdElement>),
    List(Vec<MdElement>),
    ListItem(Vec<MdElement>),
    Link { text: Vec<MdElement>, url: String },

    Heading(MdHeading),
    CodeBlock(MdCodeBlock),
    Table(MdTable),

    FlatListItem(MdListItem),
}

impl MdElement {
    pub fn ty(&self) -> MdElementType {
        match self {
            MdElement::Text(_) => MdElementType::Text,
            MdElement::Math(_) => MdElementType::Math,
            MdElement::ImageUrl(_) => MdElementType::ImageUrl,
            MdElement::Heading(_) => MdElementType::Heading,
            MdElement::CodeBlock(_) => MdElementType::CodeBlock,
            MdElement::Table(_) => MdElementType::Table,
            MdElement::ListItem(_) | MdElement::FlatListItem(_) => MdElementType::ListItem,
            MdElement::Paragraph(_) | MdElement::List(_) | MdElement::Link { .. } => {
                MdElementType::Text
            }
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            MdElement::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_math(&self) -> Option<&str> {
        match self {
            MdElement::Math(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_image_url(&self) -> Option<&str> {
        match self {
            MdElement::ImageUrl(s) => Some(s),
            _ => None,
        }
    }
}
