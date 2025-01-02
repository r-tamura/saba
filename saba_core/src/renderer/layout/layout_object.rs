use core::cell::RefCell;

use alloc::{
    rc::{Rc, Weak},
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    constants::{CHAR_HEIGHT_WITH_PADDING, CHAR_WIDTH, CONTENT_AREA_WIDTH},
    display_item::DisplayItem,
    renderer::{
        css::cssom::{ComponentValue, Declaration, Selector, StyleSheet},
        dom::node::{Node, NodeKind},
    },
};

use super::computed_style::{Color, ComputedStyle, DisplayType, FontSize};

/// https://drafts.csswg.org/css-text/#word-break-property
fn find_index_for_line_break(line: String, max_index: usize) -> usize {
    todo!();
}

/// https://drafts.csswg.org/css-text/#word-break-property
fn split_text(line: String, char_width: i64) -> Vec<String> {
    todo!();
}

pub fn create_layout_object(
    node: &Option<Rc<RefCell<Node>>>,
    parent: &Option<Rc<RefCell<LayoutObject>>>,
    cssom: &StyleSheet,
) -> Option<Rc<RefCell<LayoutObject>>> {
    let node = node.as_ref()?;
    let new_layout_object = Rc::new(RefCell::new(LayoutObject::new(node.clone(), parent)));

    for rule in &cssom.rules {
        if new_layout_object.borrow().is_node_selected(&rule.selector) {
            new_layout_object
                .borrow_mut()
                .cascading_style(rule.declarations.clone());
        }
    }

    // CSSスタイルが適用されていない場合、デフォルトの値または親ノードから継承した値を使用する
    let parent_style = parent.as_ref().map(|p| p.borrow().style());
    new_layout_object
        .borrow_mut()
        .defaulting_style(node, parent_style);

    // display: noneの場合
    if new_layout_object.borrow().style().display() == DisplayType::None {
        return None;
    }

    // displayプロパティの最終的な値を使用してノードの種類を決定
    new_layout_object.borrow_mut().update_kind();

    Some(new_layout_object)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutObjectKind {
    Block,
    Inline,
    Text,
    Unknown,
}
#[derive(Debug, Clone)]
pub struct LayoutObject {
    kind: LayoutObjectKind,
    node: Rc<RefCell<Node>>,
    first_child: Option<Rc<RefCell<LayoutObject>>>,
    next_sibling: Option<Rc<RefCell<LayoutObject>>>,
    parent: Weak<RefCell<LayoutObject>>,
    style: ComputedStyle,
    point: LayoutPoint,
    size: LayoutSize,
}

impl LayoutObject {
    fn new(node: Rc<RefCell<Node>>, parent: &Option<Rc<RefCell<LayoutObject>>>) -> Self {
        let parent = parent.as_ref().map_or(Weak::new(), |p| Rc::downgrade(&p));
        Self {
            kind: LayoutObjectKind::Block,
            node: node.clone(),
            first_child: None,
            next_sibling: None,
            parent,
            style: ComputedStyle::new(),
            point: LayoutPoint::new(0, 0),
            size: LayoutSize::new(0, 0),
        }
    }

    pub fn paint(&mut self) -> Vec<DisplayItem> {
        todo!();
    }

    pub fn compute_size(&mut self, parent_size: LayoutSize) {
        // 現状の実装では、CSSでwidth/heightを指定できないので、サイズは親ノード、子ノードのサイズで決まる
        let mut size = LayoutSize::new(0, 0);

        match self.kind() {
            LayoutObjectKind::Block => {
                size.set_width(parent_size.width());

                // 高さはすべての子ノードの高さを足し合わせたもの
                // インライン要素が横に並んでいる場合は
                let mut height = 0;
                let mut child = self.first_child();
                let mut prev_child_kind = LayoutObjectKind::Block;
                while child.is_some() {
                    let c = child.expect("first child should exist");
                    if prev_child_kind == LayoutObjectKind::Block
                        || c.borrow().kind() == LayoutObjectKind::Block
                    {
                        height += c.borrow().size.height();
                    }
                    prev_child_kind = c.borrow().kind();
                    child = c.borrow().next_sibling();
                }
                size.set_height(height);
            }
            LayoutObjectKind::Inline => {
                // すべての子ノードの高さと横幅を足し合わせたもの
                let mut width = 0;
                let mut height = 0;
                let mut child = self.first_child();
                while child.is_some() {
                    let c = child.expect("first child should exist");

                    width += c.borrow().size.width();
                    height += c.borrow().size.height();

                    child = c.borrow().next_sibling();
                }

                size.set_width(width);
                size.set_height(height);
            }
            LayoutObjectKind::Text => {
                let text = match self.node_kind() {
                    NodeKind::Text(text) => text,
                    _ => return,
                };
                let ratio = match self.style.font_size() {
                    FontSize::Medium => 1,
                    FontSize::XLarge => 2,
                    FontSize::XXLarge => 3,
                };
                let width = CHAR_WIDTH * ratio * text.len() as i64;
                if width > CONTENT_AREA_WIDTH {
                    // テキスト複数行
                    size.set_width(CONTENT_AREA_WIDTH);
                    let line_num = if width.wrapping_rem(CONTENT_AREA_WIDTH) == 0 {
                        width.wrapping_div(CONTENT_AREA_WIDTH)
                    } else {
                        width.wrapping_div(CONTENT_AREA_WIDTH) + 1
                    };
                    size.set_height(CHAR_HEIGHT_WITH_PADDING * ratio * line_num);
                } else {
                    // テキスト1行
                    size.set_width(width);
                    size.set_height(CHAR_HEIGHT_WITH_PADDING * ratio);
                }
            }
            LayoutObjectKind::Unknown => {}
        }

        self.size = size;
    }

    pub fn compute_position(
        &mut self,
        parent_point: LayoutPoint,
        prev_sibling_kind: LayoutObjectKind,
        prev_sibling_point: Option<LayoutPoint>,
        prev_sibling_size: Option<LayoutSize>,
    ) {
        let mut point = LayoutPoint::new(0, 0);

        match (self.kind(), prev_sibling_kind) {
            // 自ノードor兄弟ノードがブロック要素の場合
            (LayoutObjectKind::Block, _) | (_, LayoutObjectKind::Block) => {
                if let (Some(size), Some(pos)) = (prev_sibling_size, prev_sibling_point) {
                    point.set_y(pos.y() + size.height())
                } else {
                    point.set_y(parent_point.y());
                }
                point.set_x(parent_point.x());
            }
            // 自ノードと前兄弟ノードがインラインの場合
            (LayoutObjectKind::Inline, LayoutObjectKind::Inline) => {
                if let (Some(size), Some(pos)) = (prev_sibling_size, prev_sibling_point) {
                    point.set_x(pos.x() + size.width());
                    point.set_y(pos.y());
                } else {
                    point.set_x(parent_point.x());
                    point.set_y(parent_point.y());
                }
            }
            _ => {
                point.set_x(parent_point.x());
                point.set_y(parent_point.y());
            }
        }

        self.point = point;
    }

    pub fn is_node_selected(&self, selector: &Selector) -> bool {
        match &self.node_kind() {
            NodeKind::Element(element) => match selector {
                Selector::TypeSelector(type_name) => element.kind().to_string() == *type_name,
                Selector::ClassSelector(class_name) => element
                    .get_attr("class")
                    .map_or(false, |attr| attr.value() == *class_name),
                Selector::IdSelector(id_name) => element
                    .get_attr("id")
                    .map_or(false, |attr| attr.value() == *id_name),
                Selector::UnknownSelector => false,
            },
            _ => false,
        }
    }

    /// https://www.w3.org/TR/css-cascade-4/#cascading
    pub fn cascading_style(&mut self, declarations: Vec<Declaration>) {
        for declaration in declarations {
            match declaration.property.as_str() {
                "background-color" => match &declaration.value {
                    ComponentValue::Ident(value) => {
                        let color = Color::from_name(value).unwrap_or(Color::white());
                        self.style.set_background_color(color);
                    }
                    ComponentValue::HashToken(color_code) => {
                        let color = Color::from_code(color_code).unwrap_or(Color::white());
                        self.style.set_background_color(color);
                    }
                    _ => {}
                },
                "color" => match &declaration.value {
                    ComponentValue::Ident(value) => {
                        let color = Color::from_name(value).unwrap_or(Color::black());
                        self.style.set_color(color);
                    }
                    ComponentValue::HashToken(color_code) => {
                        let color = Color::from_code(color_code).unwrap_or(Color::black());
                        self.style.set_color(color);
                    }
                    _ => {}
                },
                "display" => {
                    if let ComponentValue::Ident(value) = declaration.value {
                        let display_type =
                            DisplayType::try_from(value.as_str()).unwrap_or(DisplayType::None);
                        self.style.set_display(display_type);
                    }
                }
                _ => {}
            }
        }
    }

    /// https://www.w3.org/TR/css-cascade-4/#defaulting-keywords
    pub fn defaulting_style(
        &mut self,
        node: &Rc<RefCell<Node>>,
        parent_style: Option<ComputedStyle>,
    ) {
        self.style.defauting(node, parent_style);
    }

    pub fn update_kind(&mut self) {
        match self.node_kind() {
            NodeKind::Document => panic!("should not create a layout object for a Document node"),
            NodeKind::Element(_) => {
                let display = self.style.display();
                match display {
                    DisplayType::Block => self.kind = LayoutObjectKind::Block,
                    DisplayType::Inline => self.kind = LayoutObjectKind::Inline,
                    DisplayType::None => {
                        panic!("should not create a layout object for display:none")
                    }
                }
            }
            NodeKind::Text(_) => self.kind = LayoutObjectKind::Text,
        }
    }

    pub fn kind(&self) -> LayoutObjectKind {
        self.kind
    }

    pub fn node_kind(&self) -> NodeKind {
        self.node.borrow().kind().clone()
    }

    pub fn set_first_child(&mut self, first_child: Option<Rc<RefCell<LayoutObject>>>) {
        self.first_child = first_child;
    }

    pub fn first_child(&self) -> Option<Rc<RefCell<LayoutObject>>> {
        self.first_child.clone()
    }

    pub fn set_next_sibling(&mut self, next_sibling: Option<Rc<RefCell<LayoutObject>>>) {
        self.next_sibling = next_sibling;
    }

    pub fn next_sibling(&self) -> Option<Rc<RefCell<LayoutObject>>> {
        self.next_sibling.as_ref().cloned()
    }

    pub fn parent(&self) -> Weak<RefCell<Self>> {
        self.parent.clone()
    }

    pub fn style(&self) -> ComputedStyle {
        self.style.clone()
    }

    pub fn point(&self) -> LayoutPoint {
        self.point
    }

    pub fn size(&self) -> LayoutSize {
        self.size
    }
}

impl PartialEq for LayoutObject {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct LayoutPoint(i64, i64);

impl LayoutPoint {
    pub fn new(x: i64, y: i64) -> Self {
        Self(x, y)
    }

    pub fn x(&self) -> i64 {
        self.0
    }

    pub fn y(&self) -> i64 {
        self.1
    }

    pub fn set_x(&mut self, x: i64) {
        self.0 = x;
    }

    pub fn set_y(&mut self, y: i64) {
        self.1 = y;
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct LayoutSize(i64, i64);

impl LayoutSize {
    pub fn new(width: i64, height: i64) -> Self {
        Self(width, height)
    }

    pub fn width(&self) -> i64 {
        self.0
    }

    pub fn height(&self) -> i64 {
        self.1
    }

    pub fn set_width(&mut self, width: i64) {
        self.0 = width;
    }

    pub fn set_height(&mut self, height: i64) {
        self.1 = height;
    }
}
