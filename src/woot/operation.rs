#![allow(dead_code)]
use super::woot_char::WootChar;

#[derive(Clone,PartialEq,Debug)]
pub enum Operation {
    Insert {w_char: WootChar, from_site: u32},
    Delete {w_char: WootChar, from_site: u32}
}
