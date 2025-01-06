use core::cell::RefCell;

use alloc::{
    rc::{Rc, Weak},
    string::String,
    vec,
    vec::Vec,
};

use crate::{browser::Browser, display_item::DisplayItem, http::HttpResponse};

use super::{
    css::{
        cssom::{CssParser, StyleSheet},
        token::CssTokenizer,
    },
    dom::{
        api::{get_js_content, get_style_content},
        node::{Element, ElementKind, NodeKind, Window},
    },
    html::{parser::HtmlParser, token::HtmlTokenizer},
    js::{
        ast::JsParser,
        runtime::{JsRuntimeBuilder, PostMessageToHost},
        token::JsLexer,
    },
    layout::layout_view::LayoutView,
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
        self.execute_script();
        self.render();
    }

    fn execute_script(&mut self) {
        let dom = match &self.frame {
            Some(frame) => frame.borrow().document(),
            None => return,
        };

        self.execute_js_code(None, get_js_content(dom));
    }

    pub fn execute_js_code(&mut self, post_message: Option<PostMessageToHost>, code: String) {
        let dom = match &self.frame {
            Some(frame) => frame.borrow().document(),
            None => panic!("frame is not created yet"),
        };

        let lexer = JsLexer::new(code);
        let mut parser = JsParser::new(lexer);
        let ast = parser.parse_ast();
        let mut runtime = JsRuntimeBuilder::new(dom)
            .post_message(post_message)
            .build();
        runtime.execute(&ast);
    }

    pub fn render(&mut self) {
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
        let element = self.get_deepest_element_by_position(position)?;
        match element.kind() {
            ElementKind::A => element.get_attr("href").map(|attr| attr.value()),
            _ => None,
        }
    }

    pub fn get_attribute(&self, position: (i64, i64), attr_name: &str) -> Option<String> {
        let element = self.get_deepest_element_by_position(position)?;
        element.get_attr(attr_name).map(|attr| attr.value())
    }

    pub fn get_deepest_element_by_position(&self, position: (i64, i64)) -> Option<Element> {
        let node = self.layout_view.as_ref()?.find_node_by_position(position)?;
        let element = match node.borrow().node_kind() {
            NodeKind::Element(element) => Some(element),
            NodeKind::Text(_) => match node.borrow().parent().upgrade()?.borrow().node_kind() {
                NodeKind::Element(element) => Some(element),
                _ => None,
            },
            _ => None,
        };
        element
    }
    pub fn display_items(&self) -> Vec<DisplayItem> {
        self.display_items.clone()
    }

    pub fn clear_display_items(&mut self) {
        self.display_items = vec![];
    }
}
