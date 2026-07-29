use crate::ir::{
    BibliographyEntry, BlockElement, BlockQuote, Cell, CodeBlock, Document, Emphasis, Figure,
    Footnote, Heading, ImageSource, InlineElement, Link, List, ListItem, Metadata, Paragraph,
    Row, Slide, Span, Table, TableOfContents,
};
use crate::lexer::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedEof { expected: String, span: Span },
    UnexpectedToken { expected: String, found: String, span: Span },
    UnmatchedBegin { name: String, span: Span },
    UnmatchedEnd { name: String, span: Span },
    MissingArgument { command: String, span: Span },
    InvalidHeadingLevel { found: String, span: Span },
    EmptyEnvironment { name: String, span: Span },
}

pub type ParseResult<T> = Result<T, ParseError>;

struct Parser {
    tokens: Vec<(Token, Span)>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<(Token, Span)>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> (&Token, &Span) {
        self.tokens
            .get(self.pos)
            .map(|(t, s)| (t, s))
            .unwrap_or((&EOFSENTINEL, &STUBSENTINEL))
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn expect_command(&mut self, name: &str) -> ParseResult<Span> {
        match self.peek() {
            (Token::Command(n), s) if n == name => {
                let span = s.clone();
                self.advance();
                Ok(span)
            }
            (Token::Command(n), s) => Err(ParseError::UnexpectedToken {
                expected: format!("\\{}", name),
                found: format!("\\{}", n),
                span: s.clone(),
            }),
            (t, s) => Err(ParseError::UnexpectedToken {
                expected: format!("\\{}", name),
                found: format!("{:?}", t),
                span: s.clone(),
            }),
        }
    }

    fn expect_open_brace(&mut self) -> ParseResult<Span> {
        match self.peek() {
            (Token::OpenBrace, s) => {
                let span = s.clone();
                self.advance();
                Ok(span)
            }
            (t, s) => Err(ParseError::UnexpectedToken {
                expected: "{".into(),
                found: format!("{:?}", t),
                span: s.clone(),
            }),
        }
    }

    fn expect_close_brace(&mut self) -> ParseResult<Span> {
        match self.peek() {
            (Token::CloseBrace, s) => {
                let span = s.clone();
                self.advance();
                Ok(span)
            }
            (t, s) => Err(ParseError::UnexpectedToken {
                expected: "}".into(),
                found: format!("{:?}", t),
                span: s.clone(),
            }),
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek().0, Token::Newline) {
            self.advance();
        }
    }

    // --- Inline parsing ---

    fn parse_text(&mut self) -> String {
        if let (Token::Text(t), _) = self.peek() {
            let s = t.clone();
            self.advance();
            s
        } else {
            String::new()
        }
    }

    fn parse_braced_content(&mut self) -> ParseResult<String> {
        self.expect_open_brace()?;
        let mut content = String::new();
        loop {
            match self.peek().0 {
                Token::CloseBrace => break,
                Token::Eof => {
                    return Err(ParseError::UnexpectedEof {
                        expected: "}".into(),
                        span: self.peek().1.clone(),
                    })
                }
                Token::Text(t) => {
                    content.push_str(t);
                    self.advance();
                }
                Token::Command(_) | Token::OpenBrace => {
                    // In braced content used for raw strings (code, keys, etc.),
                    // treat everything as text
                    let (t, _) = self.peek();
                    content.push_str(&format!("{:?}", t));
                    self.advance();
                }
                Token::Ampersand => {
                    content.push('&');
                    self.advance();
                }
                Token::Backslash => {
                    content.push('\\');
                    self.advance();
                }
                Token::Equals => {
                    content.push('=');
                    self.advance();
                }
                Token::Newline => {
                    content.push('\n');
                    self.advance();
                }
            }
        }
        self.advance(); // close brace
        Ok(content)
    }

    fn parse_braced_inlines(&mut self) -> ParseResult<Vec<InlineElement>> {
        self.expect_open_brace()?;
        let content = self.parse_inlines_until(Token::CloseBrace)?;
        self.advance(); // close brace
        Ok(content)
    }

    fn parse_inlines_until(&mut self, end: Token) -> ParseResult<Vec<InlineElement>> {
        let mut elements = Vec::new();
        loop {
            match self.peek().0.clone() {
                t if t == end => break,
                Token::Eof => break,
                Token::CloseBrace => {
                    // If we hit an unexpected close brace while looking for something else,
                    // treat it as part of the text or break depending on context
                    break;
                }
                Token::Command(name) => {
                    let span = self.peek().1.clone();
                    self.advance();
                    elements.push(self.parse_inline_command(&name, &span)?);
                }
                Token::Text(_) => {
                    let t = self.parse_text();
                    if !t.is_empty() {
                        elements.push(InlineElement::Text(t));
                    }
                }
                Token::Ampersand => {
                    elements.push(InlineElement::Text("&".into()));
                    self.advance();
                }
                Token::Backslash => {
                    elements.push(InlineElement::Text("\\".into()));
                    self.advance();
                }
                Token::Equals => {
                    elements.push(InlineElement::Text("=".into()));
                    self.advance();
                }
                Token::OpenBrace => {
                    elements.push(InlineElement::Text("{".into()));
                    self.advance();
                }
                Token::Newline => {
                    elements.push(InlineElement::Text(" ".into()));
                    self.advance();
                }
            }
        }
        Ok(elements)
    }

    fn parse_inline_command(&mut self, name: &str, span: &Span) -> ParseResult<InlineElement> {
        match name {
            "em" => {
                // \em{content} or \em{level}{content}
                self.expect_open_brace().map_err(|_| ParseError::MissingArgument {
                    command: "\\em".into(),
                    span: span.clone(),
                })?;
                let first = self.parse_inlines_until(Token::CloseBrace)?;
                self.advance(); // close brace

                // Check if there's another brace group (level form)
                let is_level_form = first.len() == 1
                    && matches!(&first[0], InlineElement::Text(t) if t.parse::<u8>().is_ok());

                if is_level_form {
                    let level: u8 = match &first[0] {
                        InlineElement::Text(t) => t.parse().unwrap_or(1),
                        _ => 1,
                    };
                    let content = self.parse_braced_inlines()?;
                    Ok(InlineElement::Emphasis(Emphasis { level, content }))
                } else {
                    Ok(InlineElement::Emphasis(Emphasis {
                        level: 1,
                        content: first,
                    }))
                }
            }
            "sub" => {
                let content = self.parse_braced_inlines().map_err(|_| ParseError::MissingArgument {
                    command: "\\sub".into(),
                    span: span.clone(),
                })?;
                Ok(InlineElement::Subscript(content))
            }
            "super" => {
                let content =
                    self.parse_braced_inlines().map_err(|_| ParseError::MissingArgument {
                        command: "\\super".into(),
                        span: span.clone(),
                    })?;
                Ok(InlineElement::Superscript(content))
            }
            "link" => {
                let target = self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                    command: "\\link".into(),
                    span: span.clone(),
                })?;
                let content =
                    self.parse_braced_inlines().map_err(|_| ParseError::MissingArgument {
                        command: "\\link".into(),
                        span: span.clone(),
                    })?;
                Ok(InlineElement::Link(Link { target, content }))
            }
            "code" => {
                let content =
                    self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                        command: "\\code".into(),
                        span: span.clone(),
                    })?;
                Ok(InlineElement::InlineCode(content))
            }
            "cite" => {
                let key = self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                    command: "\\cite".into(),
                    span: span.clone(),
                })?;
                Ok(InlineElement::Citation(key))
            }
            "ref" => {
                let label =
                    self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                        command: "\\ref".into(),
                        span: span.clone(),
                    })?;
                Ok(InlineElement::CrossReference(label))
            }
            "footnoteref" => {
                let id = self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                    command: "\\footnoteref".into(),
                    span: span.clone(),
                })?;
                Ok(InlineElement::FootnoteReference(id))
            }
            _ => {
                // Unknown command: treat as literal text
                Ok(InlineElement::Text(format!("\\{}", name)))
            }
        }
    }

    fn parse_inlines_in_braces(&mut self) -> ParseResult<Vec<InlineElement>> {
        self.expect_open_brace()?;
        let content = self.parse_inlines_until(Token::CloseBrace)?;
        self.advance();
        Ok(content)
    }

    // --- Block parsing ---

    fn parse_block_command(&mut self, name: &str, span: &Span) -> ParseResult<BlockElement> {
        match name {
            "heading" => {
                let level_str = self.parse_braced_content().map_err(|_| {
                    ParseError::MissingArgument {
                        command: "\\heading".into(),
                        span: span.clone(),
                    }
                })?;

                let level: u8 = level_str.trim().parse().map_err(|_| {
                    ParseError::InvalidHeadingLevel {
                        found: level_str,
                        span: span.clone(),
                    }
                })?;

                let content =
                    self.parse_braced_inlines().map_err(|_| ParseError::MissingArgument {
                        command: "\\heading".into(),
                        span: span.clone(),
                    })?;

                Ok(BlockElement::Heading(Heading { level, content }))
            }
            "toc" => {
                let depth = if matches!(self.peek().0, Token::OpenBrace) {
                    let s = self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                        command: "\\toc".into(),
                        span: span.clone(),
                    })?;
                    s.trim().parse().ok()
                } else {
                    None
                };
                Ok(BlockElement::TableOfContents(TableOfContents { depth }))
            }
            _ => {
                // Unknown block command: treat as paragraph text
                Ok(BlockElement::Paragraph(Paragraph {
                    content: vec![InlineElement::Text(format!("\\{}", name))],
                }))
            }
        }
    }

    fn parse_environment(&mut self, name: &str, span: &Span) -> ParseResult<BlockElement> {
        match name {
            "itemize" => {
                let items = self.parse_list_items()?;
                self.skip_newlines();
                self.expect_command("end")?;
                let end_name = self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                    command: "\\end".into(),
                    span: span.clone(),
                })?;
                if end_name != "itemize" {
                    return Err(ParseError::UnmatchedBegin {
                        name: "itemize".into(),
                        span: span.clone(),
                    });
                }
                Ok(BlockElement::List(List {
                    ordered: false,
                    items,
                }))
            }
            "enumerate" => {
                let items = self.parse_list_items()?;
                self.skip_newlines();
                self.expect_command("end")?;
                let end_name = self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                    command: "\\end".into(),
                    span: span.clone(),
                })?;
                if end_name != "enumerate" {
                    return Err(ParseError::UnmatchedBegin {
                        name: "enumerate".into(),
                        span: span.clone(),
                    });
                }
                Ok(BlockElement::List(List {
                    ordered: true,
                    items,
                }))
            }
            "quote" => {
                let content = self.parse_blocks_until(&["end"])?;
                self.skip_newlines();
                self.expect_command("end")?;
                let end_name = self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                    command: "\\end".into(),
                    span: span.clone(),
                })?;
                if end_name != "quote" {
                    return Err(ParseError::UnmatchedBegin {
                        name: "quote".into(),
                        span: span.clone(),
                    });
                }
                Ok(BlockElement::BlockQuote(BlockQuote { content }))
            }
            "code" => {
                let language = if matches!(self.peek().0, Token::OpenBrace) {
                    Some(self.parse_braced_content().map_err(|_| {
                        ParseError::MissingArgument {
                            command: "\\begin{code}".into(),
                            span: span.clone(),
                        }
                    })?)
                } else {
                    None
                };

                // Read code content until \end{code}
                let mut content = String::new();
                self.skip_newlines();
                loop {
                    match self.peek().0 {
                        Token::Command(c) if c == "end" => {
                            // Look ahead to check if it's \end{code}
                            let pos = self.pos;
                            self.advance();
                            if matches!(self.peek().0, Token::OpenBrace) {
                                if let Ok(ename) = self.parse_braced_content() {
                                    if ename == "code" {
                                        // The close brace was already consumed by parse_braced_content
                                        break;
                                    }
                                }
                            }
                            // Not \end{code}, backtrack
                            self.pos = pos;
                            content.push_str(&self.consume_char_to_string());
                        }
                        Token::Eof => break,
                        _ => {
                            content.push_str(&self.consume_char_to_string());
                        }
                    }
                }

                Ok(BlockElement::CodeBlock(CodeBlock {
                    language,
                    content,
                }))
            }
            "figure" => {
                let (image, caption) = self.parse_figure_contents(span)?;
                self.skip_newlines();
                self.expect_command("end")?;
                let end_name = self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                    command: "\\end".into(),
                    span: span.clone(),
                })?;
                if end_name != "figure" {
                    return Err(ParseError::UnmatchedBegin {
                        name: "figure".into(),
                        span: span.clone(),
                    });
                }
                Ok(BlockElement::Figure(Figure { image, caption }))
            }
            "table" => {
                self.skip_newlines();
                let caption = if self.peek().0.command_name() == Some("caption") {
                    self.advance();
                    Some(self.parse_inlines_in_braces().map_err(|_| {
                        ParseError::MissingArgument {
                            command: "\\caption".into(),
                            span: span.clone(),
                        }
                    })?)
                } else {
                    None
                };

                // Expect \begin{tabular}
                self.skip_newlines();
                self.expect_command("begin")?;
                let tabular_name = self.parse_braced_content().map_err(|_| {
                    ParseError::MissingArgument {
                        command: "\\begin".into(),
                        span: span.clone(),
                    }
                })?;
                if tabular_name != "tabular" {
                    return Err(ParseError::UnexpectedToken {
                        expected: "tabular".into(),
                        found: tabular_name,
                        span: span.clone(),
                    });
                }

                let (header, body) = self.parse_tabular_contents()?;

                self.skip_newlines();
                self.expect_command("end")?;
                let end_tab = self.parse_braced_content().map_err(|_| {
                    ParseError::MissingArgument {
                        command: "\\end".into(),
                        span: span.clone(),
                    }
                })?;
                if end_tab != "tabular" {
                    return Err(ParseError::UnmatchedBegin {
                        name: "tabular".into(),
                        span: span.clone(),
                    });
                }

                self.skip_newlines();
                self.expect_command("end")?;
                let end_name = self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                    command: "\\end".into(),
                    span: span.clone(),
                })?;
                if end_name != "table" {
                    return Err(ParseError::UnmatchedBegin {
                        name: "table".into(),
                        span: span.clone(),
                    });
                }

                Ok(BlockElement::Table(Table {
                    caption,
                    header,
                    body,
                }))
            }
            "slide" => {
                let (title, notes, content) = self.parse_slide_contents(span)?;
                self.skip_newlines();
                self.expect_command("end")?;
                let end_name = self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                    command: "\\end".into(),
                    span: span.clone(),
                })?;
                if end_name != "slide" {
                    return Err(ParseError::UnmatchedBegin {
                        name: "slide".into(),
                        span: span.clone(),
                    });
                }
                Ok(BlockElement::Slide(Slide {
                    title,
                    notes,
                    content,
                }))
            }
            "bibliography" => {
                let _entries = self.parse_bibliography_entries()?;
                self.expect_command("end")?;
                let end_name = self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                    command: "\\end".into(),
                    span: span.clone(),
                })?;
                if end_name != "bibliography" {
                    return Err(ParseError::UnmatchedBegin {
                        name: "bibliography".into(),
                        span: span.clone(),
                    });
                }
                // Bibliography is collected at document level, not as a block element.
                // Return a placeholder paragraph that will be filtered out.
                Ok(BlockElement::Paragraph(Paragraph {
                    content: vec![InlineElement::Text("__BIBLIOGRAPHY__".into())],
                }))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "known environment".into(),
                found: name.into(),
                span: span.clone(),
            }),
        }
    }

    fn parse_figure_contents(
        &mut self,
        span: &Span,
    ) -> ParseResult<(ImageSource, Option<Vec<InlineElement>>)> {
        self.skip_newlines();
        self.expect_command("image").map_err(|_| ParseError::MissingArgument {
            command: "\\image".into(),
            span: span.clone(),
        })?;

        let path = self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
            command: "\\image".into(),
            span: span.clone(),
        })?;
        let alt = self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
            command: "\\image".into(),
            span: span.clone(),
        })?;

        let image = ImageSource {
            path,
            alt: if alt.is_empty() { None } else { Some(alt) },
        };

        self.skip_newlines();
        let caption = if self.peek().0.command_name() == Some("caption") {
            self.advance();
            Some(self.parse_inlines_in_braces().map_err(|_| ParseError::MissingArgument {
                command: "\\caption".into(),
                span: span.clone(),
            })?)
        } else {
            None
        };

        Ok((image, caption))
    }

    fn parse_list_items(&mut self) -> ParseResult<Vec<ListItem>> {
        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek().0 {
                Token::Command(c) if c == "item" => {
                    self.advance();
                    let content = self.parse_block_content_until(&["item", "end"])?;
                    items.push(ListItem { content });
                }
                _ => break,
            }
        }
        Ok(items)
    }

    fn parse_tabular_contents(&mut self) -> ParseResult<(Vec<Row>, Vec<Row>)> {
        let mut header = Vec::new();
        let mut body = Vec::new();
        self.skip_newlines();

        loop {
            self.skip_newlines();
            match self.peek().0 {
                Token::Command(c) if c == "header" => {
                    self.advance();
                    self.expect_open_brace()?; // consume { before row content
                    let row = self.parse_row()?;
                    header.push(row);
                }
                Token::Command(c) if c == "end" => break,
                Token::Eof => break,
                _ => {
                    let row = self.parse_row()?;
                    body.push(row);
                }
            }
        }

        Ok((header, body))
    }

    fn parse_row(&mut self) -> ParseResult<Row> {
        // A row is: cell* \\ (or until close brace for \header{...})
        // For \header, the content is already inside braces
        // For body rows, cells are inline elements ending with \\
        let mut cells = Vec::new();
        let mut current_content: Vec<BlockElement> = Vec::new();
        let mut current_inlines: Vec<InlineElement> = Vec::new();

        loop {
            match self.peek().0.clone() {
                Token::Ampersand => {
                    self.advance();
                    // Flush current inlines to a paragraph and add as cell
                    if !current_inlines.is_empty() {
                        current_content.push(BlockElement::Paragraph(Paragraph {
                            content: std::mem::take(&mut current_inlines),
                        }));
                    }
                    let cell_content = std::mem::take(&mut current_content);
                    cells.push(Cell {
                        colspan: 1,
                        rowspan: 1,
                        content: if cell_content.is_empty() {
                            vec![BlockElement::Paragraph(Paragraph {
                                content: Vec::new(),
                            })]
                        } else {
                            cell_content
                        },
                    });
                }
                Token::Command(ref name) if name == "cell" => {
                    self.advance();
                    let colspan_str =
                        self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                            command: "\\cell".into(),
                            span: Span { line: 1, col: 1 },
                        })?;
                    let rowspan_str =
                        self.parse_braced_content().map_err(|_| ParseError::MissingArgument {
                            command: "\\cell".into(),
                            span: Span { line: 1, col: 1 },
                        })?;
                    let colspan: u8 = colspan_str.trim().parse().unwrap_or(1);
                    let rowspan: u8 = rowspan_str.trim().parse().unwrap_or(1);

                    // Content is the next brace group
                    let content_inlines = self.parse_inlines_in_braces().unwrap_or_default();
                    cells.push(Cell {
                        colspan,
                        rowspan,
                        content: vec![BlockElement::Paragraph(Paragraph {
                            content: content_inlines,
                        })],
                    });
                }
                Token::Command(ref name) if name == "cellblock" => {
                    self.advance();
                    let blocks = self.parse_braced_blocks()?;
                    cells.push(Cell {
                        colspan: 1,
                        rowspan: 1,
                        content: blocks,
                    });
                }
                Token::Backslash => {
                    // Row terminator is double backslash: \\
                    self.advance();
                    if matches!(self.peek().0, Token::Backslash) {
                        self.advance(); // consume second backslash
                        // Flush remaining content
                        if !current_inlines.is_empty() {
                            current_content.push(BlockElement::Paragraph(Paragraph {
                                content: std::mem::take(&mut current_inlines),
                            }));
                        }
                        if !current_content.is_empty() {
                            cells.push(Cell {
                                colspan: 1,
                                rowspan: 1,
                                content: std::mem::take(&mut current_content),
                            });
                        }
                        break;
                    }
                    // Single backslash: treat as text
                    current_inlines.push(InlineElement::Text("\\".into()));
                }
                Token::CloseBrace | Token::Command(_) | Token::Eof => {
                    // End of row — consume the closing brace if present
                    if matches!(self.peek().0, Token::CloseBrace) {
                        self.advance();
                    }
                    if !current_inlines.is_empty() {
                        current_content.push(BlockElement::Paragraph(Paragraph {
                            content: std::mem::take(&mut current_inlines),
                        }));
                    }
                    if !current_content.is_empty() {
                        cells.push(Cell {
                            colspan: 1,
                            rowspan: 1,
                            content: std::mem::take(&mut current_content),
                        });
                    }
                    break;
                }
                Token::Newline => {
                    self.advance();
                    if !current_inlines.is_empty() {
                        current_content.push(BlockElement::Paragraph(Paragraph {
                            content: std::mem::take(&mut current_inlines),
                        }));
                    }
                }
                Token::Text(_) | Token::OpenBrace | Token::Equals => {
                    // Parse inline content in table cell, one token at a time
                    match self.peek().0.clone() {
                        Token::Text(_) => {
                            let t = self.parse_text();
                            if !t.is_empty() {
                                current_inlines.push(InlineElement::Text(t));
                            }
                        }
                        Token::OpenBrace => {
                            current_inlines.push(InlineElement::Text("{".into()));
                            self.advance();
                        }
                        Token::Equals => {
                            current_inlines.push(InlineElement::Text("=".into()));
                            self.advance();
                        }
                        _ => {}
                    }
                }
                // All token types are covered above
            }
        }

        // Flush any remaining inlines
        if !current_inlines.is_empty() {
            current_content.push(BlockElement::Paragraph(Paragraph {
                content: std::mem::take(&mut current_inlines),
            }));
        }
        if !current_content.is_empty() && cells.is_empty() {
            cells.push(Cell {
                colspan: 1,
                rowspan: 1,
                content: std::mem::take(&mut current_content),
            });
        }

        Ok(Row { cells })
    }

    fn parse_slide_contents(
        &mut self,
        span: &Span,
    ) -> ParseResult<(Vec<InlineElement>, Option<Vec<InlineElement>>, Vec<BlockElement>)> {
        self.skip_newlines();

        self.expect_command("slidetitle").map_err(|_| ParseError::MissingArgument {
            command: "\\slidetitle".into(),
            span: span.clone(),
        })?;
        let title = self.parse_inlines_in_braces().map_err(|_| ParseError::MissingArgument {
            command: "\\slidetitle".into(),
            span: span.clone(),
        })?;

        self.skip_newlines();

        let notes = if self.peek().0.command_name() == Some("slidenotes") {
            self.advance();
            Some(self.parse_inlines_in_braces().map_err(|_| ParseError::MissingArgument {
                command: "\\slidenotes".into(),
                span: span.clone(),
            })?)
        } else {
            None
        };

        let content = self.parse_blocks_until(&["end"])?;

        Ok((title, notes, content))
    }

    fn parse_bibliography_entries(&mut self) -> ParseResult<Vec<BibliographyEntry>> {
        let mut entries = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek().0 {
                Token::Command(c) if c == "entry" => {
                    let span = self.peek().1.clone();
                    self.advance();
                    let key = self.parse_braced_content().map_err(|_| {
                        ParseError::MissingArgument {
                            command: "\\entry".into(),
                            span: span.clone(),
                        }
                    })?;

                    // Parse fields { name = {value} ... }
                    let fields = self.parse_bib_fields()?;
                    entries.push(BibliographyEntry { key, fields });
                }
                _ => break,
            }
        }
        Ok(entries)
    }

    fn parse_bib_fields(&mut self) -> ParseResult<Vec<(String, String)>> {
        self.expect_open_brace()?;
        let mut fields = Vec::new();
        self.skip_newlines();

        loop {
            match self.peek().0 {
                Token::CloseBrace => break,
                Token::Command(_) => break,
                Token::Eof => break,
                Token::Text(field_name) => {
                    let name = field_name.clone().trim().to_string();
                    self.advance();

                    // Skip whitespace, expect =
                    self.skip_newlines();
                    if matches!(self.peek().0, Token::Equals) {
                        self.advance();
                    }

                    self.skip_newlines();
                    let value = self.parse_braced_content().unwrap_or_default();
                    fields.push((name, value));
                    self.skip_newlines();
                }
                _ => {
                    self.advance();
                }
            }
        }

        self.expect_close_brace()?;
        Ok(fields)
    }

    fn parse_blocks_until(&mut self, stop_commands: &[&str]) -> ParseResult<Vec<BlockElement>> {
        let mut blocks = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek().0 {
                Token::Command(c) if stop_commands.contains(&c.as_str()) => break,
                Token::Eof => break,
                _ => {
                    if let Some(block) = self.parse_block()? {
                        blocks.push(block);
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(blocks)
    }

    fn parse_block_content_until(
        &mut self,
        stop_commands: &[&str],
    ) -> ParseResult<Vec<BlockElement>> {
        let mut blocks = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek().0 {
                Token::Command(c) if stop_commands.contains(&c.as_str()) => break,
                Token::Eof => break,
                _ => {
                    if let Some(block) = self.parse_block()? {
                        blocks.push(block);
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(blocks)
    }

    fn parse_block(&mut self) -> ParseResult<Option<BlockElement>> {
        self.skip_newlines();
        match self.peek().0.clone() {
            Token::Eof => Ok(None),
            Token::Command(name) => {
                let span = self.peek().1.clone();
                self.advance();

                if name == "begin" {
                    let env_name = self.parse_braced_content().map_err(|_| {
                        ParseError::MissingArgument {
                            command: "\\begin".into(),
                            span: span.clone(),
                        }
                    })?;
                    Ok(Some(self.parse_environment(&env_name, &span)?))
                } else if name == "end" {
                    // Unexpected end — backtrack and let outer parser handle it
                    self.pos -= 1;
                    Ok(None)
                } else if name == "footnote" {
                    let id_span = span.clone();
                    let id = self.parse_braced_content().map_err(|_| {
                        ParseError::MissingArgument {
                            command: "\\footnote".into(),
                            span: id_span.clone(),
                        }
                    })?;
                    let _content =
                        self.parse_braced_inlines().map_err(|_| ParseError::MissingArgument {
                            command: "\\footnote".into(),
                            span: id_span,
                        })?;
                    // Footnote is collected at doc level; return a placeholder
                    Ok(Some(BlockElement::Paragraph(Paragraph {
                        content: vec![InlineElement::Text(format!("__FOOTNOTE_DEF__{}", id))],
                    })))
                } else {
                    Ok(Some(self.parse_block_command(&name, &span)?))
                }
            }
            Token::Text(_) | Token::OpenBrace | Token::Ampersand | Token::Backslash
            | Token::Equals => {
                // Paragraph content
                let inlines = self.parse_inlines_until(Token::Newline)?;
                if inlines.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(BlockElement::Paragraph(Paragraph {
                        content: inlines,
                    })))
                }
            }
            Token::Newline => {
                self.advance();
                Ok(None)
            }
            _ => {
                self.advance();
                Ok(None)
            }
        }
    }

    fn parse_braced_blocks(&mut self) -> ParseResult<Vec<BlockElement>> {
        self.expect_open_brace()?;
        let blocks = self.parse_blocks_until(&[""])?;
        self.expect_close_brace()?;
        Ok(blocks)
    }

    fn consume_char_to_string(&mut self) -> String {
        match self.peek().0 {
            Token::Text(t) => {
                let s = t.clone();
                self.advance();
                s
            }
            Token::OpenBrace => {
                self.advance();
                "{".into()
            }
            Token::CloseBrace => {
                self.advance();
                "}".into()
            }
            Token::Ampersand => {
                self.advance();
                "&".into()
            }
            Token::Backslash => {
                self.advance();
                "\\".into()
            }
            Token::Equals => {
                self.advance();
                "=".into()
            }
            Token::Newline => {
                self.advance();
                "\n".into()
            }
            Token::Command(c) => {
                let s = format!("\\{}", c);
                self.advance();
                s
            }
            Token::Eof => String::new(),
        }
    }
}

// Sentinel values for EOF peeks — these live at static addresses so
// they never alias with Parser-owned data and peek() returning &STUBSENTINEL
// is consistent with the borrow checker.
static EOFSENTINEL: Token = Token::Eof;
static STUBSENTINEL: Span = Span { line: 0, col: 0 };

pub fn parse(tokens: Vec<(Token, Span)>) -> ParseResult<Document> {
    let mut parser = Parser::new(tokens);

    // Parse metadata
    let mut metadata = Metadata::default();
    loop {
        parser.skip_newlines();
        match parser.peek().0 {
            Token::Command(name) if is_metadata_command(name) => {
                let cmd = name.clone();
                parser.advance();
                let value = parser.parse_braced_content()?;
                match cmd.as_str() {
                    "title" => metadata.title = Some(value),
                    "author" => metadata.author = Some(value),
                    "date" => metadata.date = Some(value),
                    "lang" => metadata.language = Some(value),
                    _ => {}
                }
            }
            _ => break,
        }
    }

    // Parse body blocks, collecting footnotes and bibliography
    let mut body: Vec<BlockElement> = Vec::new();
    let mut footnotes: Vec<Footnote> = Vec::new();
    let mut bibliography: Vec<BibliographyEntry> = Vec::new();

    loop {
        parser.skip_newlines();
        if matches!(parser.peek().0, Token::Eof) {
            break;
        }

        match parser.parse_block() {
            Ok(Some(block)) => {
                // Filter out placeholder blocks and collect footnotes/bibliography
                match &block {
                    BlockElement::Paragraph(p) if !p.content.is_empty() => {
                        if let InlineElement::Text(t) = &p.content[0] {
                            if t.starts_with("__FOOTNOTE_DEF__") {
                                // This should have been parsed directly as a footnote by the block handler
                                // We handle it here for robustness
                                body.push(block);
                                continue;
                            }
                        }
                    }
                    _ => {}
                }
                body.push(block);
            }
            Ok(None) => break,
            Err(e) => return Err(e),
        }
    }

    // Parse footnotes from body placeholders
    body.retain(|block| {
        if let BlockElement::Paragraph(p) = block {
            if p.content.len() == 1 {
                if let InlineElement::Text(t) = &p.content[0] {
                    if t.starts_with("__FOOTNOTE_DEF__") {
                        // Extract id and look for footnote
                        return false;
                    }
                }
            }
        }
        true
    });

    // Find bibliography at the end of body
    if let Some(last) = body.last() {
        if let BlockElement::Paragraph(p) = last {
            if p.content.len() == 1 {
                if let InlineElement::Text(t) = &p.content[0] {
                    if t == "__BIBLIOGRAPHY__" {
                        body.pop();
                    }
                }
            }
        }
    }

    // Parse footnotes and bibliography separately by rescanning
    // For now, we handle footnote definitions inline in parse_block
    // We need to collect them properly. Let's re-parse with a fresh parser
    // This is inefficient but correct for now.
    let fresh_tokens = parser.tokens.clone();
    let mut p2 = Parser::new(fresh_tokens);

    // Skip metadata
    loop {
        p2.skip_newlines();
        match p2.peek().0 {
            Token::Command(name) if is_metadata_command(name) => {
                p2.advance();
                let _ = p2.parse_braced_content();
            }
            _ => break,
        }
    }

    // Scan for footnotes
    loop {
        p2.skip_newlines();
        match p2.peek().0 {
            Token::Eof => break,
            Token::Command(ref name) if name == "footnote" => {
                let start_span = p2.peek().1.clone();
                p2.advance();
                if let Ok(id) = p2.parse_braced_content() {
                    if let Ok(content) = p2.parse_braced_inlines() {
                        footnotes.push(Footnote { id, content });
                    }
                }
                // If parsing failed, try to recover
                let _ = start_span;
            }
            Token::Command(ref name) if name == "begin" => {
                p2.advance();
                if matches!(p2.peek().0, Token::OpenBrace) {
                    if let Ok(env_name) = p2.parse_braced_content() {
                        if env_name == "bibliography" {
                            // Parse bibliography
                            if let Ok(entries) = p2.parse_bibliography_entries() {
                                bibliography = entries;
                            }
                            // Skip to end of environment
                            p2.expect_command("end").ok();
                            p2.expect_open_brace().ok();
                            let _ = p2.parse_braced_content();
                            p2.expect_close_brace().ok();
                        } else {
                            // Skip unknown environment
                            p2.skip_environment();
                        }
                    }
                }
            }
            _ => {
                p2.advance();
            }
        }
    }

    Ok(Document {
        metadata,
        body,
        footnotes,
        bibliography,
    })
}

impl Parser {
    fn skip_environment(&mut self) {
        let mut depth = 1;
        while depth > 0 {
            match self.peek().0 {
                Token::Eof => break,
                Token::Command(ref name) if name == "begin" => {
                    self.advance();
                    if matches!(self.peek().0, Token::OpenBrace) {
                        let _ = self.parse_braced_content();
                    }
                    depth += 1;
                }
                Token::Command(ref name) if name == "end" => {
                    self.advance();
                    if matches!(self.peek().0, Token::OpenBrace) {
                        let _ = self.parse_braced_content();
                    }
                    depth -= 1;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }
}

fn is_metadata_command(name: &str) -> bool {
    matches!(name, "title" | "author" | "date" | "lang")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    #[test]
    fn test_parse_simple_paragraph() {
        let source = "Hello world.";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        assert_eq!(doc.body.len(), 1);
        match &doc.body[0] {
            BlockElement::Paragraph(p) => {
                assert_eq!(p.content.len(), 1);
                assert_eq!(p.content[0], InlineElement::Text("Hello world.".into()));
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn test_parse_emphasis() {
        let source = "Hello \\em{world}.";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::Paragraph(p) => {
                assert_eq!(p.content.len(), 3);
                assert_eq!(p.content[0], InlineElement::Text("Hello ".into()));
                match &p.content[1] {
                    InlineElement::Emphasis(e) => {
                        assert_eq!(e.level, 1);
                        assert_eq!(e.content[0], InlineElement::Text("world".into()));
                    }
                    _ => panic!("expected emphasis"),
                }
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn test_parse_heading() {
        let source = "\\heading{1}{Introduction}";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::Heading(h) => {
                assert_eq!(h.level, 1);
                assert_eq!(h.content[0], InlineElement::Text("Introduction".into()));
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_metadata() {
        let source = "\\title{My Doc}\n\\author{Jane}\n\nHello.";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        assert_eq!(doc.metadata.title, Some("My Doc".into()));
        assert_eq!(doc.metadata.author, Some("Jane".into()));
        assert_eq!(doc.body.len(), 1);
    }

    #[test]
    fn test_parse_list() {
        let source = "\\begin{itemize}\n\\item First\n\\item Second\n\\end{itemize}";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::List(l) => {
                assert!(!l.ordered);
                assert_eq!(l.items.len(), 2);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_parse_figure() {
        let source =
            "\\begin{figure}\n\\image{img.png}{alt}\n\\caption{A figure}\n\\end{figure}";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::Figure(f) => {
                assert_eq!(f.image.path, "img.png");
                assert!(f.caption.is_some());
            }
            _ => panic!("expected figure"),
        }
    }

    #[test]
    fn test_parse_table() {
        let source = concat!(
            "\\begin{table}\n",
            "\\caption{Data}\n",
            "\\begin{tabular}\n",
            "\\header{A & B}\n",
            "1 & 2 \\\\\n",
            "\\end{tabular}\n",
            "\\end{table}\n"
        );
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::Table(t) => {
                assert_eq!(t.header.len(), 1);
                assert_eq!(t.body.len(), 1);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn test_parse_subscript_superscript() {
        let source = "H\\sub{2}O and x\\super{2}";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::Paragraph(p) => {
                assert_eq!(p.content.len(), 4); // H, sub, "O and x", super
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn test_parse_citation_and_ref() {
        let source = "See \\cite{knuth} in \\ref{sec:intro}.";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::Paragraph(p) => {
                assert!(p
                    .content
                    .iter()
                    .any(|e| matches!(e, InlineElement::Citation(c) if c == "knuth")));
                assert!(p
                    .content
                    .iter()
                    .any(|e| matches!(e, InlineElement::CrossReference(r) if r == "sec:intro")));
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn test_parse_slide() {
        let source = concat!(
            "\\title{My Pres}\n\n",
            "\\begin{slide}\n",
            "\\slidetitle{Slide 1}\n",
            "\\slidenotes{Talk about X}\n",
            "Content here.\n",
            "\\end{slide}\n"
        );
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        assert_eq!(doc.metadata.title, Some("My Pres".into()));
        match &doc.body[0] {
            BlockElement::Slide(s) => {
                assert_eq!(s.title[0], InlineElement::Text("Slide 1".into()));
                assert!(s.notes.is_some());
                assert_eq!(s.content.len(), 1);
            }
            _ => panic!("expected slide"),
        }
    }

    #[test]
    fn test_empty_input() {
        let tokens = lex("");
        let doc = parse(tokens).unwrap();
        assert_eq!(doc.body.len(), 0);
        assert_eq!(doc.footnotes.len(), 0);
        assert_eq!(doc.bibliography.len(), 0);
    }

    #[test]
    fn test_only_whitespace() {
        let tokens = lex("\n\n\n");
        let doc = parse(tokens).unwrap();
        assert_eq!(doc.body.len(), 0);
    }

    #[test]
    fn test_emphasis_level_2() {
        // \em{2}{bold} must be in paragraph context since \em is inline-only
        let source = "Hello \\em{2}{bold} text.";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::Paragraph(p) => {
                assert!(p.content.iter().any(|e| match e {
                    InlineElement::Emphasis(em) => em.level == 2,
                    _ => false,
                }));
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn test_nested_emphasis() {
        // \em in paragraph context with nested \em (auto level 2)
        let source = "Hello \\em{outer \\em{inner} text}!";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::Paragraph(p) => {
                assert!(p.content.iter().any(|e| match e {
                    InlineElement::Emphasis(em) => {
                        em.content.iter().any(|c| matches!(c, InlineElement::Emphasis(_)))
                    }
                    _ => false,
                }));
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn test_link() {
        let source = "Visit \\link{https://example.com}{the site}.";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::Paragraph(p) => {
                assert!(p.content.iter().any(|e| match e {
                    InlineElement::Link(l) => l.target == "https://example.com",
                    _ => false,
                }));
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn test_ordered_list() {
        let source = "\\begin{enumerate}\n\\item First\n\\item Second\n\\end{enumerate}";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::List(l) => {
                assert!(l.ordered);
                assert_eq!(l.items.len(), 2);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_code_block() {
        let source = "\\begin{code}\nlet x = 1;\n\\end{code}";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::CodeBlock(cb) => {
                assert_eq!(cb.language, None);
                assert_eq!(cb.content, "let x = 1;\n");
            }
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn test_blockquote() {
        let source = "\\begin{quote}\nA quoted paragraph.\n\\end{quote}";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::BlockQuote(bq) => {
                assert_eq!(bq.content.len(), 1);
            }
            _ => panic!("expected blockquote"),
        }
    }

    #[test]
    fn test_footnote_parsing() {
        let source = concat!(
            "Text with a note\\footnoteref{note1}.\n",
            "\\footnote{note1}{This is the footnote.}"
        );
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        assert_eq!(doc.footnotes.len(), 1);
        assert_eq!(doc.footnotes[0].id, "note1");
        // Body should have the paragraph with the footnote reference
        assert!(doc.body.iter().any(|b| match b {
            BlockElement::Paragraph(p) => p.content.iter().any(|e| match e {
                InlineElement::FootnoteReference(id) => id == "note1",
                _ => false,
            }),
            _ => false,
        }));
    }

    #[test]
    fn test_bibliography_parsing() {
        let source = concat!(
            "\\begin{bibliography}\n",
            "\\entry{key1}{author=Smith, title=The Book, year=2020}\n",
            "\\end{bibliography}\n"
        );
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        assert_eq!(doc.bibliography.len(), 1);
        assert_eq!(doc.bibliography[0].key, "key1");
    }

    #[test]
    fn test_toc() {
        // TOC is \toc, not \tableofcontents
        let source = "\\toc{2}";
        let tokens = lex(source);
        let doc = parse(tokens).unwrap();
        match &doc.body[0] {
            BlockElement::TableOfContents(toc) => {
                assert_eq!(toc.depth, Some(2));
            }
            _ => panic!("expected TOC"),
        }
    }
}
