use core::cell::RefCell;

use alloc::{
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};

use crate::renderer::dom::node::{Element, NodeKind};

use super::node::{ElementKind, Node};

pub fn get_element_by_id(node: Option<Rc<RefCell<Node>>>, id: &str) -> Option<Rc<RefCell<Node>>> {
    let node = node?;
    if let NodeKind::Element(element) = node.borrow().kind() {
        if let Some(attr) = element.get_attr("id") {
            if attr.value() == id {
                return Some(node.clone());
            }
        }
    }
    for node in [node.borrow().first_child(), node.borrow().next_sibling()] {
        if let Some(result) = get_element_by_id(node, id) {
            return Some(result);
        }
    }
    None
}

pub fn get_target_element_node(
    root: Option<Rc<RefCell<Node>>>,
    element_kind: ElementKind,
) -> Option<Rc<RefCell<Node>>> {
    let root = root?;
    if root.borrow().kind()
        == NodeKind::Element(Element::new(&element_kind.to_string(), Vec::new()))
    {
        return Some(root.clone());
    }

    if let Some(result) = get_target_element_node(root.borrow().first_child(), element_kind) {
        return Some(result);
    }
    let result = get_target_element_node(root.borrow().next_sibling(), element_kind);
    result
}

pub fn get_style_content(root: Rc<RefCell<Node>>) -> String {
    get_target_element_node(Some(root), ElementKind::Style)
        .and_then(|node| node.borrow().first_child())
        .and_then(|text_node| match text_node.borrow().kind() {
            NodeKind::Text(ref s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or("".to_string())
}

pub fn get_js_content(root: Rc<RefCell<Node>>) -> String {
    let js_node = match get_target_element_node(Some(root), ElementKind::Script) {
        Some(node) => node,
        None => return "".to_string(),
    };
    let text_node = match js_node.borrow().first_child() {
        Some(node) => node,
        None => return "".to_string(),
    };
    let content = match text_node.borrow().kind() {
        NodeKind::Text(s) => s,
        _ => "".to_string(),
    };
    content
}
