#![allow(dead_code,unused_variables,unused_imports)]

extern crate crust;
use super::woot_char::WootChar;
use self::crust::PeerId;

#[derive(Clone,PartialEq,Debug,RustcDecodable,RustcEncodable)]
pub enum Operation {
    Insert {w_char: WootChar, from_site: PeerId},
    Delete {w_char: WootChar, from_site: PeerId}
}
unsafe impl Send for Operation {}
unsafe impl Sync for Operation {}
