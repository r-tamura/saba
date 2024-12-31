use core::cell::RefCell;

use alloc::{
    rc::{Rc, Weak},
    string::{String, ToString},
};

use crate::{browser::Browser, http::HttpResponse, utils::convert_dom_to_string};

use super::{
    dom::node::Window,
    html::{parser::HtmlParser, token::HtmlTokenizer},
};

#[derive(Debug, Clone)]
pub struct Page {
    browser: Weak<RefCell<Browser>>,
    frame: Option<Rc<RefCell<Window>>>,
}

impl Page {
    pub fn new() -> Self {
        Self {
            browser: Weak::new(),
            frame: None,
        }
    }

    pub fn set_browser(&mut self, browser: Weak<RefCell<Browser>>) {
        self.browser = browser;
    }

    pub fn receive_response(&mut self, response: HttpResponse) -> String {
        self.create_frame(response.body());

        // デバッグ用
        match &self.frame {
            Some(frame) => {
                let dom = frame.borrow().document().clone();
                let debug = convert_dom_to_string(&Some(dom));
                debug
            }
            None => "".to_string(),
        }
    }

    fn create_frame(&mut self, html: String) {
        let html_tokenizer = HtmlTokenizer::new(html);
        let frame = HtmlParser::new(html_tokenizer).construct_tree();
        self.frame = Some(frame);
    }
}
