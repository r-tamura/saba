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

/// (name: String, value: Option<RuntimeValue);
type VariableMap = Vec<(String, Option<RuntimeValue>)>;

/// https://262.ecma-international.org/#sec-environment-records
#[derive(Debug, Clone)]
pub struct Environment {
    variables: VariableMap,
    outer: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    fn new(outer: Option<Rc<RefCell<Environment>>>) -> Self {
        Self {
            variables: VariableMap::new(),
            outer,
        }
    }

    fn get_variable(&self, name: &str) -> Option<RuntimeValue> {
        // ローカルスコープから変数を探索
        for variable in &self.variables {
            if variable.0 == name {
                return variable.1.clone();
            }
        }
        // 外側のスコープから変数を探索
        self.outer.as_ref()?.borrow_mut().get_variable(name)
    }

    fn add_variable(&mut self, name: String, value: Option<RuntimeValue>) {
        self.variables.push((name, value));
    }

    fn update_variable(&mut self, name: String, value: Option<RuntimeValue>) {
        if let Some((i, _variable)) = self
            .variables
            .iter()
            .enumerate()
            .find(|(_, variable)| variable.0 == name)
        {
            self.variables.remove(i);
            self.variables.push((name, value));
        }
    }
}

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
        match (&self, &rhs) {
            (RuntimeValue::Number(left), RuntimeValue::Number(right)) => {
                RuntimeValue::Number(left + right)
            }
            _ => RuntimeValue::StringLiteral(self.to_string() + &rhs.to_string()),
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
            // NaN
            _ => RuntimeValue::Number(u64::MIN),
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

pub struct JsRuntime {
    env: Rc<RefCell<Environment>>,
}

impl JsRuntime {
    pub fn new() -> Self {
        Self {
            env: Rc::new(RefCell::new(Environment::new(None))),
        }
    }

    pub fn execute(&mut self, program: &Program) {
        for node in program.body() {
            self.eval(&Some(node.clone()), self.env.clone());
        }
    }

    pub fn eval(
        &mut self,
        node: &Option<Rc<Node>>,
        env: Rc<RefCell<Environment>>,
    ) -> Option<RuntimeValue> {
        let node = node.clone()?;
        match node.borrow() {
            Node::ExpressionStatement(expr) => return self.eval(&expr, env.clone()),
            Node::AdditiveExpression {
                operator,
                left,
                right,
            } => {
                let left = self.eval(&left, env.clone())?;
                let right = self.eval(&right, env.clone())?;
                match *operator {
                    '+' => Some(left + right),
                    '-' => Some(left - right),
                    _ => None,
                }
            }
            Node::AssignmentExpression {
                operator,
                left,
                right,
            } => {
                if operator != &'=' {
                    return None;
                }

                let left = left.as_ref()?;
                if let Node::Identifier(left_id) = left.borrow() {
                    let new_value = self.eval(right, env.clone());
                    env.borrow_mut()
                        .update_variable(left_id.to_string(), new_value);
                }

                None
            }
            Node::MemberExpression { .. } => {
                todo!();
            }
            Node::VariableDeclarationList { declarations } => {
                for declaration in declarations {
                    self.eval(&declaration, env.clone());
                }
                None
            }
            Node::VariableDeclaration {
                id,
                initializer: initial_value,
            } => {
                if let Some(node) = id {
                    if let Node::Identifier(name) = node.borrow() {
                        let initial_value = self.eval(&initial_value, env.clone());
                        env.borrow_mut()
                            .add_variable(name.to_string(), initial_value);
                    }
                }
                None
            }
            Node::Identifier(name) => env
                .borrow_mut()
                .get_variable(name)
                // 初回変数名定義の場合は、値が保存されていないので文字列で扱う
                .or(Some(RuntimeValue::StringLiteral(name.to_string()))),
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
        let env = Rc::new(RefCell::new(Environment::new(None)));

        ast.body()
            .iter()
            .map(|node| runtime.eval(&Some(node.clone()), env.clone()))
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

    #[test]
    fn test_assign_variable() {
        let actuals = eval(r#"var foo=42;"#);
        let expected = [None];
        assert_eq!(actuals, expected);
    }

    #[test]
    fn test_add_variable_and_num() {
        let actuals = eval(r#"var foo=42; foo+1"#);
        let expected = [None, Some(RuntimeValue::Number(43))];
        assert_eq!(actuals, expected);
    }

    #[test]
    fn test_reassign_variable() {
        let actuals = eval(r#"var foo=42; foo=1; foo"#);
        let expected = [None, None, Some(RuntimeValue::Number(1))];
        assert_eq!(actuals, expected);
    }
}
