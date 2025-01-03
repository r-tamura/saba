use alloc::{
    string::{String, ToString},
    vec::Vec,
};

const RESERVED_WORDS: [&str; 3] = ["var", "function", "return"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// https://262.ecma-international.org/#sec-punctuators
    Punctuator(char),
    /// https://262.ecma-international.org/#sec-literals-numeric-literals
    Number(u64),
    /// https://262.ecma-international.org/#sec-identifier-names
    Identifier(String),
    /// https://262.ecma-international.org/#sec-keywords-and-reserved-words
    Keyword(String),
    /// https://262.ecma-international.org/#sec-literals-string-literals
    StringLiteral(String),
}

pub struct JsLexer {
    pos: usize,
    input: Vec<char>,
}

impl JsLexer {
    pub fn new(js: String) -> Self {
        Self {
            pos: 0,
            input: js.chars().collect(),
        }
    }

    fn exhausted(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek(&self) -> char {
        self.input[self.pos]
    }

    fn consume(&mut self) -> char {
        let c = self.input[self.pos];
        self.pos += 1;
        c
    }

    fn skip_whitespaces(&mut self) {
        while self.peek() == ' ' || self.peek() == '\n' {
            self.consume();

            if self.exhausted() {
                return;
            }
        }
    }

    fn skip_n(&mut self, n: usize) {
        self.pos += n;
    }

    fn next_equals_to(&self, keyword: &str) -> bool {
        if self.pos + keyword.len() > self.input.len() {
            return false;
        }

        let candidate: String = self.input[self.pos..self.pos + keyword.len()]
            .iter()
            .collect();
        candidate == keyword
    }

    fn peek_reserved_word(&self) -> Option<String> {
        for reserved in RESERVED_WORDS {
            if self.next_equals_to(reserved) {
                return Some(reserved.to_string());
            }
        }
        None
    }

    fn consume_identifier(&mut self) -> String {
        let mut result = String::new();

        loop {
            if self.exhausted() {
                return result;
            }

            if self.peek().is_ascii_alphanumeric() || self.peek() == '$' {
                result.push(self.consume());
            } else {
                return result;
            }
        }
    }

    fn consume_string(&mut self) -> String {
        let mut result = String::new();
        assert!(
            self.peek() == '"' || self.peek() == '\'',
            "current char should be string start quote",
        );
        self.consume();

        loop {
            if self.exhausted() {
                return result;
            }

            if self.peek() == '"' || self.peek() == '\'' {
                self.consume();
                return result;
            }

            result.push(self.consume());
        }
    }

    fn consume_number(&mut self) -> u64 {
        let mut num = 0;

        loop {
            if self.exhausted() {
                return num;
            }

            match self.peek() {
                c @ '0'..='9' => {
                    num = num * 10 + (c.to_digit(10).unwrap() as u64);
                    self.consume();
                }
                _ => return num,
            }
        }
    }
}

impl Iterator for JsLexer {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted() {
            return None;
        }

        self.skip_whitespaces();

        if let Some(keyword) = self.peek_reserved_word() {
            self.skip_n(keyword.len());
            return Some(Token::Keyword(keyword));
        }

        let c = self.peek();

        let token = match c {
            '+' | '-' | ';' | '=' | '(' | ')' | '{' | '}' | ',' | '.' => {
                let t = Token::Punctuator(c);
                self.consume();
                t
            }
            '0'..='9' => Token::Number(self.consume_number()),
            'a'..='z' | 'A'..='Z' | '_' | '$' => Token::Identifier(self.consume_identifier()),
            '"' | '\'' => Token::StringLiteral(self.consume_string()),
            _ => unimplemented!("char '{:?}' is not supported yet", c),
        };

        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let input = "".to_string();
        let mut lexer = JsLexer::new(input).peekable();
        assert!(lexer.peek().is_none());
    }

    #[test]
    fn test_single_quoted_string() {
        let input = r#"'string'"#.to_string();
        let mut lexer = JsLexer::new(input).peekable();
        assert_eq!(
            lexer.next(),
            Some(Token::StringLiteral("string".to_string()))
        );
    }

    #[test]
    fn test_num() {
        let input = "42".to_string();
        let mut lexer = JsLexer::new(input).peekable();
        let expected = [Token::Number(42)].to_vec();
        let mut i = 0;
        while lexer.peek().is_some() {
            assert_eq!(Some(expected[i].clone()), lexer.next());
            i += 1;
        }
        assert!(lexer.peek().is_none());
    }

    #[test]
    fn test_add_nums() {
        let input = "1 + 2".to_string();
        let mut lexer = JsLexer::new(input).peekable();
        let expected = [Token::Number(1), Token::Punctuator('+'), Token::Number(2)].to_vec();
        let mut i = 0;
        while lexer.peek().is_some() {
            assert_eq!(Some(expected[i].clone()), lexer.next());
            i += 1;
        }
        assert!(lexer.peek().is_none());
    }

    #[test]
    fn test_assign_variable() {
        let input = "var foo=\"bar\";".to_string();
        let mut lexer = JsLexer::new(input).peekable();
        let expected = [
            Token::Keyword("var".to_string()),
            Token::Identifier("foo".to_string()),
            Token::Punctuator('='),
            Token::StringLiteral("bar".to_string()),
            Token::Punctuator(';'),
        ]
        .to_vec();
        let mut i = 0;
        while lexer.peek().is_some() {
            assert_eq!(Some(expected[i].clone()), lexer.next());
            i += 1;
        }
        assert!(lexer.peek().is_none());
    }

    #[test]
    fn test_add_variable_and_num() {
        let input = "var foo=42; var result=foo+1;".to_string();
        let mut lexer = JsLexer::new(input).peekable();
        let expected = [
            Token::Keyword("var".to_string()),
            Token::Identifier("foo".to_string()),
            Token::Punctuator('='),
            Token::Number(42),
            Token::Punctuator(';'),
            Token::Keyword("var".to_string()),
            Token::Identifier("result".to_string()),
            Token::Punctuator('='),
            Token::Identifier("foo".to_string()),
            Token::Punctuator('+'),
            Token::Number(1),
            Token::Punctuator(';'),
        ]
        .to_vec();
        let mut i = 0;
        while lexer.peek().is_some() {
            assert_eq!(Some(expected[i].clone()), lexer.next());
            i += 1;
        }
        assert!(lexer.peek().is_none());
    }

    #[test]
    fn test_add_local_variable_and_num() {
        let input = "function foo() { var a=42; return a; } var result = foo() + 1;".to_string();
        let mut lexer = JsLexer::new(input).peekable();
        let expected = [
            Token::Keyword("function".to_string()),
            Token::Identifier("foo".to_string()),
            Token::Punctuator('('),
            Token::Punctuator(')'),
            Token::Punctuator('{'),
            Token::Keyword("var".to_string()),
            Token::Identifier("a".to_string()),
            Token::Punctuator('='),
            Token::Number(42),
            Token::Punctuator(';'),
            Token::Keyword("return".to_string()),
            Token::Identifier("a".to_string()),
            Token::Punctuator(';'),
            Token::Punctuator('}'),
            Token::Keyword("var".to_string()),
            Token::Identifier("result".to_string()),
            Token::Punctuator('='),
            Token::Identifier("foo".to_string()),
            Token::Punctuator('('),
            Token::Punctuator(')'),
            Token::Punctuator('+'),
            Token::Number(1),
            Token::Punctuator(';'),
        ]
        .to_vec();
        let mut i = 0;
        while lexer.peek().is_some() {
            assert_eq!(Some(expected[i].clone()), lexer.next());
            i += 1;
        }
        assert!(lexer.peek().is_none());
    }
}
