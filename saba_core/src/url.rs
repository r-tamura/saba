use alloc::string::{String, ToString};

#[derive(Debug, Clone, PartialEq)]
struct Url {
    url: String,
    host: String,
    port: String,
    path: String,
    searchpart: String,
}

impl Url {
    pub fn new(url: String) -> Self {
        return Self {
            url,
            host: "".to_string(),
            port: "".to_string(),
            path: "".to_string(),
            searchpart: "".to_string(),
        };
    }
}
