#![allow(dead_code,unused_variables,unused_imports,unused_must_use)]

extern crate crust;
extern crate rand;
use self::crust::PeerId;
use self::rand::random;
use network::{Message,MsgKind};
use async_queue::AsyncQueue;
use std::sync::Arc;
use std::collections::VecDeque;

pub struct Logger {
    messages: Arc<AsyncQueue<Message>>
}

impl Logger {
    pub fn init() -> Logger {
        Logger {
            messages: Arc::new(AsyncQueue::new())
        }
    }

    pub fn reception(&self, message: Message) {
        let logger_queue = self.messages.clone();
        logger_queue.enq(message);
    }

    pub fn print(&self) {
        let logger_queue = self.messages.clone();
        let q = logger_queue.queue.lock().unwrap();
        for c in q.iter() {
            println!("{}: From: {} Content: {} Kind: {:?}", c.seq_num, c.source, c.message, c.kind);
        }
    }
}

#[cfg(test)]
mod test{
    use super::*;


}
// #[test]
// fn test_logger() {
//     let logger = Logger::init();
//     let peer_id: PeerId = random();
//     let message1 = Message {
//         source: peer_id.clone(),
//         message: "Woot Operation to insert 'a'".to_string(),
//         kind: MsgKind::Broadcast,
//         seq_num: 0
//     };
//     logger.reception(message1);
//
//     let message2 = Message {
//         source: peer_id.clone(),
//         message: "Woot Operation to insert 'b'".to_string(),
//         kind: MsgKind::Broadcast,
//         seq_num: 0
//     };
//     logger.reception(message1);
//     logger.reception(message2);
//     {
//         let logger_queue = logger.messages.clone();
//         let q = logger_queue.queue.lock().unwrap();
//         assert_eq!(q.len(), 2);
//     }
//     logger.print();
// }
