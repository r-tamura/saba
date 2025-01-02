use alloc::string::String;

use crate::renderer::layout::{
    computed_style::ComputedStyle,
    layout_object::{LayoutPoint, LayoutSize},
};

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayItem {
    Rect {
        style: ComputedStyle,
        layout_point: LayoutPoint,
        layout_size: LayoutSize,
    },
    Text {
        text: String,
        style: ComputedStyle,
        layout_point: LayoutPoint,
    },
}
