use core::{
    borrow::Borrow,
    cell::RefCell,
    fmt::Formatter,
    ops::{Add, Sub},
};

use alloc::{
    format,
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};

use crate::renderer::dom::node::Node as DomNode;

use super::ast::{Node, Program};

type VariableMap = Vec<(String, Option<RuntimeValue>)>;

/// https://262.ecma-international.org/#sec-ecmascript-language-types

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    /// https://262.ecma-international.org/#sec-numeric-types
    Number(u64),
    /// https://262.ecma-international.org/#sec-ecmascript-language-types-string-type
    StringLiteral(String),
    HtmlElement {
        object: Rc<RefCell<DomNode>>,
        property: Option<String>,
    },
}

impl Add<RuntimeValue> for RuntimeValue {
    type Output = RuntimeValue;

    fn add(self, rhs: RuntimeValue) -> RuntimeValue {
        match (self, rhs) {
            (RuntimeValue::Number(left), RuntimeValue::Number(right)) => {
                RuntimeValue::Number(left + right)
            }
            (RuntimeValue::StringLiteral(left), RuntimeValue::StringLiteral(right)) => {
                RuntimeValue::StringLiteral(left + &right)
            }
            _ => unimplemented!("not supported type"),
        }
    }
}

impl Sub<RuntimeValue> for RuntimeValue {
    type Output = RuntimeValue;

    fn sub(self, rhs: RuntimeValue) -> RuntimeValue {
        match (self, rhs) {
            (RuntimeValue::Number(left), RuntimeValue::Number(right)) => {
                RuntimeValue::Number(left - right)
            }
            _ => unimplemented!("not supported type"),
        }
    }
}

impl core::fmt::Display for RuntimeValue {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        let s = match self {
            RuntimeValue::Number(n) => format!("{}", n),
            RuntimeValue::StringLiteral(s) => s.to_string(),
            _ => todo!(),
        };
        write!(f, "{}", s)
    }
}

pub struct JsRuntime;

impl JsRuntime {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&mut self, program: &Program) {
        for node in program.body() {
            self.eval(&Some(node.clone()));
        }
    }

    pub fn eval(&mut self, node: &Option<Rc<Node>>) -> Option<RuntimeValue> {
        let node = node.clone()?;
        match node.borrow() {
            Node::ExpressionStatement(expr) => return self.eval(&expr),
            Node::AdditiveExpression {
                operator,
                left,
                right,
            } => {
                let left = self.eval(&left)?;
                let right = self.eval(&right)?;
                match *operator {
                    '+' => Some(left + right),
                    '-' => Some(left - right),
                    _ => None,
                }
            }
            Node::AssignmentExpression { .. } => {
                todo!();
            }
            Node::MemberExpression { .. } => {
                todo!();
            }
            Node::NumericLiteral(n) => Some(RuntimeValue::Number(*n)),
            Node::StringLiteral(s) => Some(RuntimeValue::StringLiteral(s.to_string())),
            _ => {
                unimplemented!("node {:?} is not supported yet", node.as_ref());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::js::ast::JsParser;
    use crate::renderer::js::token::JsLexer;

    fn eval(s: &str) -> Vec<Option<RuntimeValue>> {
        let input = s.to_string();
        let lexer = JsLexer::new(input);
        let mut parser = JsParser::new(lexer);
        let ast = parser.parse_ast();
        let mut runtime = JsRuntime::new();

        ast.body()
            .iter()
            .map(|node| runtime.eval(&Some(node.clone())))
            .collect()
    }

    #[test]
    fn test_num() {
        let actuals = eval("42");
        let expected = [Some(RuntimeValue::Number(42))];

        assert_eq!(actuals, expected);
    }

    #[test]
    fn test_add_nums() {
        let actuals = eval("1 + 2");
        let expected = [Some(RuntimeValue::Number(3))];

        assert_eq!(actuals, expected);
    }

    #[test]
    fn test_sub_nums() {
        let actuals = eval("2 - 1");
        let expected = [Some(RuntimeValue::Number(1))];

        assert_eq!(actuals, expected);
    }

    #[test]
    fn test_string() {
        let actuals = eval(r#""a""#);
        let expected = [Some(RuntimeValue::StringLiteral("a".to_string()))];
        assert_eq!(actuals, expected);
    }

    #[test]
    fn test_concatenate_strings() {
        let actuals = eval(r#""a" + "b""#);
        let expected = [Some(RuntimeValue::StringLiteral("ab".to_string()))];
        assert_eq!(actuals, expected);
    }
}
