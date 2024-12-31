use core::cell::RefCell;

use alloc::{rc::Rc, vec::Vec};

use crate::renderer::page::Page;

#[derive(Debug, Clone)]
pub struct Browser {
    active_page_index: usize,
    pages: Vec<Rc<RefCell<Page>>>,
}

impl Browser {
    pub fn new() -> Rc<RefCell<Self>> {
        let mut page = Page::new();

        let browser = Rc::new(RefCell::new(Self {
            active_page_index: 0,
            pages: Vec::new(),
        }));

        page.set_browser(Rc::downgrade(&browser));
        browser.borrow_mut().pages.push(Rc::new(RefCell::new(page)));

        browser
    }

    pub fn current_page(&self) -> Rc<RefCell<Page>> {
        assert!(self.pages.len() > 0, "browser must have a page at least");
        self.pages[self.active_page_index].clone()
    }
}
