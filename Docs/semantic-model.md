# Semantic Model

The Semantic Model defines the abstract structure of a VernierOffice document.

It is independent of syntax, implementation language, and output format.

It is the foundation of the IR (Intermediate Representation) and the DSL (Domain-Specific Language).

## Document

A document is a metadata block followed by a sequence of block-level elements.

```
Document ::= {
    metadata: Metadata,
    body: [BlockElement],
    footnotes: [Footnote],
    bibliography: [BibliographyEntry],
}
```

Footnotes and bibliography entries are collected at the document level but referenced from the body.

## Metadata

```
Metadata ::= {
    title:       Text?,
    author:      Text?,
    date:        Text?,
    language:    Text?,
}
```

All metadata fields are optional. The backend uses them for title pages, document properties, and locale-specific formatting.

## Block Elements

A block element occupies the full width of its container and stacks vertically.

### Heading

```
Heading ::= {
    level:   u8,        // 1-based depth
    content: [InlineElement],
}
```

The heading level determines its position in the document hierarchy and table of contents. Level 1 is the top-level section heading (not the document title).

### Paragraph

```
Paragraph ::= {
    content: [InlineElement],
}
```

The fundamental text block. May contain any inline content.

### Table of Contents

```
TableOfContents ::= {
    depth: u8?,          // max heading level; None = all levels
}
```

A placeholder that the backend replaces with a generated table of contents derived from the heading structure.

### List

```
List ::= {
    ordered: bool,
    items:   [ListItem],
}
```

Ordered lists convey sequence or priority. Unordered lists convey set membership. The backend chooses the marker style (numbers, bullets, etc.).

### ListItem

```
ListItem ::= {
    content: [BlockElement],
}
```

A single list item may contain multiple block elements (nested paragraphs, sub-lists, code blocks).

### BlockQuote

```
BlockQuote ::= {
    content: [BlockElement],
}
```

An extended quotation set apart from the main text. May contain multiple blocks.

### CodeBlock

```
CodeBlock ::= {
    language: Text?,       // programming language for syntax highlighting
    content:  Text,
}
```

Multi-line preformatted text. Whitespace is preserved. The language hint is optional and used by backends for syntax highlighting.

### Figure

```
Figure ::= {
    image:   ImageSource,
    caption: [InlineElement]?,
}
```

A semantic grouping of an image with its caption. The figure is a block element that can be cross-referenced.

### Table

```
Table ::= {
    caption: [InlineElement]?,
    header:  [Row]?,
    body:    [Row],
}
```

A semantic table with an optional header row and caption. Each row in the header is semantically distinct from body rows.

### Row

```
Row ::= {
    cells: [Cell],
}
```

A horizontal group of cells.

### Cell

```
Cell ::= {
    colspan: u8?,            // default 1
    rowspan: u8?,            // default 1
    content: [BlockElement],
}
```

A cell may contain any block-level content, including paragraphs, lists, and nested tables. Merged cells are represented via colspan/rowspan.

### Slide

```
Slide ::= {
    title:   [InlineElement],
    notes:   [InlineElement]?,
    content: [BlockElement],
}
```

The fundamental unit of a presentation. A slide has a title, optional speaker notes, and arbitrary block content.

---

## Inline Elements

An inline element flows within a block and does not break the text line.

### Text

```
Text ::= { value: string }
```

Plain text. The leaf node of all inline content.

### Emphasis

```
Emphasis ::= {
    level:   u8,            // 1 = primary emphasis, 2 = secondary, etc.
    content: [InlineElement],
}
```

Semantic emphasis. Level 1 maps to italic in most backends; level 2 maps to bold. Higher nesting levels produce combined styling. This follows the HTML `<em>` model where nesting indicates increasing emphasis.

### Subscript

```
Subscript ::= {
    content: [InlineElement],
}
```

Text lowered relative to the baseline. Used for chemical formulas, indices, etc.

### Superscript

```
Superscript ::= {
    content: [InlineElement],
}
```

Text raised relative to the baseline. Used for exponents, footnote markers, ordinals, etc.

### Link

```
Link ::= {
    target:  Text,          // URL or internal label
    content: [InlineElement],
}
```

A navigable hyperlink. The target may be an external URL or an internal cross-reference label.

### InlineCode

```
InlineCode ::= {
    content: Text,
}
```

Inline code, command, or literal text. Rendered in a monospace font by backends.

### Citation

```
Citation ::= {
    key: Text,
}
```

A reference to a bibliography entry. The backend formats the citation and links to the bibliography.

### CrossReference

```
CrossReference ::= {
    label: Text,
}
```

A reference to a labelled element (heading, figure, table) within the document. The backend resolves it to the correct number and text.

### FootnoteReference

```
FootnoteReference ::= {
    id: Text,
}
```

An inline marker that references a footnote by its identifier. The backend replaces it with a superscript number and places the footnote content at the page bottom.

---

## Reference Elements

Elements collected at the document level and referenced from inline content.

### Footnote

```
Footnote ::= {
    id:      Text,
    content: [BlockElement],
}
```

An explanatory note. Referenced by `FootnoteReference { id }` in the body. The backend places the content appropriately (page bottom or end of document).

### BibliographyEntry

```
BibliographyEntry ::= {
    key:   Text,
    fields: { Text: Text },    // field name → value (author, title, year, ...)
}
```

A single bibliography entry keyed by citation key. The backend formats entries according to its configured style.

---

## Resources

### ImageSource

```
ImageSource ::= {
    path: Text,              // path to image file
    alt:  Text?,             // accessibility description
}
```

References an external image file. The path is relative to the document source.

---

## Element Taxonomy

| Element | Kind | Category |
|---------|------|----------|
| Heading | Block | Document Structure |
| Paragraph | Block | Document Structure |
| TableOfContents | Block | Document Structure |
| List | Block | Presentation |
| ListItem | Block | Presentation |
| BlockQuote | Block | Presentation |
| CodeBlock | Block | Presentation |
| Figure | Block | Figures |
| Table | Block | Tables |
| Row | Block | Tables |
| Cell | Block | Tables |
| Slide | Block | Presentation |
| Text | Inline | Text |
| Emphasis | Inline | Text |
| Subscript | Inline | Text |
| Superscript | Inline | Text |
| Link | Inline | Text |
| InlineCode | Inline | Text |
| Citation | Inline | References |
| CrossReference | Inline | References |
| FootnoteReference | Inline | References |
| Footnote | Reference | References |
| BibliographyEntry | Reference | References |
| ImageSource | Resource | Figures |

---

## Design Decisions

**Headings use numeric levels, not named variants.** Chapter, section, subsection are conventions that vary by language and domain. A level integer is simpler and more deterministic.

**Emphasis uses levels, not bold/italic.** Bold and italic are presentation variants. Nested emphasis carries semantic meaning (increasing importance or stress) and is independent of visual rendering.

**Tables are restricted to a header and body.** Complex tables with multiple header rows, footer rows, or row headers are not supported in the first version. The model can be extended if needed.

**Lists can contain any block content.** A list item is not limited to inline text. It may contain paragraphs, code blocks, nested lists, etc. This matches the behavior of all reference formats.

**The document model is the same for documents and presentations.** A slide is a block element with a title and speaker notes. A presentation is a document whose body consists of slides. This avoids duplicating the model.

**Metadata is minimal.** Only title, author, date, and language are included. Additional metadata (keywords, subject, revision) can be added later without changing the core model.

**Presentation features are in scope.** Slides, slide titles, and speaker notes are core because PPTX is a Tier 1 target. Slide layouts, transitions, and animations are backend-only.

**Footnotes are document-level, not page-level.** The model does not distinguish between footnotes and endnotes — the backend chooses placement. The semantic content is identical.
