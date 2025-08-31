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
    TableHead,
    TableRow,
    TableCell,
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

#[derive(Debug, Clone, Default)]
pub struct MdElement {
    pub ty: MdElementType,
    pub text: String,
    pub math: String,
    pub image_url: String,
    pub code_block: MdCodeBlock,
    pub list_item: MdListItem,
    pub heading: MdHeading,

    pub table: MdTable,
    pub table_cell: String,
    pub table_head: Vec<String>,
    pub table_row: Vec<String>,
}
