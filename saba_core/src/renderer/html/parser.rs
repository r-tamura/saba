use core::{cell::RefCell, str::FromStr};

use alloc::{rc::Rc, string::String, vec::Vec};

use crate::renderer::{
    dom::node::{Element, ElementKind, Node, NodeKind, Window},
    html::token::HtmlToken,
};

use super::{attribute::Attribute, token::HtmlTokenizer};

const SPACE: char = ' ';
const LINE_FEED: char = '\n';

/// https://html.spec.whatwg.org/multipage/parsing.html#the-insertion-mode
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    AfterHead,
    InBody,
    Text,
    AfterBody,
    AfterAfterBody,
}

#[derive(Debug, Clone)]
pub struct HtmlParser {
    window: Rc<RefCell<Window>>,
    /// https://html.spec.whatwg.org/multipage/parsing.html#original-insertion-mode
    mode: InsertionMode,
    // 状態遷移したときに、以前のInsertionModeを保存するために利用される
    original_insertion_mode: InsertionMode,
    /// https://html.spec.whatwg.org/multipage/parsing.html#the-stack-of-open-elements
    // 開いているタグのスタック
    stack_of_open_elements: Vec<Rc<RefCell<Node>>>,
    t: HtmlTokenizer,
}

impl HtmlParser {
    pub fn new(t: HtmlTokenizer) -> Self {
        Self {
            window: Rc::new(RefCell::new(Window::new())),
            mode: InsertionMode::Initial,
            original_insertion_mode: InsertionMode::Initial,
            stack_of_open_elements: Vec::new(),
            t,
        }
    }

    fn contains_in_stack(&mut self, element_kind: ElementKind) -> bool {
        for i in 0..self.stack_of_open_elements.len() {
            if self.stack_of_open_elements[i].borrow().element_kind() == Some(element_kind) {
                return true;
            }
        }

        false
    }

    fn pop_until(&mut self, element_kind: ElementKind) {
        assert!(
            self.contains_in_stack(element_kind),
            "stack doesn't have an element {:?}",
            element_kind,
        );

        loop {
            let current = match self.stack_of_open_elements.pop() {
                Some(node) => node,
                None => return,
            };

            if current.borrow().element_kind() == Some(element_kind) {
                return;
            }
        }
    }

    fn pop_current_node(&mut self, element_kind: ElementKind) -> bool {
        let current = match self.stack_of_open_elements.last() {
            Some(node) => node,
            None => return false,
        };

        if current.borrow().element_kind() == Some(element_kind) {
            self.stack_of_open_elements.pop();
            return true;
        }

        false
    }

    /// 親ノードの持つ子供の最後尾に新しいノードを追加します
    fn insert_node(&mut self, parent: Rc<RefCell<Node>>, new_node: Node) {
        // if HtmlParser::has_child(&current) {
        //     // last_childと等価?
        //     let mut last_sibling = current.borrow().first_child();
        //     loop {
        //         last_sibling = match last_sibling {
        //             Some(ref node) => {
        //                 if node.borrow().next_sibling().is_some() {
        //                     node.borrow().next_sibling()
        //                 } else {
        //                     break;
        //                 }
        //             }
        //             None => unimplemented!("last_sibiling shoud be Some"),
        //         }
        //     }
        //     let last_sibling = current.borrow_mut().last_child();
        //     last_sibling
        //         .upgrade()
        //         .unwrap()
        //         .borrow_mut()
        //         .set_next_sibling(Some(new_node.clone()));
        //     new_node.borrow_mut().set_previous_sibling(Rc::downgrade(
        //         &last_sibling.upgrade().expect("last_sibling should be Some"),
        //     ))
        // } else {
        //     current.borrow_mut().set_first_child(Some(new_node.clone()));
        // }

        let new_node = Rc::new(RefCell::new(new_node));
        let mut current_node = parent.borrow_mut();
        match current_node.last_child().upgrade() {
            Some(last_child) => {
                last_child
                    .borrow_mut()
                    .set_next_sibling(Some(new_node.clone()));
            }
            None => {
                current_node.set_first_child(Some(new_node.clone()));
            }
        }
        current_node.set_last_child(Rc::downgrade(&new_node));
        new_node.borrow_mut().set_parent(Rc::downgrade(&parent));

        self.stack_of_open_elements.push(new_node);
    }

    fn create_char(&self, c: char) -> Node {
        let mut s = String::new();
        s.push(c);
        Node::new(NodeKind::Text(s))
    }

    /// 現在のノードによって以下の2つの処理を行います
    /// 現在のノードがTextノードのとき, テキストの最後にに文字を挿入します
    /// 現在のノードが上記以外のとき, 最後の子ノードの次のノードとしてTextノードを追加します
    ///   ただし、引数の文字が無視されるべき文字のときは、Textノードを追加しません
    fn insert_char(&mut self, c: char) {
        let current = match self.stack_of_open_elements.last() {
            Some(node) => node.clone(),
            None => return,
        };

        if let NodeKind::Text(ref mut s) = current.borrow_mut().kind {
            s.push(c);
            return;
        }

        if c == SPACE || c == LINE_FEED {
            return;
        }

        // let node = Rc::new(RefCell::new(self.create_char(c)));
        self.insert_node(current, self.create_char(c));
    }

    fn create_element(&self, tag: &str, attributes: Vec<Attribute>) -> Node {
        Node::new(NodeKind::Element(Element::new(tag, attributes)))
    }

    fn insert_element(&mut self, tag: &str, attributes: Vec<Attribute>) {
        let current = match self.stack_of_open_elements.last() {
            Some(node) => node.clone(),
            // Documentが最初にスタックに積まれているという仕様
            None => self.window.borrow().document(),
        };

        // let new_node = Rc::new(RefCell::new(self.create_element(tag, attributes)));
        self.insert_node(current, self.create_element(tag, attributes));
    }

    pub fn construct_tree(&mut self) -> Rc<RefCell<Window>> {
        let mut token = self.t.next();

        while token.is_some() {
            match self.mode {
                InsertionMode::Initial => {
                    // <!doctype html>のようなトークンは文字トークンになり、文字トークンは無視する
                    if let Some(HtmlToken::Char(_)) = token {
                        token = self.t.next();
                        continue;
                    }

                    self.mode = InsertionMode::BeforeHtml;
                    continue;
                }
                InsertionMode::BeforeHtml => {
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == SPACE || c == LINE_FEED {
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::StartTag {
                            ref tag,
                            ref attributes,
                            ..
                        }) => {
                            if tag == "html" {
                                self.insert_element(tag, attributes.to_vec());
                                self.mode = InsertionMode::BeforeHead;
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::Eof) | None => return self.window.clone(),
                        _ => {}
                    }

                    self.insert_element("head", Vec::new());
                    self.mode = InsertionMode::InHead;
                    continue;
                }
                InsertionMode::BeforeHead => match token {
                    Some(HtmlToken::Char(c)) => {
                        if c == SPACE || c == LINE_FEED {
                            token = self.t.next();
                            continue;
                        }
                    }
                    Some(HtmlToken::StartTag {
                        ref tag,
                        ref attributes,
                        ..
                    }) => {
                        if tag == "head" {
                            self.insert_element(tag, attributes.to_vec());
                            self.mode = InsertionMode::InHead;
                            token = self.t.next();
                            continue;
                        }
                    }
                    Some(HtmlToken::Eof) | None => {
                        break;
                    }
                    _ => {}
                },
                // <head>タグ内では<style>タグ, <script>タグのみサポート
                InsertionMode::InHead => {
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == SPACE || c == LINE_FEED {
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::StartTag {
                            ref tag,
                            ref attributes,
                            ..
                        }) => {
                            if tag == "style" || tag == "script" {
                                self.insert_element(tag, attributes.to_vec());
                                self.original_insertion_mode = self.mode;
                                self.mode = InsertionMode::Text;
                                token = self.t.next();
                                continue;
                            }
                            // 仕様外の挙動
                            // <head>が省略されているHTML文書で無限ループが起きてしまうことへの対応
                            if tag == "body" {
                                self.pop_until(ElementKind::Head);
                                self.mode = InsertionMode::AfterHead;
                                continue;
                            }
                            // サポートしているその他のタグ(?)
                            if let Ok(_element_kind) = ElementKind::from_str(tag) {
                                self.pop_until(ElementKind::Head);
                                self.mode = InsertionMode::AfterHead;
                                continue;
                            }
                        }
                        Some(HtmlToken::EndTag { ref tag }) => {
                            if tag == "head" {
                                self.mode = InsertionMode::AfterHead;
                                token = self.t.next();
                                self.pop_until(ElementKind::Head);
                                continue;
                            }
                        }
                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }
                    }
                    // サポートしていないタグは無視する
                    token = self.t.next();
                    continue;
                }
                InsertionMode::AfterHead => {
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == SPACE || c == LINE_FEED {
                                self.insert_char(c);
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::StartTag {
                            ref tag,
                            ref attributes,
                            ..
                        }) => {
                            if tag == "body" {
                                self.insert_element(tag, attributes.to_vec());
                                token = self.t.next();
                                self.mode = InsertionMode::InBody;
                                continue;
                            }
                        }
                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }
                        _ => {}
                    }
                    // bodyタグが存在しない場合に自動挿入
                    self.insert_element("body", Vec::new());
                    self.mode = InsertionMode::InBody;
                    continue;
                }
                InsertionMode::InBody => match token {
                    Some(HtmlToken::StartTag {
                        ref tag,
                        ref attributes,
                        ..
                    }) => match tag.as_str() {
                        "p" => {
                            self.insert_element(tag, attributes.to_vec());
                            token = self.t.next();
                            continue;
                        }
                        "h1" | "h2" => {
                            self.insert_element(tag, attributes.to_vec());
                            token = self.t.next();
                            continue;
                        }
                        "a" => {
                            self.insert_element(tag, attributes.to_vec());
                            token = self.t.next();
                            continue;
                        }
                        _ => {
                            token = self.t.next();
                        }
                    },
                    Some(HtmlToken::EndTag { ref tag }) => match tag.as_str() {
                        "body" => {
                            self.mode = InsertionMode::AfterBody;
                            token = self.t.next();
                            if !self.contains_in_stack(ElementKind::Body) {
                                // すでにbody開始タグがあると失敗
                                continue;
                            }
                            self.pop_until(ElementKind::Body);
                            continue;
                        }
                        "html" => {
                            if self.pop_current_node(ElementKind::Body) {
                                self.mode = InsertionMode::AfterBody;
                                assert!(self.pop_current_node(ElementKind::Html));
                            } else {
                                token = self.t.next();
                            }
                            continue;
                        }
                        "p" => {
                            let element_kind = ElementKind::from_str(tag)
                                .expect("failed to convert string to ElementKind");
                            token = self.t.next();
                            self.pop_until(element_kind);
                            continue;
                        }
                        "h1" | "h2" => {
                            let element_kind = ElementKind::from_str(tag)
                                .expect("failed to convert string to ElementKind");
                            token = self.t.next();
                            self.pop_until(element_kind);
                            continue;
                        }
                        "a" => {
                            let element_kind = ElementKind::from_str(tag)
                                .expect("failed to convert string to ElementKind");
                            token = self.t.next();
                            self.pop_until(element_kind);
                            continue;
                        }
                        _ => {
                            token = self.t.next();
                        }
                    },
                    Some(HtmlToken::Eof) | None => {
                        return self.window.clone();
                    }
                    Some(HtmlToken::Char(c)) => {
                        self.insert_char(c);
                        token = self.t.next();
                        continue;
                    }
                },
                InsertionMode::Text => match token {
                    Some(HtmlToken::EndTag { ref tag }) => match tag.as_str() {
                        "style" => {
                            self.pop_until(ElementKind::Style);
                            self.mode = self.original_insertion_mode;
                            token = self.t.next();
                            continue;
                        }
                        "script" => {
                            self.pop_until(ElementKind::Script);
                            self.mode = self.original_insertion_mode;
                            token = self.t.next();
                            continue;
                        }
                        _ => {}
                    },
                    Some(HtmlToken::Char(c)) => {
                        self.insert_char(c);
                        token = self.t.next();
                        continue;
                    }
                    Some(HtmlToken::Eof) | None => {
                        return self.window.clone();
                    }
                    _ => {}
                },
                InsertionMode::AfterBody => {
                    match token {
                        Some(HtmlToken::Char(_c)) => {
                            token = self.t.next();
                            continue;
                        }
                        Some(HtmlToken::EndTag { ref tag }) => {
                            if tag == "html" {
                                self.mode = InsertionMode::AfterAfterBody;
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::Eof) | None => {
                            return self.window.clone();
                        }
                        _ => {}
                    }
                    // パースできないHTMLでもできる限りHTMLとして解釈するように
                    self.mode = InsertionMode::InBody;
                }
                InsertionMode::AfterAfterBody => match token {
                    Some(HtmlToken::Char(_c)) => {
                        token = self.t.next();
                        continue;
                    }
                    Some(HtmlToken::Eof) | None => {
                        return self.window.clone();
                    }
                    _ => {}
                },
            }
        }

        Rc::clone(&self.window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn test_empty() {
        let html = "".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let expected = Rc::new(RefCell::new(Node::new(NodeKind::Document)));

        assert_eq!(expected, window.borrow().document());
    }

    #[test]
    fn test_body() {
        let html = "<html><head></head><body></body></html>".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Document))),
            document
        );

        let html = document
            .borrow()
            .first_child()
            .expect("failed to get a first child of document");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "html",
                Vec::new()
            ))))),
            html
        );

        let head = html
            .borrow()
            .first_child()
            .expect("failed to get a first child of html");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "head",
                Vec::new()
            ))))),
            head
        );

        let body = head
            .borrow()
            .next_sibling()
            .expect("failed to get a next sibling of head");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "body",
                Vec::new()
            ))))),
            body
        );
    }

    #[test]
    fn test_text() {
        let html = "<html><head></head><body>text</body></html>".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Document))),
            document
        );

        let html = document
            .borrow()
            .first_child()
            .expect("failed to get a first child of document");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "html",
                Vec::new()
            ))))),
            html
        );

        let body = html
            .borrow()
            .first_child()
            .expect("failed to get a first child of document")
            .borrow()
            .next_sibling()
            .expect("failed to get a next sibling of head");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "body",
                Vec::new()
            ))))),
            body
        );

        let text = body
            .borrow()
            .first_child()
            .expect("failed to get a first child of document");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Text("text".to_string())))),
            text
        );
    }

    #[test]
    fn test_multiple_nodes() {
        let html = "<html><head></head><body><p><a foo=bar>text</a></p></body></html>".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();

        let body = document
            .borrow()
            .first_child()
            .expect("failed to get a first child of document")
            .borrow()
            .first_child()
            .expect("failed to get a first child of document")
            .borrow()
            .next_sibling()
            .expect("failed to get a next sibling of head");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "body",
                Vec::new()
            ))))),
            body
        );

        let p = body
            .borrow()
            .first_child()
            .expect("failed to get a first child of body");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "p",
                Vec::new()
            ))))),
            p
        );

        let mut attr = Attribute::new();
        attr.add_char('f', true);
        attr.add_char('o', true);
        attr.add_char('o', true);
        attr.add_char('b', false);
        attr.add_char('a', false);
        attr.add_char('r', false);
        let a = p
            .borrow()
            .first_child()
            .expect("failed to get a first child of p");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "a",
                vec![attr]
            ))))),
            a
        );

        let text = a
            .borrow()
            .first_child()
            .expect("failed to get a first child of a");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Text("text".to_string())))),
            text
        );
    }

    // <head>タグの開始タグ終了タグの間に改行が存歳するとパースが停止しない不具合
    #[test]
    fn test_html_should_be_parsed_when_newline_exists_betwenn_open_tag_and_close_tag() {
        let html = r#"<html><head>
</head><body></body></html>"#
            .to_string();

        let t = HtmlTokenizer::new(html);

        // Assert
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();
        let body = document
            .borrow()
            .first_child()
            .expect("failed to get a first child of document")
            .borrow()
            .first_child()
            .expect("failed to get a first child of document")
            .borrow()
            .next_sibling()
            .expect("failed to get a next sibling of head");

        assert!(body.borrow().first_child().is_none());
    }
}
