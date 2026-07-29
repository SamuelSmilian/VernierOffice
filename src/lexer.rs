use crate::ir::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Command(String),
    OpenBrace,
    CloseBrace,
    Text(String),
    Newline,
    Ampersand,
    Backslash,
    Equals,
    Eof,
}

impl Token {
    pub fn command_name(&self) -> Option<&str> {
        match self {
            Token::Command(name) => Some(name),
            _ => None,
        }
    }
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    done: bool,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            done: false,
        }
    }

    fn span(&self) -> Span {
        Span {
            line: self.line,
            col: self.col,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        if self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            Some(c)
        } else {
            None
        }
    }

    fn consume_while<F: Fn(char) -> bool>(&mut self, f: F) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if f(c) {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    fn skip_comment(&mut self) {
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn read_command(&mut self) -> String {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphabetic() {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        name
    }
}

impl Iterator for Lexer {
    type Item = (Token, Span);

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // Skip carriage returns only (spaces and tabs are significant in text)
        while let Some('\r') = self.peek() {
            self.advance();
        }

        let span = self.span();

        match self.peek() {
            None => {
                self.done = true;
                Some((Token::Eof, span))
            }
            Some('%') => {
                self.advance();
                self.skip_comment();
                // Recurse to get the next token after the comment
                self.next()
            }
            Some('\\') => {
                self.advance();
                match self.peek() {
                    Some(c) if c.is_ascii_alphabetic() => {
                        let name = self.read_command();
                        if name.is_empty() {
                            Some((Token::Text("\\".into()), span))
                        } else {
                            Some((Token::Command(name), span))
                        }
                    }
                    Some('\\') => {
                        self.advance();
                        Some((Token::Backslash, span))
                    }
                    Some('{') => {
                        self.advance();
                        Some((Token::Text("{".into()), span))
                    }
                    Some('}') => {
                        self.advance();
                        Some((Token::Text("}".into()), span))
                    }
                    Some('%') => {
                        self.advance();
                        Some((Token::Text("%".into()), span))
                    }
                    Some('&') => {
                        self.advance();
                        Some((Token::Text("&".into()), span))
                    }
                    _ => Some((Token::Backslash, span)),
                }
            }
            Some('{') => {
                self.advance();
                Some((Token::OpenBrace, span))
            }
            Some('}') => {
                self.advance();
                Some((Token::CloseBrace, span))
            }
            Some('&') => {
                self.advance();
                Some((Token::Ampersand, span))
            }
            Some('=') => {
                self.advance();
                Some((Token::Equals, span))
            }
            Some('\n') => {
                self.advance();
                Some((Token::Newline, span))
            }
            Some(_) => {
                let text = self.consume_while(|c| {
                    !matches!(c, '\\' | '{' | '}' | '%' | '&' | '=' | '\n' | '\r')
                });
                if text.is_empty() {
                    self.advance();
                    self.next()
                } else {
                    Some((Token::Text(text), span))
                }
            }
        }
    }
}

pub fn lex(source: &str) -> Vec<(Token, Span)> {
    Lexer::new(source).collect()
}
