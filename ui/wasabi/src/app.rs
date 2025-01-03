use core::cell::RefCell;

use alloc::{
    format,
    rc::Rc,
    string::{String, ToString},
};
use noli::{error::Result as OsResult, prelude::MouseEvent, println, rect::Rect};
use noli::{
    prelude::{Api, SystemApi},
    window::{StringSize, Window},
};
use saba_core::{
    browser::Browser,
    constants::{
        ADDRESSBAR_HEIGHT, BLACK, CONTENT_AREA_HEIGHT, CONTENT_AREA_WIDTH, DARKGREY, GREY,
        LIGHTGREY, TITLE_BAR_HEIGHT, TOOLBAR_HEIGHT, WHITE, WINDOW_HEIGHT, WINDOW_INIT_X_POS,
        WINDOW_INIT_Y_POS, WINDOW_PADDING, WINDOW_WIDTH,
    },
    display_item::DisplayItem,
    error::Error,
    http::HttpResponse,
    renderer::layout::computed_style::{FontSize, TextDecoration},
};

use crate::cursor::Cursor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug)]
pub struct WasabiUI {
    browser: Rc<RefCell<Browser>>,
    input_url: String,
    input_mode: InputMode,
    window: Window,
    cursor: Cursor,
}

impl WasabiUI {
    pub fn new(browser: Rc<RefCell<Browser>>) -> Self {
        Self {
            browser,
            input_url: String::new(),
            input_mode: InputMode::Normal,
            window: Window::new(
                "saba".to_string(),
                WHITE,
                WINDOW_INIT_X_POS,
                WINDOW_INIT_Y_POS,
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
            )
            .unwrap(),
            cursor: Cursor::new(),
        }
    }

    fn start_editing(&mut self) {
        self.input_url = String::new();
        self.input_mode = InputMode::Editing;
    }

    fn end_editing(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn start(
        &mut self,
        handle_url: fn(String) -> Result<HttpResponse, Error>,
    ) -> Result<(), Error> {
        self.setup()?;

        self.run_app(handle_url)?;

        Ok(())
    }

    fn run_app(
        &mut self,
        handle_url: fn(String) -> Result<HttpResponse, Error>,
    ) -> Result<(), Error> {
        loop {
            self.handle_mouse_input(handle_url)?;
            self.handle_key_input(handle_url)?;
        }
    }

    fn handle_mouse_input(
        &mut self,
        handle_url: fn(String) -> Result<HttpResponse, Error>,
    ) -> Result<(), Error> {
        let (button, position) = match Api::get_mouse_cursor_info() {
            Some(MouseEvent { button, position }) => (button, position),
            _ => return Ok(()),
        };

        self.window.flush_area(self.cursor.rect());
        self.cursor.set_position(position.x, position.y);
        self.window.flush_area(self.cursor.rect());
        self.cursor.flush();

        if !(button.l() || button.c() || button.r()) {
            return Ok(());
        }

        let relative_pos = (
            position.x - WINDOW_INIT_X_POS,
            position.y - WINDOW_INIT_Y_POS,
        );

        fn in_window((x, y): (i64, i64)) -> bool {
            0 <= x && x < WINDOW_WIDTH && 0 <= y && y < WINDOW_HEIGHT
        }
        if !in_window(relative_pos) {
            println!("button clicked OUTSIDE window: {button:?} {position:?}");
            return Ok(());
        }

        fn in_toolbar((_x, y): (i64, i64)) -> bool {
            TITLE_BAR_HEIGHT <= y && y < TOOLBAR_HEIGHT + TITLE_BAR_HEIGHT
        }
        if in_toolbar(relative_pos) {
            self.clear_address_bar()?;
            self.start_editing();
            println!("button clicked in toolbar: {button:?} {position:?}");
            return Ok(());
        }

        self.end_editing();

        let position_in_content_area = (
            relative_pos.0,
            relative_pos.1 - TITLE_BAR_HEIGHT - TOOLBAR_HEIGHT,
        );
        let next_destination = self
            .browser
            .borrow()
            .current_page()
            .borrow_mut()
            .get_link_at(position_in_content_area);
        if let Some(url) = next_destination {
            self.input_url = url.clone();
            self.update_address_bar()?;
            self.start_navigation(handle_url, url)?;
        }

        Ok(())
    }

    fn handle_key_input(
        &mut self,
        handle_url: fn(String) -> Result<HttpResponse, Error>,
    ) -> Result<(), Error> {
        match self.input_mode {
            InputMode::Normal => {
                // キー入力を無視
                let _ = Api::read_key();
            }
            InputMode::Editing => {
                if let Some(c) = Api::read_key() {
                    let code = c as u8;
                    match code {
                        0x0A => {
                            // ENTER
                            self.start_navigation(handle_url, self.input_url.clone())?;
                            self.input_url = String::new();
                            self.input_mode = InputMode::Normal;
                        }
                        0x7F | 0x08 => {
                            // DELETE or BACKSPACE
                            self.input_url.pop();
                            self.update_address_bar()?;
                        }
                        _ => {
                            self.input_url.push(c);
                            self.update_address_bar()?;
                        }
                    };
                }
            }
        }

        Ok(())
    }

    fn start_navigation(
        &mut self,
        handle_url: fn(String) -> Result<HttpResponse, Error>,
        destination: String,
    ) -> Result<(), Error> {
        self.clear_content_area()?;

        handle_url(destination).map(|response| {
            self.browser
                .borrow()
                .current_page()
                .borrow_mut()
                .receive_response(response);
        })?;

        self.update_ui()?;

        Ok(())
    }

    fn update_ui(&mut self) -> Result<(), Error> {
        let display_items = self
            .browser
            .borrow()
            .current_page()
            .borrow()
            .display_items();
        for item in display_items {
            match item {
                DisplayItem::Text {
                    text,
                    style,
                    layout_point,
                } => self
                    .window
                    .draw_string(
                        style.color().code_u32(),
                        layout_point.x() + WINDOW_PADDING,
                        layout_point.y() + WINDOW_PADDING + TOOLBAR_HEIGHT,
                        &text,
                        convert_font_size(style.font_size()),
                        style.text_decoration() == TextDecoration::Underline,
                    )
                    .map_err(|_| Error::InvalidUI("failed to draw a string".to_string())),
                DisplayItem::Rect {
                    style,
                    layout_point,
                    layout_size,
                } => self
                    .window
                    .fill_rect(
                        style.background_color().code_u32(),
                        layout_point.x() + WINDOW_PADDING,
                        layout_point.y() + WINDOW_PADDING + TOOLBAR_HEIGHT,
                        layout_size.width(),
                        layout_size.height(),
                    )
                    .map_err(|_| Error::InvalidUI("failed to draw a string".to_string())),
            }?
        }

        self.window.flush();

        Ok(())
    }

    fn setup(&mut self) -> Result<(), Error> {
        self.setup_toolbar().map_err(|error| {
            Error::InvalidUI(format!(
                "failed to initialize a toolbar with error: {:#?}",
                error
            ))
        })?;
        self.window.flush();
        Ok(())
    }

    fn setup_toolbar(&mut self) -> OsResult<()> {
        // ツールバーの四角
        self.window
            .fill_rect(LIGHTGREY, 0, 0, WINDOW_WIDTH, TOOLBAR_HEIGHT)?;

        // ツールバーコンテンツエリア
        self.window
            .draw_line(GREY, 0, TOOLBAR_HEIGHT, WINDOW_WIDTH - 1, TOOLBAR_HEIGHT)?;
        self.window.draw_line(
            DARKGREY,
            0,
            TOOLBAR_HEIGHT + 1,
            WINDOW_WIDTH - 1,
            TOOLBAR_HEIGHT + 1,
        )?;

        // アドレスバー
        self.window
            .draw_string(BLACK, 5, 5, "Address:", StringSize::Medium, false)?;
        self.window
            .fill_rect(WHITE, 70, 2, WINDOW_WIDTH - 74, 2 + ADDRESSBAR_HEIGHT)?;
        self.window.draw_line(GREY, 70, 2, WINDOW_WIDTH - 4, 2)?;
        self.window.draw_line(BLACK, 71, 3, WINDOW_WIDTH - 5, 3)?;
        self.window
            .draw_line(GREY, 71, 3, 71, 1 + ADDRESSBAR_HEIGHT)?;

        Ok(())
    }

    fn reset_address_bar(&mut self) -> Result<(), Error> {
        self.window
            .fill_rect(WHITE, 72, 4, WINDOW_WIDTH - 76, ADDRESSBAR_HEIGHT - 2)
            .map_err(|_| Error::InvalidUI("failed to clear an address bar".to_string()))
    }

    fn flush_address_bar(&mut self) {
        self.window.flush_area(
            Rect::new(
                WINDOW_INIT_X_POS,
                WINDOW_INIT_Y_POS + TITLE_BAR_HEIGHT,
                WINDOW_WIDTH,
                TOOLBAR_HEIGHT,
            )
            .expect("failed to create a rect for the address bar"),
        );
    }

    fn update_address_bar(&mut self) -> Result<(), Error> {
        self.reset_address_bar()?;
        self.window
            .draw_string(BLACK, 74, 6, &self.input_url, StringSize::Medium, false)
            .map_err(|_| Error::InvalidUI("failed to update an address bar".to_string()))?;
        self.flush_address_bar();
        Ok(())
    }

    /// アドレスバーの内容をクリアします
    fn clear_address_bar(&mut self) -> Result<(), Error> {
        self.reset_address_bar()?;
        self.flush_address_bar();
        Ok(())
    }

    fn clear_content_area(&mut self) -> Result<(), Error> {
        self.window
            .fill_rect(
                WHITE,
                0,
                TOOLBAR_HEIGHT + 2,
                CONTENT_AREA_WIDTH,
                CONTENT_AREA_HEIGHT - 2,
            )
            .map_err(|_| Error::InvalidUI("failed to clear a content area".to_string()))?;

        self.window.flush();

        Ok(())
    }
}

fn convert_font_size(size: FontSize) -> StringSize {
    match size {
        FontSize::Medium => StringSize::Medium,
        FontSize::XLarge => StringSize::Large,
        FontSize::XXLarge => StringSize::XLarge,
    }
}
