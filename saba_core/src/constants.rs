pub const WHITE: u32 = 0xffffff;
pub const LIGHTGREY: u32 = 0xd3d3d3;
pub const GREY: u32 = 0x808080;
pub const DARKGREY: u32 = 0x5a5a5a;
pub const BLACK: u32 = 0x000000;

pub const ADDRESSBAR_HEIGHT: i64 = 20;

pub const WINDOW_INIT_X_POS: i64 = 30;
pub const WINDOW_INIT_Y_POS: i64 = 50;

pub const WINDOW_WIDTH: i64 = 600;
pub const WINDOW_HEIGHT: i64 = 400;
pub const WINDOW_PADDING: i64 = 5;

// noliライブラリに定義されている定数
pub const TITLE_BAR_HEIGHT: i64 = 24;

pub const TOOLBAR_HEIGHT: i64 = 26;

pub const CONTENT_AREA_WIDTH: i64 = WINDOW_WIDTH - WINDOW_PADDING * 2;
pub const CONTENT_AREA_HEIGHT: i64 =
    WINDOW_HEIGHT - TITLE_BAR_HEIGHT - TOOLBAR_HEIGHT - WINDOW_PADDING * 2;

pub const CHAR_WIDTH: i64 = 8;
pub const CHAR_HEIGHT: i64 = 16;
pub const CHAR_HEIGHT_WITH_PADDING: i64 = CHAR_HEIGHT + 4;
