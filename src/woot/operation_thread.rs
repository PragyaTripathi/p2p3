#![allow(dead_code)]

use std::thread;
use super::static_site::StaticSite;
use std::sync::mpsc::channel;

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
