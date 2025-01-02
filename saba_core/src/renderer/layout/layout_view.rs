use core::cell::RefCell;

use alloc::{rc::Rc, vec::Vec};

use crate::{
    constants::CONTENT_AREA_WIDTH,
    display_item::DisplayItem,
    renderer::{
        css::cssom::StyleSheet,
        dom::{
            api::get_target_element_node,
            node::{ElementKind, Node},
        },
    },
};

use super::layout_object::{
    create_layout_object, LayoutObject, LayoutObjectKind, LayoutPoint, LayoutSize,
};

/// 引数で指定されたノードを先頭としたすべての兄弟ノードのレイアウトオブジェクトを生成します
fn build_layout_siblings(
    first_node: Option<Rc<RefCell<Node>>>,
    parent: &mut Option<Rc<RefCell<LayoutObject>>>,
    cssom: &StyleSheet,
) {
    let mut first_node_layout = build_layout_tree(&first_node, &parent, cssom);
    if first_node.is_some() && first_node_layout.is_none() {
        let mut candidate_node = first_node
            .expect("first child shoud exist")
            .borrow()
            .next_sibling();

        loop {
            first_node_layout = build_layout_tree(&candidate_node, &parent, cssom);

            if first_node_layout.is_none() && candidate_node.is_some() {
                candidate_node = candidate_node
                    .expect("next sibling should exists")
                    .borrow()
                    .next_sibling();
                continue;
            }

            break;
        }
    }
}

/// レイアウトツリーを構築します
/// レイアウトツリーの要素はDOM要素の中から画面に表示される(display: noneでない)要素のみで構成されたものだけになります
fn build_layout_tree(
    node: &Option<Rc<RefCell<Node>>>,
    parent: &Option<Rc<RefCell<LayoutObject>>>,
    cssom: &StyleSheet,
) -> Option<Rc<RefCell<LayoutObject>>> {
    let mut target_node = node.clone();
    let mut current_layout = create_layout_object(&node, parent, cssom);

    while current_layout.is_none() {
        if let Some(node) = target_node {
            target_node = node.borrow().next_sibling();
            current_layout = create_layout_object(&target_node, parent, cssom);
        } else {
            return current_layout;
        }
    }

    let node = match target_node {
        Some(node) => node,
        None => return current_layout,
    };

    // 最初に画面に表示される子ノードをレイアウトツリー上の子ノードとする
    // （画面表示されない子ノードはスキップ）
    let original_first_child = node.borrow().first_child();
    let mut first_child_layout = build_layout_tree(&original_first_child, &current_layout, cssom);
    build_layout_siblings(original_first_child, &mut first_child_layout, cssom);

    // 最初に画面に表示される兄弟ノードをレイアウトツリー上の次の兄弟ノードとする
    // （画面表示されない兄弟ノードはスキップ）
    let original_next_sibling = node.borrow().next_sibling();
    let mut next_sibling_layout = build_layout_tree(&original_next_sibling, &None, cssom);
    build_layout_siblings(original_next_sibling, &mut next_sibling_layout, cssom);

    {
        let current_layout = current_layout
            .as_ref()
            .expect("layout object should exist here");
        current_layout
            .borrow_mut()
            .set_first_child(first_child_layout);
        current_layout
            .borrow_mut()
            .set_next_sibling(next_sibling_layout);
    }

    current_layout
}

#[derive(Debug, Clone)]
pub struct LayoutView {
    root: Option<Rc<RefCell<LayoutObject>>>,
}

impl LayoutView {
    pub fn new(root: Rc<RefCell<Node>>, cssom: &StyleSheet) -> Self {
        let body_root = get_target_element_node(Some(root), ElementKind::Body);

        let mut tree = Self {
            root: build_layout_tree(&body_root, &None, cssom),
        };
        tree.update_layout();

        tree
    }

    /// レイアウトツリーの各ノードのサイズを計算します
    fn calculat_node_size(node: &Option<Rc<RefCell<LayoutObject>>>, parent_size: LayoutSize) {
        let node = match node.as_ref() {
            Some(node) => node,
            None => return,
        };
        // ブロック要素の場合、横幅は親ノードに依存、高さは子ノードに依存します
        if node.borrow().kind() == LayoutObjectKind::Block {
            node.borrow_mut().compute_size(parent_size);
        }

        let first_child = node.borrow().first_child();
        Self::calculat_node_size(&first_child, node.borrow().size());

        let next_sibling = node.borrow().next_sibling();
        Self::calculat_node_size(&next_sibling, parent_size);

        // 子ノードのサイズに依存するものは、子ノードのサイズ決定後に計算する
        // ブロック要素: 高さは子ノードの高さに依存する
        // インライン要素: 横幅、高さは子ノードの横幅、高さに依存する
        node.borrow_mut().compute_size(parent_size);
    }

    fn calculate_node_position(
        node: &Option<Rc<RefCell<LayoutObject>>>,
        parent_point: LayoutPoint,
        prev_sibling_kind: LayoutObjectKind,
        prev_sibling_point: Option<LayoutPoint>,
        prev_sibling_size: Option<LayoutSize>,
    ) {
        let node = match node.as_ref() {
            Some(node) => node,
            None => return,
        };

        node.borrow_mut().compute_position(
            parent_point,
            prev_sibling_kind,
            prev_sibling_point,
            prev_sibling_size,
        );

        Self::calculate_node_position(
            &node.borrow().first_child(),
            node.borrow().point(),
            prev_sibling_kind,
            None,
            None,
        );

        Self::calculate_node_position(
            &node.borrow().next_sibling(),
            parent_point,
            node.borrow().kind(),
            Some(node.borrow().point()),
            Some(node.borrow().size()),
        );
    }

    fn update_layout(&mut self) {
        Self::calculat_node_size(&self.root, LayoutSize::new(CONTENT_AREA_WIDTH, 0));

        Self::calculate_node_position(
            &self.root,
            LayoutPoint::new(0, 0),
            LayoutObjectKind::Block,
            None,
            None,
        );
    }

    fn paint_node(node: &Option<Rc<RefCell<LayoutObject>>>, display_items: &mut Vec<DisplayItem>) {
        let node = match node.as_ref() {
            Some(node) => node,
            None => return,
        };

        display_items.extend(node.borrow_mut().paint());
        Self::paint_node(&node.borrow().first_child(), display_items);
        Self::paint_node(&node.borrow().next_sibling(), display_items);
    }

    pub fn paint(&self) -> Vec<DisplayItem> {
        let mut display_items = Vec::new();
        Self::paint_node(&self.root, &mut display_items);
        display_items
    }

    pub fn root(&self) -> Option<Rc<RefCell<LayoutObject>>> {
        self.root.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc::string::ToString;
    use crate::renderer::css::cssom::CssParser;
    use crate::renderer::css::token::CssTokenizer;
    use crate::renderer::dom::api::get_style_content;
    use crate::renderer::dom::node::Element;
    use crate::renderer::dom::node::NodeKind;
    use crate::renderer::html::parser::HtmlParser;
    use crate::renderer::html::token::HtmlTokenizer;
    use alloc::string::String;

    fn create_layout_view(html: String) -> LayoutView {
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let dom = window.borrow().document();
        let style = get_style_content(dom.clone());
        let css_tokenizer = CssTokenizer::new(style);
        let cssom = CssParser::new(css_tokenizer).parse_stylesheet();
        LayoutView::new(dom, &cssom)
    }

    #[test]
    fn test_empty() {
        let layout_view = create_layout_view("".to_string());
        assert_eq!(None, layout_view.root());
    }

    #[test]
    fn test_body() {
        let html = "<html><head></head><body></body></html>".to_string();
        let layout_view = create_layout_view(html);

        let root = layout_view.root();
        assert!(root.is_some());
        assert_eq!(
            LayoutObjectKind::Block,
            root.clone().expect("root should exist").borrow().kind()
        );
        assert_eq!(
            NodeKind::Element(Element::new("body", Vec::new())),
            root.clone()
                .expect("root should exist")
                .borrow()
                .node_kind()
        );
    }

    #[test]
    fn test_text() {
        let html = "<html><head></head><body>text</body></html>".to_string();
        let layout_view = create_layout_view(html);

        let root = layout_view.root();
        assert!(root.is_some());
        assert_eq!(
            LayoutObjectKind::Block,
            root.clone().expect("root should exist").borrow().kind()
        );
        assert_eq!(
            NodeKind::Element(Element::new("body", Vec::new())),
            root.clone()
                .expect("root should exist")
                .borrow()
                .node_kind()
        );

        let text = root.expect("root should exist").borrow().first_child();
        assert!(text.is_some());
        assert_eq!(
            NodeKind::Text("text".to_string()),
            text.clone()
                .expect("text node should exist")
                .borrow()
                .node_kind()
        );
        assert_eq!(
            LayoutObjectKind::Text,
            text.clone()
                .expect("text node should exist")
                .borrow()
                .kind()
        );
    }

    #[test]
    fn test_display_none() {
        let html = "<html><head><style>body{display:none;}</style></head><body>text</body></html>"
            .to_string();
        let layout_view = create_layout_view(html);

        assert_eq!(None, layout_view.root());
    }

    #[test]
    fn test_hidden_class() {
        let html = r#"<html>
    <head>
    <style>
      .hidden {
        display: none;
      }
    </style>
    </head>
    <body>
      <a class="hidden">link1</a>
      <p></p>
      <p class="hidden"><a>link2</a></p>
    </body>
    </html>"#
            .to_string();
        let layout_view = create_layout_view(html);

        let root = layout_view.root();
        assert!(root.is_some());
        assert_eq!(
            LayoutObjectKind::Block,
            root.clone().expect("root should exist").borrow().kind()
        );
        assert_eq!(
            NodeKind::Element(Element::new("body", Vec::new())),
            root.clone()
                .expect("root should exist")
                .borrow()
                .node_kind()
        );

        let p = root.expect("root should exist").borrow().first_child();
        assert!(p.is_some());
        assert_eq!(
            LayoutObjectKind::Block,
            p.clone().expect("p node should exist").borrow().kind()
        );
        assert_eq!(
            NodeKind::Element(Element::new("p", Vec::new())),
            p.clone().expect("p node should exist").borrow().node_kind()
        );

        assert!(p
            .clone()
            .expect("p node should exist")
            .borrow()
            .first_child()
            .is_none());
        assert!(p
            .expect("p node should exist")
            .borrow()
            .next_sibling()
            .is_none());
    }
}
