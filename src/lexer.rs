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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let tokens = lex("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].0, Token::Eof);
    }

    #[test]
    fn test_only_comments() {
        let tokens = lex("% This is a comment\n% Another comment\n");
        // Comments yield newlines; last token is Eof
        let non_newline: Vec<_> = tokens
            .iter()
            .filter(|(t, _)| !matches!(t, Token::Newline))
            .collect();
        assert_eq!(non_newline.len(), 1);
        assert_eq!(non_newline[0].0, Token::Eof);
    }

    #[test]
    fn test_plain_text() {
        let tokens = lex("hello world");
        assert_eq!(tokens.len(), 2); // Text + Eof
        assert_eq!(tokens[0].0, Token::Text("hello world".into()));
    }

    #[test]
    fn test_command() {
        let tokens = lex("\\em{text}");
        assert_eq!(tokens[0].0, Token::Command("em".into()));
        assert_eq!(tokens[1].0, Token::OpenBrace);
        assert_eq!(tokens[2].0, Token::Text("text".into()));
        assert_eq!(tokens[3].0, Token::CloseBrace);
    }

    #[test]
    fn test_escaped_braces() {
        let tokens = lex("\\{not a group\\}");
        assert_eq!(tokens[0].0, Token::Text("{".into()));
        assert_eq!(tokens[1].0, Token::Text("not a group".into()));
        assert_eq!(tokens[2].0, Token::Text("}".into()));
    }

    #[test]
    fn test_escaped_backslash() {
        let tokens = lex("a\\\\b");
        // Should be: Text("a"), Backslash, Text("b")
        assert_eq!(tokens[0].0, Token::Text("a".into()));
        assert_eq!(tokens[1].0, Token::Backslash);
        assert_eq!(tokens[2].0, Token::Text("b".into()));
    }

    #[test]
    fn test_escaped_percent() {
        let tokens = lex("not \\% a comment");
        assert_eq!(tokens[0].0, Token::Text("not ".into()));
        assert_eq!(tokens[1].0, Token::Text("%".into()));
        assert_eq!(tokens[2].0, Token::Text(" a comment".into()));
    }

    #[test]
    fn test_escaped_ampersand() {
        let tokens = lex("a \\& b");
        assert_eq!(tokens[0].0, Token::Text("a ".into()));
        assert_eq!(tokens[1].0, Token::Text("&".into()));
        assert_eq!(tokens[2].0, Token::Text(" b".into()));
    }

    #[test]
    fn test_backslash_at_eof() {
        let tokens = lex("text\\");
        // A lone backslash at EOF should be a Backslash token
        assert_eq!(tokens[0].0, Token::Text("text".into()));
        assert_eq!(tokens[1].0, Token::Backslash);
    }

    #[test]
    fn test_unicode_text() {
        let tokens = lex("café – en dash — em dash … ellipsis");
        assert!(tokens[0].0 == Token::Text("café – en dash — em dash … ellipsis".into()));
    }

    #[test]
    fn test_multiple_newlines() {
        let tokens = lex("line 1\n\n\nline 2");
        let newline_count = tokens.iter().filter(|(t, _)| matches!(t, Token::Newline)).count();
        assert_eq!(newline_count, 3);
    }

    #[test]
    fn test_ampersand_token() {
        let tokens = lex("A & B");
        assert_eq!(tokens[0].0, Token::Text("A ".into()));
        assert_eq!(tokens[1].0, Token::Ampersand);
        assert_eq!(tokens[2].0, Token::Text(" B".into()));
    }

    #[test]
    fn test_equals_token() {
        let tokens = lex("x = y");
        assert_eq!(tokens[0].0, Token::Text("x ".into()));
        assert_eq!(tokens[1].0, Token::Equals);
        assert_eq!(tokens[2].0, Token::Text(" y".into()));
    }

    #[test]
    fn test_command_with_digits_is_text() {
        // Backslash followed by digits should be treated as Backslash + text
        let tokens = lex("\\3foo");
        assert_eq!(tokens[0].0, Token::Backslash);
        assert_eq!(tokens[1].0, Token::Text("3foo".into()));
    }

    #[test]
    fn test_span_tracks_line_numbers() {
        let tokens = lex("line1\nline2\n\\em{x}");
        let spans: Vec<_> = tokens.iter().map(|(_, s)| (s.line, s.col)).collect();
        // "line1" starts at line 1, col 1
        assert_eq!(spans[0], (1, 1));
        // The newline token is at line 1, col 6
        assert!(spans.iter().any(|(l, _)| *l == 2));
        // Command on line 3
        assert!(spans.iter().any(|(l, _)| *l == 3));
    }

    #[test]
    fn test_carriage_return_skipped() {
        let tokens = lex("hello\r\nworld");
        let texts: Vec<_> = tokens
            .iter()
            .filter_map(|(t, _)| {
                if let Token::Text(s) = t {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(texts, vec!["hello", "world"]);
    }

    #[test]
    fn test_comment_at_end_of_line() {
        let tokens = lex("visible%hidden\nmore");
        assert_eq!(tokens[0].0, Token::Text("visible".into()));
        assert_eq!(tokens[1].0, Token::Newline);
        assert_eq!(tokens[2].0, Token::Text("more".into()));
    }
}
