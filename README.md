# VernierOffice

**VernierOffice** is a semantic document compiler that transforms a lightweight, LaTeX-inspired markup language into OOXML formats (DOCX and PPTX).

Instead of manipulating Office documents through complex APIs, VernierOffice provides a minimal semantic language that separates content from presentation.

## Status

**v0.1.0-alpha** — Core compiler pipeline implemented with DOCX and PPTX backends.

**v0.2.0-alpha** — Minor version during bug repairing.

**v0.3.0-beta** — First verified usable version.

## Features

- Semantic markup with emphasis, subscripts, superscripts, links, inline code
- Paragraphs, headings (levels 1–3), table of contents
- Ordered and unordered lists, block quotes, code blocks
- Tables with header rows, merged cells, and captions
- Figures with images and captions
- Footnotes and bibliography
- Slide-based presentations with speaker notes
- DOCX output (Office Open XML)
- PPTX output (PresentationML)

## Installation

```sh
cargo install vernier-office
```

## Usage

```sh
# Compile to DOCX (default)
vernier-office compile document.vo --output document.docx

# Compile to PPTX
vernier-office compile slides.vo --output slides.pptx

# Format inferred from output extension
vernier-office compile document.vo --output document.pptx
```

## Repository Structure

```
VernierOffice/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Public API
│   ├── ir.rs            # Intermediate representation types
│   ├── lexer.rs         # Lexer (source → tokens)
│   ├── parser.rs        # Parser (tokens → IR)
│   └── backend/
│       ├── mod.rs       # Backend trait
│       ├── docx.rs      # DOCX backend (OOXML)
│       └── pptx.rs      # PPTX backend (PresentationML)
├── tests/
│   └── integration_test.rs
├── Example/
│   ├── getting-started.vo
│   └── presentation.vo
├── Docs/
│   ├── semantic-model.md      # Abstract semantic model
│   ├── dsl-syntax.md          # DSL syntax specification
│   ├── feature-inventory.md   # Feature inventory and decisions
│   └── ir-architecture.md     # IR design and compiler architecture
├── Cargo.toml
└── README.md
```

## Design Goals

- AI-first document authoring
- Semantic rather than visual markup
- Minimal syntax
- Deterministic compilation
- Multiple output formats from a single source

## Documentation

- `Docs/dsl-syntax.md` — Language syntax reference
- `Docs/semantic-model.md` — Abstract semantic model
- `Docs/ir-architecture.md` — Compiler pipeline and IR design
- `Docs/feature-inventory.md` — Feature classification and decisions

## License

MIT
