use alloc::string::{String, ToString};

use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    name: String,
    value: String,
}

impl Attribute {
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }

    pub fn add_char(&mut self, c: char, is_name: bool) {
        if is_name {
            self.name.push(c);
        } else {
            self.value.push(c);
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn value(&self) -> String {
        self.value.clone()
    }
}

impl Default for Attribute {
    fn default() -> Self {
        Self {
            name: String::new(),
            value: String::new(),
        }
    }
}

impl TryFrom<&str> for Attribute {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.split_once("=") {
            // name="value"
            Some((name, value)) => Ok(Self::new(name.to_string(), value.to_string())),
            // name (disabledなど)
            None => Ok(Self::new(value.to_string(), "true".to_string())),
        }
    }
}
