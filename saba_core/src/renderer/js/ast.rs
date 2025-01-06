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
    VariableDeclarationList {
        declarations: Vec<Option<Rc<Node>>>,
    },
    VariableDeclaration {
        id: Option<Rc<Node>>,
        initializer: Option<Rc<Node>>,
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
    CallExpression {
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
    pub fn new_assignment_expression(
        operator: char,
        left: Option<Rc<Node>>,
        right: Option<Rc<Node>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::AssignmentExpression {
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

    pub fn new_variable_declaration(
        id: Option<Rc<Self>>,
        initializer: Option<Rc<Self>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::VariableDeclaration { id, initializer }))
    }

    pub fn new_variable_declaration_list(declarations: Vec<Option<Rc<Self>>>) -> Option<Rc<Self>> {
        Some(Rc::new(Node::VariableDeclarationList { declarations }))
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
        Some(Rc::new(Node::CallExpression { callee, arguments }))
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
            Token::Identifier(name) => Node::new_identifier(name),
            Token::Number(n) => Node::new_numeric_literal(n),
            Token::StringLiteral(s) => Node::new_string_literal(s),
            _ => None,
        }
    }

    fn member_expression(&mut self) -> Option<Rc<Node>> {
        let expr = self.primary_expression();
        match self.t.peek() {
            // メソッド呼び出しの場合
            Some(Token::Punctuator(c)) if c == &'.' => {
                assert!(self.t.next().is_some());
                Node::new_member_expression(expr, self.identifier())
            }
            _ => expr,
        }
    }

    fn arguments(&mut self) -> Vec<Option<Rc<Node>>> {
        let mut arguments = vec![];
        loop {
            match self.t.peek() {
                Some(token) => match token {
                    Token::Punctuator(c) if c == &')' => {
                        assert!(self.t.next().is_some());
                        break;
                    }
                    Token::Punctuator(c) if c == &',' => {
                        assert!(self.t.next().is_some());
                    }
                    _ => arguments.push(self.assignment_expression()),
                },
                None => break,
            }
        }
        arguments
    }

    fn left_hand_side_expression(&mut self) -> Option<Rc<Node>> {
        let expr = self.member_expression();

        let token = match self.t.peek() {
            Some(token) => token,
            None => return expr,
        };

        match token {
            Token::Punctuator(c) => {
                if c == &'(' {
                    // '{'を消費する}
                    assert!(self.t.next().is_some());
                    return Node::new_call_expression(expr, self.arguments());
                }
                expr
            }
            _ => expr,
        }
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
        // AssignExpression ::= AdditiveEpxression ( "=" AdditiveExpression )*
        let expr = self.additive_expression();
        let token = match self.t.peek() {
            Some(token) => token,
            None => return expr,
        };

        match token {
            Token::Punctuator('=') => {
                // Consume '='
                assert!(self.t.next().is_some());
                Node::new_assignment_expression('=', expr, self.assignment_expression())
            }
            _ => expr,
        }
    }

    fn initialiser(&mut self) -> Option<Rc<Node>> {
        // Initializer ::= "=" AssignmentExpression
        match self.t.next()? {
            Token::Punctuator(c) => match c {
                '=' => self.assignment_expression(),
                _ => None,
            },
            _ => None,
        }
    }

    fn identifier(&mut self) -> Option<Rc<Node>> {
        match self.t.next()? {
            Token::Identifier(name) => Node::new_identifier(name),
            _ => None,
        }
    }

    fn variable_declaration(&mut self) -> Option<Rc<Node>> {
        // 1つの変数宣言のみサポート
        // ```js
        // var x = 42, y = "a";
        // ```
        // のような複数の変数宣言はサポートしていない
        let ident = self.identifier();
        let declarator = Node::new_variable_declaration(ident, self.initialiser());
        let declarations = vec![declarator];
        Node::new_variable_declaration_list(declarations)
    }

    fn statement(&mut self) -> Option<Rc<Node>> {
        let node = match self.t.peek()? {
            Token::Keyword(keyword) => match keyword.as_ref() {
                "var" => {
                    // Consume 'var' keyword
                    assert!(self.t.next().is_some());
                    self.variable_declaration()
                }
                "return" => {
                    assert!(self.t.next().is_some());
                    Node::new_return_statement(self.assignment_expression())
                }
                _ => None,
            },
            _ => Node::new_expression_statement(self.assignment_expression()),
        };

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
        // '{'を消費する
        match self.t.next() {
            Some(token) => match token {
                Token::Punctuator(c) => assert_eq!(c, '{'),
                _ => unimplemented!(
                    "function should have open curly blacket buto got {:?}",
                    token
                ),
            },
            _ => unimplemented!("function should have open curly blacket but got None"),
        }

        let mut body = vec![];
        loop {
            match self.t.peek() {
                Some(Token::Punctuator(c)) if c == &'}' => {
                    assert!(self.t.next().is_some());
                    break;
                }
                _ => body.push(self.source_element()),
            }
        }
        Node::new_block_statement(body)
    }

    fn parameter_list(&mut self) -> Vec<Option<Rc<Node>>> {
        // '('を消費する
        match self.t.next() {
            Some(token) => match token {
                Token::Punctuator(c) => assert_eq!(c, '('),
                _ => unimplemented!("function should have `(` but got {:?}", token),
            },
            None => unimplemented!("function should have `(` but got None"),
        }

        let mut params = vec![];
        while let Some(token) = self.t.peek() {
            match token {
                Token::Punctuator(c) => {
                    match c {
                        ')' => {
                            assert!(self.t.next().is_some());
                            break;
                        }
                        ',' => {
                            assert!(self.t.next().is_some());
                        }
                        _ => {}
                    };
                }
                _ => {
                    params.push(self.identifier());
                }
            }
        }

        params
    }

    fn function_declaration(&mut self) -> Option<Rc<Node>> {
        Node::new_function_declaration(
            self.identifier(),
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

    fn create_ast(input: Vec<Rc<Node>>) -> Program {
        let mut program = Program::new();
        let mut body = vec![];
        for node in input {
            body.push(node.clone());
        }
        program.set_body(body);
        program
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

    #[test]
    fn test_single_quoted_string() {
        let mut parser = create_parser(r#"'string'"#.to_string());
        let expected = create_ast(vec![Rc::new(Node::ExpressionStatement(Some(Rc::new(
            Node::StringLiteral("string".to_string()),
        ))))]);
        assert_eq!(expected, parser.parse_ast());
    }

    #[test]
    fn test_assign_variable() {
        let mut parser = create_parser(r#"var foo="bar";"#.to_string());
        let mut expected = Program::new();
        let mut body = Vec::new();
        body.push(Rc::new(Node::VariableDeclarationList {
            declarations: [Some(Rc::new(Node::VariableDeclaration {
                id: Some(Rc::new(Node::Identifier("foo".to_string()))),
                initializer: Some(Rc::new(Node::StringLiteral("bar".to_string()))),
            }))]
            .to_vec(),
        }));
        expected.set_body(body);
        assert_eq!(expected, parser.parse_ast());
    }

    #[test]
    fn test_add_variable_and_num() {
        let mut parser = create_parser("var foo=42; var result=foo+1;".to_string());
        let mut expected = Program::new();
        let mut body = Vec::new();
        body.push(Rc::new(Node::VariableDeclarationList {
            declarations: [Some(Rc::new(Node::VariableDeclaration {
                id: Some(Rc::new(Node::Identifier("foo".to_string()))),
                initializer: Some(Rc::new(Node::NumericLiteral(42))),
            }))]
            .to_vec(),
        }));
        body.push(Rc::new(Node::VariableDeclarationList {
            declarations: [Some(Rc::new(Node::VariableDeclaration {
                id: Some(Rc::new(Node::Identifier("result".to_string()))),
                initializer: Some(Rc::new(Node::AdditiveExpression {
                    operator: '+',
                    left: Some(Rc::new(Node::Identifier("foo".to_string()))),
                    right: Some(Rc::new(Node::NumericLiteral(1))),
                })),
            }))]
            .to_vec(),
        }));
        expected.set_body(body);
        assert_eq!(expected, parser.parse_ast());
    }

    #[test]
    fn test_define_function() {
        let mut parser = create_parser("function foo() { return 42; }".to_string());
        let mut expected = Program::new();
        let mut body = Vec::new();
        body.push(Rc::new(Node::FunctionDeclaration {
            id: Some(Rc::new(Node::Identifier("foo".to_string()))),
            params: [].to_vec(),
            body: Some(Rc::new(Node::BlockStatement {
                body: [Some(Rc::new(Node::ReturnStatement {
                    argument: Some(Rc::new(Node::NumericLiteral(42))),
                }))]
                .to_vec(),
            })),
        }));
        expected.set_body(body);
        assert_eq!(expected, parser.parse_ast());
    }

    #[test]
    fn test_add_function_add_num() {
        let input = "function foo() { return 42; } var result = foo() + 1;".to_string();
        let lexer = JsLexer::new(input);
        let mut parser = JsParser::new(lexer);
        let mut expected = Program::new();
        let mut body = Vec::new();
        body.push(Rc::new(Node::FunctionDeclaration {
            id: Some(Rc::new(Node::Identifier("foo".to_string()))),
            params: [].to_vec(),
            body: Some(Rc::new(Node::BlockStatement {
                body: [Some(Rc::new(Node::ReturnStatement {
                    argument: Some(Rc::new(Node::NumericLiteral(42))),
                }))]
                .to_vec(),
            })),
        }));
        body.push(Rc::new(Node::VariableDeclarationList {
            declarations: [Some(Rc::new(Node::VariableDeclaration {
                id: Some(Rc::new(Node::Identifier("result".to_string()))),
                initializer: Some(Rc::new(Node::AdditiveExpression {
                    operator: '+',
                    left: Some(Rc::new(Node::CallExpression {
                        callee: Some(Rc::new(Node::Identifier("foo".to_string()))),
                        arguments: [].to_vec(),
                    })),
                    right: Some(Rc::new(Node::NumericLiteral(1))),
                })),
            }))]
            .to_vec(),
        }));
        expected.set_body(body);
        assert_eq!(expected, parser.parse_ast());
    }

    #[test]
    fn test_define_function_with_args() {
        let mut parser = create_parser("function foo(a, b) { return a+b; }".to_string());
        let expected = create_ast(vec![Rc::new(Node::FunctionDeclaration {
            id: Some(Rc::new(Node::Identifier("foo".to_string()))),
            params: [
                Some(Rc::new(Node::Identifier("a".to_string()))),
                Some(Rc::new(Node::Identifier("b".to_string()))),
            ]
            .to_vec(),
            body: Some(Rc::new(Node::BlockStatement {
                body: [Some(Rc::new(Node::ReturnStatement {
                    argument: Some(Rc::new(Node::AdditiveExpression {
                        operator: '+',
                        left: Some(Rc::new(Node::Identifier("a".to_string()))),
                        right: Some(Rc::new(Node::Identifier("b".to_string()))),
                    })),
                }))]
                .to_vec(),
            })),
        })]);
        assert_eq!(expected, parser.parse_ast());
    }
}
