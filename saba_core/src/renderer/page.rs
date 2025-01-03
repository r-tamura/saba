use core::cell::RefCell;

use alloc::{
    format,
    rc::{Rc, Weak},
    string::String,
    vec,
    vec::Vec,
};

use crate::{
    browser::Browser, display_item::DisplayItem, http::HttpResponse, utils::convert_dom_to_string,
};

use super::{
    css::{
        cssom::{CssParser, StyleSheet},
        token::CssTokenizer,
    },
    dom::{
        api::get_style_content,
        node::{ElementKind, NodeKind, Window},
    },
    html::{parser::HtmlParser, token::HtmlTokenizer},
    layout::{layout_object::LayoutObjectKind, layout_view::LayoutView},
};

#[derive(Debug, Clone)]
pub struct Page {
    browser: Weak<RefCell<Browser>>,
    frame: Option<Rc<RefCell<Window>>>,
    style: Option<StyleSheet>,
    layout_view: Option<LayoutView>,
    display_items: Vec<DisplayItem>,
}

impl Page {
    pub fn new() -> Self {
        Self {
            browser: Weak::new(),
            frame: None,
            style: None,
            layout_view: None,
            display_items: vec![],
        }
    }

    pub fn set_browser(&mut self, browser: Weak<RefCell<Browser>>) {
        self.browser = browser;
    }

    pub fn receive_response(&mut self, response: HttpResponse) {
        self.create_frame(response.body());
        self.set_layout_view();
        self.paint_tree();
    }

    fn create_frame(&mut self, html: String) {
        let html_tokenizer = HtmlTokenizer::new(html);
        let frame = HtmlParser::new(html_tokenizer).construct_tree();

        let dom = frame.borrow().document();
        let style = get_style_content(dom);
        let css_tokenizer = CssTokenizer::new(style);
        let cssom = CssParser::new(css_tokenizer).parse_stylesheet();

        self.frame = Some(frame);
        self.style = Some(cssom);
    }

    fn set_layout_view(&mut self) {
        let dom = match self.frame.as_ref() {
            Some(frame) => frame.borrow().document(),
            None => return,
        };
        let style = match self.style.clone() {
            Some(style) => style,
            None => return,
        };

        let layout_view = LayoutView::new(dom, &style);
        self.layout_view = Some(layout_view);
    }

    fn paint_tree(&mut self) {
        if let Some(layout_view) = &self.layout_view {
            self.display_items = layout_view.paint();
        }
    }

    /// 指定された位置に<a>タグが存在するとき、その<a>タグのリンクを返します
    pub fn get_link_at(&self, position: (i64, i64)) -> Option<String> {
        let view = self.layout_view.as_ref()?;
        let node = view.find_node_by_position(position)?;
        // aタグの子ノードが返されるのでparentでaタグを取得
        let parent = node.borrow().parent().upgrade()?;
        let link = if let NodeKind::Element(element) = parent.borrow().node_kind() {
            match element.kind() {
                ElementKind::A => element.get_attr("href").map(|attr| attr.value()),
                _ => None,
            }
        } else {
            None
        };
        link
    }

    pub fn display_items(&self) -> Vec<DisplayItem> {
        self.display_items.clone()
    }

    pub fn clear_display_items(&mut self) {
        self.display_items = vec![];
    }
}
