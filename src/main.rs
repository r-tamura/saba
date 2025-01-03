#![no_std]
#![no_main]

extern crate alloc;

use core::cell::RefCell;

use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use net_wasabi::http::HttpClient;
use noli::*;
use saba_core::browser::Browser;
use saba_core::error::Error;
use saba_core::http::HttpResponse;
use saba_core::url::Url;
use ui_wasabi::app::WasabiUI;

fn handle_url(url: String) -> Result<HttpResponse, Error> {
    println!("fetch {url}");
    let url = Url::new(url)
        .parse()
        .map_err(|e| Error::UnexpectedInput(format!("input html is not supported: {:?}", e)))?;

    let client = HttpClient::new();
    client
        .get(
            url.host(),
            url.port()
                .parse::<u16>()
                .expect(&format!("port number should be u16 but got {}", url.port(),)),
            url.path(),
        )
        .map_err(|e| Error::Network(format!("failed to get http response: {:?}", e)))
        .map(|res| {
            println!("{:?}", res);
            res
        })
}

fn main() -> u64 {
    let browser = Browser::new();

    let ui = Rc::new(RefCell::new(WasabiUI::new(browser)));

    let result = match ui.borrow_mut().start(handle_url) {
        Err(e) => {
            println!("browser fails to start {:?}", e);
            1
        }
        Ok(_) => 0,
    };
    result
}

entry_point!(main);
