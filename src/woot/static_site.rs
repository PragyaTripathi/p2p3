#![allow(dead_code,unused_variables,unused_imports)]

use std::sync::{Arc,Mutex};
use super::site::{Site, UISend};
use network::{MessagePasser, MessagePasserT};
use msg::Msg;

#[derive(Clone)]
pub struct StaticSite {
    pub inner: Arc<Mutex<Site>>
}

impl StaticSite {
    pub fn new(mp: MessagePasser<Msg>, ui_send: Arc<UISend>) -> StaticSite {
        StaticSite {
            inner: Arc::new((Mutex::new(Site::new(mp.get_id().clone(), mp, ui_send))))
        }
    }
}
