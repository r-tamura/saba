use core::{
    borrow::Borrow,
    cell::RefCell,
    fmt::Formatter,
    iter::zip,
    ops::{Add, Sub},
};

use alloc::vec;
use alloc::{
    format,
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};

use crate::renderer::dom::node::NodeKind as DomNodeKind;
use crate::renderer::dom::{api::get_element_by_id, node::Node as DomNode};

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
            RuntimeValue::HtmlElement { object, .. } => {
                format!("HtmlElement: {:#?}", object)
            }
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    id: String,
    params: Vec<Option<Rc<Node>>>,
    body: Option<Rc<Node>>,
}

impl Function {
    fn new(id: String, params: Vec<Option<Rc<Node>>>, body: Option<Rc<Node>>) -> Self {
        Self { id, params, body }
    }
}

pub struct JsRuntime {
    dom_root: Rc<RefCell<DomNode>>,
    env: Rc<RefCell<Environment>>,
    functions: Vec<Rc<Function>>,
}

impl JsRuntime {
    pub fn new(dom_root: Rc<RefCell<DomNode>>) -> Self {
        Self {
            dom_root,
            env: Rc::new(RefCell::new(Environment::new(None))),
            functions: vec![],
        }
    }

    fn find_function(&mut self, name: &str) -> Rc<Function> {
        let f = self.functions.iter().find(|&f| name == f.id.to_string());
        f.expect(&format!("function {:?} is not defined", name))
            .clone()
    }

    fn call_browser_api(
        &mut self,
        func: &RuntimeValue,
        arguments: &[Option<Rc<Node>>],
        env: Rc<RefCell<Environment>>,
    ) -> (bool, Option<RuntimeValue>) {
        match func {
            &RuntimeValue::StringLiteral(ref s) if s == "document.getElementById" => {
                let arg = match self.eval(&arguments[0], env.clone()) {
                    Some(a) => a,
                    None => return (true, None),
                };
                let target = match get_element_by_id(Some(self.dom_root.clone()), &arg.to_string())
                {
                    Some(a) => a,
                    None => return (true, None),
                };
                (
                    true,
                    Some(RuntimeValue::HtmlElement {
                        object: target,
                        property: None,
                    }),
                )
            }
            _ => (false, None),
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
                let left = self.eval(left, env.clone())?;
                let right = self.eval(right, env.clone())?;
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

                let left = left.as_ref()?.clone();
                if let Node::Identifier(left_id) = left.borrow() {
                    let new_value = self.eval(right, env.clone());
                    env.borrow_mut()
                        .update_variable(left_id.to_string(), new_value);
                    return None;
                }

                // domNode.textContent = "hello"のサポート
                if let Some(RuntimeValue::HtmlElement { object, property }) =
                    self.eval(&Some(left), env.clone())
                {
                    let right = self.eval(right, env.clone())?;
                    if let Some(prop) = property {
                        if prop == "textContent" {
                            object
                                .borrow_mut()
                                .set_first_child(Some(Rc::new(RefCell::new(DomNode::new(
                                    DomNodeKind::Text(right.to_string()),
                                )))));
                        }
                    }
                }

                None
            }
            Node::MemberExpression { object, property } => {
                let object_value = self.eval(object, env.clone())?;
                let property_name = match self.eval(property, env.clone()) {
                    Some(value) => value,
                    None => return Some(object_value),
                };

                // オブジェクトがHTML要素の場合にpropertyを更新
                if let RuntimeValue::HtmlElement { object, property } = object_value {
                    assert!(property.is_none());
                    return Some(RuntimeValue::HtmlElement {
                        object,
                        property: Some(property_name.to_string()),
                    });
                }

                Some(object_value + RuntimeValue::StringLiteral(".".to_string()) + property_name)
            }
            Node::VariableDeclarationList { declarations } => {
                for declaration in declarations {
                    self.eval(&declaration, env.clone());
                }
                None
            }
            Node::VariableDeclaration { id, initializer } => {
                if let Some(node) = id {
                    if let Node::Identifier(name) = node.borrow() {
                        let initilizer = self.eval(&initializer, env.clone());
                        env.borrow_mut().add_variable(name.to_string(), initilizer);
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
            // Q. 最後の式が'return'でない場合でも返り値になってしまう?
            Node::BlockStatement { body } => body
                .iter()
                .fold(None, |_acc, stmt| self.eval(stmt, env.clone())),
            Node::ReturnStatement { argument } => self.eval(&argument, env.clone()),
            Node::FunctionDeclaration { id, params, body } => {
                let node = self.eval(id, env.clone())?;
                if let RuntimeValue::StringLiteral(id) = node {
                    let cloned_body = body.as_ref().map(|b| b.clone());
                    self.functions
                        .push(Rc::new(Function::new(id, params.to_vec(), cloned_body)));
                };
                None
            }
            Node::CallExpression { callee, arguments } => {
                // 新しい関数のスコープを作成
                let new_env = Rc::new(RefCell::new(Environment::new(Some(env))));
                let function_name = self.eval(callee, new_env.clone())?;
                // ブラウザAPIの呼び出し
                let (called, result) =
                    self.call_browser_api(&function_name, arguments, new_env.clone());
                if called {
                    return result;
                }

                // JavaScriptランタイムに定義された関数呼び出し
                let function_name = match function_name {
                    RuntimeValue::StringLiteral(s) => s,
                    _ => panic!("expect a function name, but got {:?}", function_name),
                };

                let function = self.find_function(&function_name);

                // 関数呼び出しの引数を関数のスコープへ追加
                assert!(arguments.len() == function.params.len());
                for (arg, param) in zip(arguments, &function.params) {
                    let param_name = self.eval(&param, new_env.clone());
                    // Note: Node::IdentifierをevalしたときにinitializerがないとStringValueとして評価される
                    // TODO: 同名の変数が外のスコープある引数は正しく評価できない
                    if let Some(RuntimeValue::StringLiteral(name)) = param_name {
                        new_env
                            .borrow_mut()
                            .add_variable(name, self.eval(arg, new_env.clone()));
                    }
                }

                self.eval(&function.body.clone(), new_env.clone())
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
        let dom = Rc::new(RefCell::new(DomNode::new(DomNodeKind::Document)));
        let mut runtime = JsRuntime::new(dom);
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

    #[test]
    fn test_add_function_and_num() {
        let actuals = eval(r#"function foo() { return 42; } foo()+1"#);
        let expected = [None, Some(RuntimeValue::Number(43))];
        assert_eq!(actuals, expected);
    }

    #[test]
    fn test_define_function_with_args() {
        let actuals = eval(r#"function foo(a, b) { return a + b; } foo(1, 2) + 3;"#);
        let expected = [None, Some(RuntimeValue::Number(6))];
        assert_eq!(actuals, expected);
    }

    #[test]
    fn test_local_variable() {
        let actuals = eval(r#"var a=42; function foo() { var a=1; return a; } foo()+a"#);
        let expected = [None, None, Some(RuntimeValue::Number(43))];
        assert_eq!(actuals, expected);
    }

    // #[test]
    // fn test_function_argument_shadowing_outer_variable() {
    //     let actuals = eval(r#"var a=42; function foo(a) { return a + 1; } foo(1)+a"#);
    //     let expected = [None, None, Some(RuntimeValue::Number(44))];
    //     assert_eq!(actuals, expected);
    // }
}
