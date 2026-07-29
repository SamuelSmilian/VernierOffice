# IR Design and Compiler Architecture

## Pipeline

```
Source (.vo)
  │
  ▼
Lexer ──► Token stream
  │
  ▼
Parser ──► Document (IR)
  │
  ▼
Backend ──► Output (.docx, .pptx, .html)
```

### Stages

1. **Lexer**: Converts source text into a token stream. Tokens are lightweight: commands, braces, text, newlines, and environment markers. The lexer handles escape sequences (`\\`, `\{`, etc.) and strips comments.

2. **Parser**: Consumes the token stream and produces a `Document`. The parser is a recursive descent parser matching the grammar defined in the DSL syntax specification. It validates nesting, matches `\begin{}`/`\end{}` pairs, and reports errors with source locations.

3. **Backend**: Consumes a `Document` and emits the target format. Backends implement a common trait. Each backend is responsible for its own formatting decisions (fonts, sizes, spacing, etc.).

## Module Structure

```
src/
├── main.rs              # CLI: compile command
├── lib.rs               # Public API: parse(), compile()
├── ir.rs                # IR types
├── lexer.rs             # Lexer
├── parser.rs            # Parser
├── backend/
│   ├── mod.rs           # Backend trait
│   ├── docx.rs          # DOCX backend
│   └── pptx.rs          # PPTX backend
```

## IR Types

The IR is a set of owned Rust types that directly model the Semantic Model.

### Document

```rust
pub struct Document {
    pub metadata: Metadata,
    pub body: Vec<BlockElement>,
    pub footnotes: Vec<Footnote>,
    pub bibliography: Vec<BibliographyEntry>,
}
```

### Metadata

```rust
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub language: Option<String>,
}
```

### BlockElement

```rust
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
```

### Heading

```rust
pub struct Heading {
    pub level: u8,
    pub content: Vec<InlineElement>,
}
```

### Paragraph

```rust
pub struct Paragraph {
    pub content: Vec<InlineElement>,
}
```

### TableOfContents

```rust
pub struct TableOfContents {
    pub depth: Option<u8>,
}
```

### List

```rust
pub struct List {
    pub ordered: bool,
    pub items: Vec<ListItem>,
}

pub struct ListItem {
    pub content: Vec<BlockElement>,
}
```

### BlockQuote

```rust
pub struct BlockQuote {
    pub content: Vec<BlockElement>,
}
```

### CodeBlock

```rust
pub struct CodeBlock {
    pub language: Option<String>,
    pub content: String,
}
```

### Figure

```rust
pub struct Figure {
    pub image: ImageSource,
    pub caption: Option<Vec<InlineElement>>,
}

pub struct ImageSource {
    pub path: String,
    pub alt: Option<String>,
}
```

### Table

```rust
pub struct Table {
    pub caption: Option<Vec<InlineElement>>,
    pub header: Vec<Row>,
    pub body: Vec<Row>,
}

pub struct Row {
    pub cells: Vec<Cell>,
}

pub struct Cell {
    pub colspan: u8,
    pub rowspan: u8,
    pub content: Vec<BlockElement>,
}
```

### Slide

```rust
pub struct Slide {
    pub title: Vec<InlineElement>,
    pub notes: Option<Vec<InlineElement>>,
    pub content: Vec<BlockElement>,
}
```

### InlineElement

```rust
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
```

### Emphasis

```rust
pub struct Emphasis {
    pub level: u8,
    pub content: Vec<InlineElement>,
}
```

### Link

```rust
pub struct Link {
    pub target: String,
    pub content: Vec<InlineElement>,
}
```

### Footnotes and Bibliography

```rust
pub struct Footnote {
    pub id: String,
    pub content: Vec<InlineElement>,
}

pub struct BibliographyEntry {
    pub key: String,
    pub fields: Vec<(String, String)>,
}
```

## Backend Trait

```rust
pub trait Backend {
    /// Compile a document to the writer in the target format.
    fn compile<W: Write + Seek>(&self, document: &Document, writer: W) -> Result<(), Error>;
}
```

Each backend implements this trait. The backend receives the IR and writes the output to a writer that supports both `Write` and `Seek` (required for ZIP-based OOXML formats).

## Source Locations

For error reporting, each IR node carries an optional source location:

```rust
pub struct Span {
    pub line: usize, // 1-based
    pub col: usize,  // 1-based, byte offset from line start
}
```

Parser errors include the span where the error occurred:

```rust
pub enum ParseError {
    UnexpectedToken { expected: String, found: String, span: Span },
    UnmatchedEnvironment { name: String, opened: Span },
    MissingArgument { command: String, span: Span },
    InvalidHeadingLevel { value: String, span: Span },
    // ...
}
```

## Default Values

When constructing IR nodes with optional fields:

| Field | Default |
|-------|---------|
| `Emphasis.level` | 1 |
| `Cell.colspan` | 1 |
| `Cell.rowspan` | 1 |
| `TableOfContents.depth` | None (all levels) |
| `ImageSource.alt` | None |

## Validation

After parsing, the IR is validated:

1. All `\ref{label}` and `\cite{key}` targets must exist.
2. All `\footnoteref{id}` references must match a `\footnote{id}{...}`.
3. Heading levels must not skip (level 3 after level 1 without level 2).
4. Table rows must have the same number of cells (accounting for colspan).

Validation errors are reported as warnings or errors depending on severity.

## CLI

```
vernier-office compile <input.vo> [--output <path>] [--format <docx|pptx>]
```

The format is inferred from the output file extension if not specified. If neither is given, defaults to `docx` with the input filename.

## DOCX Backend

The DOCX backend generates an OOXML package (.docx is a ZIP file with XML contents).

Mapping:

| IR Element | OOXML Element |
|------------|---------------|
| `Document` | `word/document.xml` |
| `Metadata` | `docProps/core.xml` |
| `Heading` | `w:p` + `w:pStyle` with heading style |
| `Paragraph` | `w:p` |
| `Emphasis(1)` | `w:r` + `w:i` |
| `Emphasis(2)` | `w:r` + `w:b` |
| `Emphasis(3+)` | `w:r` + `w:b` + `w:i` |
| `Subscript` | `w:r` + `w:vertAlign` = "subscript" |
| `Superscript` | `w:r` + `w:vertAlign` = "superscript" |
| `Link` | `w:hyperlink` + `w:r` |
| `InlineCode` | `w:r` + `w:rFonts` (monospace) |
| `List` (unordered) | `w:p` + `w:numPr` (bullet) |
| `List` (ordered) | `w:p` + `w:numPr` (decimal) |
| `BlockQuote` | `w:p` + `w:ind` (indented) |
| `CodeBlock` | `w:p` + `w:rFonts` (monospace, preserve whitespace) |
| `Figure` | `w:drawing` + image relationship |
| `Table` | `w:tbl` |
| `Footnote` | `w:footnote` |
| `Citation` | `w:r` + formatted citation text |
| `CrossReference` | `w:r` + cross-reference field |
| `TableOfContents` | `w:sdt` + TOC field |

### DOCX File Structure

The generated .docx package contains:

```
output.docx
├── [Content_Types].xml
├── _rels/
│   └── .rels
├── word/
│   ├── document.xml
│   ├── styles.xml
│   ├── footnotes.xml
│   ├── settings.xml
│   └── _rels/
│       └── document.xml.rels
└── docProps/
    ├── core.xml
    └── app.xml
```

Images referenced by figures are copied into `word/media/` and referenced via relationships.

## PPTX Backend

The PPTX backend generates a PresentationML package.

| IR Element | OOXML Element |
|------------|---------------|
| `Slide` | `p:sld` in `ppt/slides/slideN.xml` |
| `Slidetitle` | `p:sp` + title placeholder |
| `Spidenotes` | `p:notes` in `ppt/notesSlides/notesSlideN.xml` |
| `Paragraph` | `p:sp` + `a:p` |
| `List` | `p:sp` + `a:p` + `a:buChar`/`a:buAutoNum` |
| `Figure` | `p:sp` + image |
| `Table` | `a:tbl` |

### PPTX File Structure

```
output.pptx
├── [Content_Types].xml
├── _rels/
│   └── .rels
├── ppt/
│   ├── presentation.xml
│   ├── slideMasters/
│   │   └── slideMaster1.xml
│   ├── slideLayouts/
│   │   └── slideLayout1.xml
│   ├── slides/
│   │   └── slide1.xml
│   ├── notesSlides/
│   │   └── notesSlide1.xml
│   └── _rels/
│       └── presentation.xml.rels
└── docProps/
    ├── core.xml
    └── app.xml
```

## Implementation Order

1. IR types (`ir.rs`) — data structures
2. Lexer (`lexer.rs`) — source → tokens
3. Parser (`parser.rs`) — tokens → IR
4. Backend trait (`backend/mod.rs`)
5. CLI (`main.rs`) — argument parsing, pipeline orchestration
6. DOCX backend (`backend/docx.rs`) — IR → .docx
7. PPTX backend (`backend/pptx.rs`) — IR → .pptx
8. Error handling and validation
