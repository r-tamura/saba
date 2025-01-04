use core::iter::Peekable;

use alloc::string::String;
use alloc::vec;
use alloc::{rc::Rc, vec::Vec};

use super::token::{JsLexer, Token};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    ExpressionStatement(Option<Rc<Node>>),
    AdditiveExpression {
        operator: char,
        left: Option<Rc<Node>>,
        right: Option<Rc<Node>>,
    },
    AssignmentExpression {
        operator: char,
        left: Option<Rc<Node>>,
        right: Option<Rc<Node>>,
    },
    MemberExpression {
        object: Option<Rc<Node>>,
        property: Option<Rc<Node>>,
    },
    NumericLiteral(u64),
    VariableDeclaration {
        declarations: Vec<Option<Rc<Node>>>,
    },
    VariableDeclarator {
        id: Option<Rc<Node>>,
        init: Option<Rc<Node>>,
    },
    Identifier(String),
    StringLiteral(String),
    BlockStatement {
        body: Vec<Option<Rc<Node>>>,
    },
    ReturnStatement {
        argument: Option<Rc<Node>>,
    },
    FunctionDeclaration {
        id: Option<Rc<Node>>,
        params: Vec<Option<Rc<Node>>>,
        body: Option<Rc<Node>>,
    },
    CallExpressioin {
        callee: Option<Rc<Node>>,
        arguments: Vec<Option<Rc<Node>>>,
    },
}

impl Node {
    pub fn new_expression_statement(expression: Option<Rc<Self>>) -> Option<Rc<Self>> {
        Some(Rc::new(Node::ExpressionStatement(expression)))
    }

    pub fn new_additive_expression(
        operator: char,
        left: Option<Rc<Node>>,
        right: Option<Rc<Node>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::AdditiveExpression {
            operator,
            left,
            right,
        }))
    }

    pub fn new_member_expression(
        object: Option<Rc<Self>>,
        property: Option<Rc<Self>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::MemberExpression { object, property }))
    }

    pub fn new_numeric_literal(value: u64) -> Option<Rc<Self>> {
        Some(Rc::new(Node::NumericLiteral(value)))
    }

    pub fn new_variable_declarator(
        id: Option<Rc<Self>>,
        init: Option<Rc<Self>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::VariableDeclarator { id, init }))
    }

    pub fn new_variable_declaration(declarations: Vec<Option<Rc<Self>>>) -> Option<Rc<Self>> {
        Some(Rc::new(Node::VariableDeclaration { declarations }))
    }

    pub fn new_identifier(name: String) -> Option<Rc<Self>> {
        Some(Rc::new(Node::Identifier(name)))
    }

    pub fn new_string_literal(value: String) -> Option<Rc<Self>> {
        Some(Rc::new(Node::StringLiteral(value)))
    }

    pub fn new_block_statement(body: Vec<Option<Rc<Self>>>) -> Option<Rc<Self>> {
        Some(Rc::new(Node::BlockStatement { body }))
    }

    pub fn new_return_statement(argument: Option<Rc<Self>>) -> Option<Rc<Self>> {
        Some(Rc::new(Node::ReturnStatement { argument }))
    }

    pub fn new_function_declaration(
        id: Option<Rc<Self>>,
        params: Vec<Option<Rc<Self>>>,
        body: Option<Rc<Self>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::FunctionDeclaration { id, params, body }))
    }

    pub fn new_call_expression(
        callee: Option<Rc<Self>>,
        arguments: Vec<Option<Rc<Self>>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::CallExpressioin { callee, arguments }))
    }
}

pub struct JsParser {
    t: Peekable<JsLexer>,
}

impl JsParser {
    pub fn new(t: JsLexer) -> Self {
        Self { t: t.peekable() }
    }

    fn primary_expression(&mut self) -> Option<Rc<Node>> {
        match self.t.next()? {
            Token::Number(n) => Node::new_numeric_literal(n),
            Token::StringLiteral(s) => Node::new_string_literal(s),
            _ => None,
        }
    }

    fn member_expression(&mut self) -> Option<Rc<Node>> {
        self.primary_expression()
    }

    fn arguments(&mut self) -> Vec<Option<Rc<Node>>> {
        todo!();
    }

    fn left_hand_side_expression(&mut self) -> Option<Rc<Node>> {
        self.member_expression()
    }

    fn additive_expression(&mut self) -> Option<Rc<Node>> {
        let left = self.left_hand_side_expression();
        let token = match self.t.peek() {
            Some(token) => token.clone(),
            None => return left,
        };
        match token {
            Token::Punctuator(op) => match op {
                '+' | '-' => {
                    assert!(self.t.next().is_some());
                    Node::new_additive_expression(op, left, self.assignment_expression())
                }
                _ => left,
            },
            _ => left,
        }
    }

    fn assignment_expression(&mut self) -> Option<Rc<Node>> {
        self.additive_expression()
    }

    fn initialiser(&mut self) -> Option<Rc<Node>> {
        todo!();
    }

    fn identifier(&mut self) -> Option<Rc<Node>> {
        todo!();
    }

    fn variable_declaration(&mut self) -> Option<Rc<Node>> {
        todo!();
    }

    fn statement(&mut self) -> Option<Rc<Node>> {
        let node = Node::new_expression_statement(self.assignment_expression());
        match self.t.peek() {
            Some(Token::Punctuator(c)) if c == &';' => {
                assert!(self.t.next().is_some());
            }
            // セミコロンは省略できる
            _ => {}
        }
        node
    }

    fn function_body(&mut self) -> Option<Rc<Node>> {
        todo!();
    }

    fn parameter_list(&mut self) -> Vec<Option<Rc<Node>>> {
        todo!();
    }

    fn function_declaration(&mut self) -> Option<Rc<Node>> {
        Node::new_function_declaration(
            self.initialiser(),
            self.parameter_list(),
            self.function_body(),
        )
    }

    fn source_element(&mut self) -> Option<Rc<Node>> {
        let token = self.t.peek()?;

        match token {
            Token::Keyword(keyword) => {
                match keyword.as_str() {
                    "function" => {
                        // 'function'キーワードを消費
                        self.t.next().expect("should have the next token");
                        self.function_declaration()
                    }
                    _ => self.statement(),
                }
            }
            _ => self.statement(),
        }
    }

    pub fn parse_ast(&mut self) -> Program {
        let mut program = Program::new();
        let mut body = vec![];
        loop {
            let node = self.source_element();

            match node {
                Some(node) => body.push(node),
                None => {
                    program.set_body(body);
                    break;
                }
            }
        }
        program
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    body: Vec<Rc<Node>>,
}

impl Program {
    pub fn new() -> Self {
        Self { body: vec![] }
    }

    pub fn set_body(&mut self, body: Vec<Rc<Node>>) {
        self.body = body;
    }

    pub fn body(&self) -> &Vec<Rc<Node>> {
        &self.body
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    fn create_parser(input: String) -> JsParser {
        let lexer = JsLexer::new(input);
        let parser = JsParser::new(lexer);
        parser
    }

    #[test]
    fn test_empty() {
        let mut parser = create_parser("".to_string());
        let expected = Program::new();
        assert_eq!(expected, parser.parse_ast());
    }

    #[test]
    fn test_num() {
        let mut parser = create_parser("42".to_string());
        let mut expected = Program::new();
        let mut body = Vec::new();
        body.push(Rc::new(Node::ExpressionStatement(Some(Rc::new(
            Node::NumericLiteral(42),
        )))));
        expected.set_body(body);
        assert_eq!(expected, parser.parse_ast());
    }

    #[test]
    fn test_add_nums() {
        let mut parser = create_parser("1 + 2".to_string());
        let mut expected = Program::new();
        let mut body = Vec::new();
        body.push(Rc::new(Node::ExpressionStatement(Some(Rc::new(
            Node::AdditiveExpression {
                operator: '+',
                left: Some(Rc::new(Node::NumericLiteral(1))),
                right: Some(Rc::new(Node::NumericLiteral(2))),
            },
        )))));
        expected.set_body(body);
        assert_eq!(expected, parser.parse_ast());
    }

    #[test]
    fn test_string() {
        let mut parser = create_parser(r#""string""#.to_string());
        let body = vec![Rc::new(Node::ExpressionStatement(Some(Rc::new(
            Node::StringLiteral("string".to_string()),
        ))))];
        let mut expected = Program::new();
        expected.set_body(body);
        assert_eq!(expected, parser.parse_ast());
    }
}
