#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub metadata: Metadata,
    pub body: Vec<BlockElement>,
    pub footnotes: Vec<Footnote>,
    pub bibliography: Vec<BibliographyEntry>,
}

impl Document {
    pub fn is_presentation(&self) -> bool {
        self.body.iter().any(|b| matches!(b, BlockElement::Slide(_)))
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockElement {
    Heading(Heading),
    Paragraph(Paragraph),
    TableOfContents(TableOfContents),
    List(List),
    BlockQuote(BlockQuote),
    CodeBlock(CodeBlock),
    Figure(Figure),
    Table(Table),
    Slide(Slide),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    pub level: u8,
    pub content: Vec<InlineElement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Paragraph {
    pub content: Vec<InlineElement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableOfContents {
    pub depth: Option<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub ordered: bool,
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub content: Vec<BlockElement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockQuote {
    pub content: Vec<BlockElement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Figure {
    pub image: ImageSource,
    pub caption: Option<Vec<InlineElement>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageSource {
    pub path: String,
    pub alt: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub caption: Option<Vec<InlineElement>>,
    pub header: Vec<Row>,
    pub body: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub colspan: u8,
    pub rowspan: u8,
    pub content: Vec<BlockElement>,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            colspan: 1,
            rowspan: 1,
            content: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Slide {
    pub title: Vec<InlineElement>,
    pub notes: Option<Vec<InlineElement>>,
    pub content: Vec<BlockElement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineElement {
    Text(String),
    Emphasis(Emphasis),
    Subscript(Vec<InlineElement>),
    Superscript(Vec<InlineElement>),
    Link(Link),
    InlineCode(String),
    Citation(String),
    CrossReference(String),
    FootnoteReference(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Emphasis {
    pub level: u8,
    pub content: Vec<InlineElement>,
}

impl Default for Emphasis {
    fn default() -> Self {
        Emphasis {
            level: 1,
            content: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub target: String,
    pub content: Vec<InlineElement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Footnote {
    pub id: String,
    pub content: Vec<InlineElement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BibliographyEntry {
    pub key: String,
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}
