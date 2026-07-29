# DSL Syntax

The VernierOffice language uses a lightweight, LaTeX-inspired syntax.

## Design Principles

- Commands are prefixed with `\` and use `{}` for arguments.
- Environments use `\begin{name}` and `\end{name}`.
- Paragraphs are separated by one or more blank lines.
- Whitespace at the start and end of lines is insignificant.
- `%` starts a line comment; the rest of the line is ignored.

## Lexical Structure

### Commands

```
\name{arg1}{arg2}...
```

A command name consists of ASCII letters. Arguments are enclosed in braces. The number of arguments is fixed per command.

### Environments

```
\begin{name}
content
\end{name}
```

An environment groups content under a named construct. The body may contain block elements.

### Text

Any character sequence that is not a command or environment delimiter is literal text.

Consecutive whitespace characters within a paragraph collapse to a single space.

### Blank Lines

One or more blank lines separate paragraphs. Blank lines within environments are significant.

### Comments

```
% This is a comment
```

A `%` outside of a code block or literal context starts a comment that extends to the end of the line.

### Special Characters

The characters `\`, `{`, `}`, `%`, `&` have special meaning. To include them literally in text, prefix with `\`:

```
\\  →  \
\{  →  {
\}  →  }
\%  →  %
\&  →  &
```

---

## Document Structure

### Metadata

```
\title{My Document Title}
\author{Author Name}
\date{2026-07-29}
\lang{en}
```

All metadata commands are optional. They must appear before any block content. Each may appear at most once.

### Headings

```
\heading{1}{Section Title}
\heading{2}{Subsection Title}
\heading{3}{Subsubsection Title}
```

The first argument is the heading level (1-based integer). The second is the heading text, which may contain inline elements.

### Paragraph

```
This is a paragraph of text. It may contain
inline commands like \em{emphasis} and span
multiple lines in the source.

This is a second paragraph, separated by a blank line.
```

### Table of Contents

```
\toc
```

With depth limit:

```
\toc{3}
```

---

## Inline Elements

### Emphasis

```
\em{text}
\em{level}{text}
```

Without a level, it defaults to 1 (primary emphasis). Level 2 is secondary.

```
Normal \em{emphasized} text.
Normal \em{2}{strongly emphasized} text.
\em{3}{very strongly emphasized} text.
```

### Subscript and Superscript

```
\sub{text}
\super{text}
```

```
H\sub{2}O
E = mc\super{2}
```

### Link

```
\link{url}{display text}
```

```
See \link{https://example.com}{the documentation} for details.
```

### Inline Code

```
\code{let x = 1;}
```

### Citation

```
\cite{key}
```

```
As shown by \cite{knuth1984}...
```

### Cross-Reference

```
\ref{label}
```

```
See \heading{1}{Introduction} \ref{sec:intro} for background.
```

### Footnote Reference

```
\footnoteref{id}
```

```
This claim requires evidence\footnoteref{fn:evidence}.
```

---

## Block Elements

### Lists

Unordered list:

```
\begin{itemize}
\item First item
\item Second item
\end{itemize}
```

Ordered list:

```
\begin{enumerate}
\item Step one
\item Step two
\end{enumerate}
```

List items may contain multiple blocks:

```
\begin{itemize}
\item First item, first paragraph.

     Second paragraph of the first item.

\item Second item
\end{itemize}
```

Nested lists:

```
\begin{itemize}
\item Top level
\begin{itemize}
\item Nested item
\end{itemize}
\item Another top-level item
\end{itemize}
```

### Block Quote

```
\begin{quote}
A quoted passage that may span
multiple paragraphs.

Second paragraph of the quote.
\end{quote}
```

### Code Block

```
\begin{code}{rust}
fn main() {
    println!("Hello, world!");
}
\end{code}
```

The language argument is optional:

```
\begin{code}
Plain preformatted text.
\end{code}
```

Whitespace within a code block is preserved literally.

---

## Figures

```
\begin{figure}
\image{path/to/image.png}{Alt text for accessibility}
\caption{Figure caption with \em{emphasis}}
\end{figure}
```

The `\image` command specifies the image file path and alternative text. The `\caption` is a label applied to the figure by the backend.

A figure without a caption:

```
\begin{figure}
\image{diagram.png}{Architecture overview}
\end{figure}
```

---

## Tables

### Basic Table

```
\begin{table}
\caption{Quarterly results}
\begin{tabular}
\header{Quarter & Revenue & Profit}
\header{Q1 & \$100 & \$20}
Q1 & \$100 & \$20 \\
Q2 & \$120 & \$25 \\
Q3 & \$115 & \$22 \\
Q4 & \$130 & \$28 \\
\end{tabular}
\end{table}
```

A `tabular` consists of zero or more `\header` rows followed by body rows. Body rows use `\\` to terminate. The last body row may omit `\\`.

The `\caption` is optional.

### Merged Cells

```
\cell{colspan}{rowspan}{content}
```

Example:

```
\begin{tabular}
\header{\cell{2}{1}{Spanning Header} & Normal}
Data 1 & Data 2 \\
Data 3 & \cell{1}{2}{Merged down} \\
Data 4 & \\
\end{tabular}
```

`colspan` and `rowspan` are integers ≥ 1. Default is 1.

### Cells with Block Content

A cell may contain block elements (paragraphs, lists) using `\cellblock`:

```
\begin{tabular}
\header{Feature & Description}
Simple cell & Plain text \\
Complex cell & \cellblock{
    \begin{itemize}
    \item Point one
    \item Point two
    \end{itemize}
} \\
\end{tabular}
```

---

## Footnotes

```
\footnote{id}{Footnote content.}

This is a statement\footnoteref{id}.
```

Footnote definitions are collected at the end of the document body. They may appear anywhere in the source; the backend places them appropriately.

Footnote content is inline text.

---

## Bibliography

```
\begin{bibliography}
\entry{knuth1984}{
    author = {Donald Knuth},
    title = {The TeXbook},
    year = {1984},
}
\entry{lamport1994}{
    author = {Leslie Lamport},
    title = {LaTeX: A Document Preparation System},
    year = {1994},
}
\end{bibliography}
```

Each `\entry{key}{fields}` defines one bibliography entry. The `fields` block uses `name = {value}` pairs. Field names are not prescribed by the language; the backend formats entries according to the configured citation style.

---

## Slides (Presentation Mode)

```
\begin{slide}
\slidetitle{Introduction}
\slidenotes{Mention the project goals.}

\begin{itemize}
\item First key point
\item Second key point
\end{itemize}

\begin{figure}
\image{diagram.png}{Overview}
\end{figure}
\end{slide}
```

A slide has a title, optional speaker notes, and block content. Multiple slides form a presentation:

```
\title{My Presentation}
\author{Jane Doe}

\begin{slide}
\slidetitle{Slide One}
Content.
\end{slide}

\begin{slide}
\slidetitle{Slide Two}
More content.
\end{slide}
```

A document is inferred to be a presentation when its body contains `\begin{slide}`.

---

## Complete Example

```
\title{Getting Started with VernierOffice}
\author{Jane Doe}
\date{2026-07-29}
\lang{en}

\toc

\heading{1}{Introduction}

VernierOffice is an \em{experimental} document language. It separates
semantics from presentation, letting backends handle the visual rendering.

Key features:

\begin{itemize}
\item Semantic markup with \code{\\em\{text\}}
\item \link{https://example.com}{Hyperlinks} and cross-references
\item Tables, figures, and footnotes
\end{itemize}

\heading{2}{Installation}

Install using Cargo:

\begin{code}{sh}
cargo install vernier-office
\end{code}

\heading{2}{Usage}

\heading{3}{Basic Document}

Create a file with the \code{.vo} extension and compile it:

\begin{code}{sh}
vernier-office compile document.vo --output document.docx
\end{code}

\heading{3}{Presentations}

Use slides for presentations:

\begin{code}{sh}
vernier-office compile slides.vo --output slides.pptx
\end{code}

\heading{2}{Reference}

\begin{table}
\caption{Inline commands summary}
\begin{tabular}
\header{Command & Purpose}
\\em\{text\} & Semantic emphasis \\
\\link\{url\}\{text\} & Hyperlink \\
\\code\{text\} & Inline code \\
\\cite\{key\} & Citation \\
\\ref\{label\} & Cross-reference \\
\end{tabular}
\end{table}

\heading{1}{Conclusion}

This is a minimal but complete document. See \cite{knuth1984} for
background on typesetting systems.

\begin{bibliography}
\entry{knuth1984}{
    author = {Donald Knuth},
    title = {The TeXbook},
    year = {1984},
}
\end{bibliography}
```

---

## Grammar

```
document        := metadata* block*
metadata        := '\title' '{' inline* '}'
                 | '\author' '{' inline* '}'
                 | '\date' '{' inline* '}'
                 | '\lang' '{' text '}'

block           := heading
                 | paragraph
                 | toc
                 | bullet_list
                 | ordered_list
                 | block_quote
                 | code_block
                 | figure
                 | table
                 | slide
                 | footnote_def
                 | bibliography

heading         := '\heading' '{' integer '}' '{' inline* '}'
toc             := '\toc' ('{' integer '}')?
paragraph       := inline+ blank_line

inline          := text
                 | '\em' '{' inline* '}'
                 | '\em' '{' integer '}' '{' inline* '}'
                 | '\sub' '{' inline* '}'
                 | '\super' '{' inline* '}'
                 | '\link' '{' inline* '}' '{' inline* '}'
                 | '\code' '{' text '}'
                 | '\cite' '{' text '}'
                 | '\ref' '{' text '}'
                 | '\footnoteref' '{' text '}'
                 | escape

escape          := '\\' | '\{' | '\}' | '\%' | '\&'

bullet_list     := '\begin{itemize}' list_item* '\end{itemize}'
ordered_list    := '\begin{enumerate}' list_item* '\end{enumerate}'
list_item       := '\item' inline_or_block*

block_quote     := '\begin{quote}' block* '\end{quote}'
code_block      := '\begin{code}' ('{' text '}')? text '\end{code}'

figure          := '\begin{figure}' image caption? '\end{figure}'
image           := '\image' '{' text '}' '{' text '}'
caption         := '\caption' '{' inline* '}'

table           := '\begin{table}' caption? tabular '\end{table}'
tabular         := '\begin{tabular}' header_row* body_row* '\end{tabular}'
header_row      := '\header' '{' cell+ '}'
body_row        := cell+ '\\'
cell            := '\cell' '{' integer '}' '{' integer '}' '{' inline* '}'
                 | '\cellblock' '{' block* '}'
                 | inline*

slide           := '\begin{slide}' slide_title slide_notes? block* '\end{slide}'
slide_title     := '\slidetitle' '{' inline* '}'
slide_notes     := '\slidenotes' '{' inline* '}'

footnote_def    := '\footnote' '{' text '}' '{' inline* '}'
bibliography    := '\begin{bibliography}' bib_entry* '\end{bibliography}'
bib_entry       := '\entry' '{' text '}' '{' field* '}'
field           := text '=' '{' inline* '}'

text            := [^\\{}%&\n]+             % raw text, no special chars
integer         := [1-9][0-9]*
blank_line      := \n\s*\n
```

---

## Parser Rules

1. **No lookahead.** The grammar is LL(1) by construction. The `\` prefix disambiguates commands from text.

2. **Fixed arity.** Every command accepts a fixed number of arguments. There are no optional arguments (except `\toc{depth}`, where the argument is either present with a brace group or absent).

3. **Environments are explicit.** Every `\begin{name}` must be matched by `\end{name}`. Nesting of different environments is allowed. Self-nesting (e.g., list inside list) is allowed.

4. **Paragraph boundaries.** A blank line (two consecutive newlines with only whitespace between) terminates the current paragraph and starts a new one. Blank lines within environments are preserved.

5. **Code blocks are opaque.** Content between `\begin{code}` and `\end{code}` is treated as literal text. No commands or special characters are interpreted.

6. **Table rows.** Within `\begin{tabular}`, `\header{...}` rows are header rows. Lines ending with `\\` are body rows. `&` separates cells on the same row.

7. **Commands take inline content.** Block-level commands (environments) contain blocks. Inline commands contain inline content and text. The only exception is `\cellblock{...}` which may contain blocks within a table cell.

---

## Backend Mapping

Backends map each semantic element to the target format:

| DSL Construct | DOCX | PPTX | HTML |
|---|---|---|---|
| `\heading{n}{t}` | Heading style n | Slide title / heading | `<hn>` |
| Paragraph | `w:p` | Text body | `<p>` |
| `\em{t}` | Italic run | Italic run | `<em>` |
| `\em{2}{t}` | Bold run | Bold run | `<strong>` |
| `\link{u}{t}` | Hyperlink | Hyperlink | `<a>` |
| `\code{t}` | Monospace run | Monospace run | `<code>` |
| `\begin{itemize}` | Bullet list | Bullet list | `<ul>` |
| `\begin{enumerate}` | Numbered list | Numbered list | `<ol>` |
| `\begin{quote}` | Indented paragraph | Indented text | `<blockquote>` |
| `\begin{code}{l}` | Monospace paragraph | Monospace text | `<pre><code>` |
| `\begin{figure}` | Drawing + caption | Picture + caption | `<figure>` |
| `\begin{table}` | Table + caption | Table + caption | `<table>` + `<caption>` |
| `\begin{slide}` | N/A | Slide | `<section>` |
| `\toc` | TOC field | N/A | Nested `<ol>` |
| `\cite{k}` | Citation field | Citation text | `<cite>` |
| `\ref{l}` | Cross-reference field | Cross-reference text | `<a>` |
| `\footnote{i}{t}` | Footnote | Footnote text | Footnote anchor |
