use network::Message;
use async_queue::AsyncQueue;
use std::sync::Arc;
use std::collections::VecDeque;

pub struct Logger<T:Message> {
    pub messages: Arc<AsyncQueue<T>>
}

impl<T:Message> Logger<T> {
    pub fn init() -> Logger<T> {
        Logger {
            messages: Arc::new(AsyncQueue::new())
        }
    }

    pub fn reception(&self, message: T) {
        let logger_queue = self.messages.clone();
        logger_queue.enq(message);
    }

    pub fn print(&self) {
        let logger_queue = self.messages.clone();
        let q = logger_queue.queue.lock().unwrap();
        for c in q.iter() {
            println!("{:?}", c);
        }
    }
}
