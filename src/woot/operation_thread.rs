#![allow(dead_code,unused_variables,unused_imports, unused_mut)]

use rustc_serialize::json;
use super::char_id::create_char_id;
use super::char_id::CharId;
use super::woot_char::WootChar;
use super::static_site::StaticSite;
use super::operation::Operation;
use std::{thread,env};
use std::thread::sleep;
use std::time::Duration;
use std::sync::mpsc::channel;
use super::crust::PeerId;

pub fn run(static_site: StaticSite) {

    let site = static_site.inner.clone();
    let site2 = static_site.inner.clone();

    // Implementing Multiple Producer, Single Consumer
    let (sender, receiver) = channel();
    thread::spawn(move || {
        loop {
            let mut site = site.lock().unwrap();
            // Receive operation
            match site.pool.pop_front() {
                Some(v) => {
                    println!("Found something!");
                    sender.send(v).unwrap();
                },
                None => {}
            }
        }
    });

    loop {
        let op = receiver.recv().unwrap();
        let mut site2 = site2.lock().unwrap();
        site2.implement_operation(op);
        println!("Implemented something");
    }
}
