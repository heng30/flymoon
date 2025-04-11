pub mod parser;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum MdElementType {
    #[default]
    Text,
    ImageUrl,
    ListItem,
    Heading,
    CodeBlock,
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
pub struct MdElement {
    pub ty: MdElementType,
    pub text: String,
    pub image_url: String,
    pub code_block: String,
    pub list_item: MdListItem,
    pub heading: MdHeading,
}
