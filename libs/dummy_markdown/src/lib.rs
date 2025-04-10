pub mod parser;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum UIMarkdownElementType {
    #[default]
    Text,
    ImageUrl,
    ListItem,
    Heading,
    CodeBlock,
}

#[derive(Debug, Clone, Default)]
pub struct UIUrl {
    pub text: String,
    pub url: String,
}

#[derive(Debug, Clone, Default)]
pub struct UIHeading {
    pub level: i32,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct UIListItem {
    pub level: i32,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct UIMarkdownElement {
    pub ty: UIMarkdownElementType,
    pub text: String,
    pub image_url: String,
    pub code_block: String,
    pub list_item: UIListItem,
    pub heading: UIHeading,
}
