use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use woot::operation::Operation;
use std::sync::Arc;
use std::{thread,env};
use woot::char_id::create_char_id;
use woot::char_id::CharId;
use woot::woot_char::WootChar;

pub struct Pool {
    operations: VecDeque<Operation>,
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl Pool {
    pub fn new() -> Pool {
        Pool { operations: VecDeque::new(), head: AtomicUsize::new(0), tail: AtomicUsize::new(0) }
    }

    pub fn try_pop(&mut self) -> Option<Operation> {
        let current_head = self.head.load(Ordering::Acquire);

        return if current_head == self.tail.load(Ordering::Acquire) {
            None
        } else {
            let op = self.operations.pop_front();
            self.head.fetch_add(1, Ordering::Release);
            op
        }
    }

    pub fn push(&mut self, op: Operation) {
        let current_tail = self.tail.load(Ordering::Acquire);
        self.operations.push_back(op);
        self.tail.fetch_add(1, Ordering::Release);
    }

    pub fn pop(&mut self) -> Operation {
        loop {
            match self.try_pop()  {
                None => {},
                Some(v) => return v
            }
        }
    }
}

pub struct Consumer {
    buffer: Arc<Pool>,
}

/// A handle to the queue which allows adding values onto the buffer
pub struct Producer {
    buffer: Arc<Pool>,
}

impl Producer {
    pub fn push(&mut self, op: Operation) {
        match Arc::get_mut(&mut self.buffer) {
            Some(pool) => {
                pool.push(op);
            },
            None => {}
        }
    }

    pub fn size(&self) -> usize {
        (*self.buffer).tail.load(Ordering::Acquire) - (*self.buffer).head.load(Ordering::Acquire)
    }
}

impl Consumer {
    pub fn pop(&mut self) -> Option<Operation> {
        return match Arc::get_mut(&mut self.buffer) {
            Some(pool) => {
                Some(pool.pop())
            },
            None => None
        }
    }

    pub fn try_pop(&mut self) -> Option<Operation> {
        return match Arc::get_mut(&mut self.buffer) {
            Some(pool) => {
                pool.try_pop()
            },
            None => None
        }
    }

    pub fn size(&self) -> usize {
        (*self.buffer).tail.load(Ordering::Acquire) - (*self.buffer).head.load(Ordering::Acquire)
    }
}

pub fn make_pool() -> (Producer, Consumer) {
    let arc = Arc::new(Pool::new());
    (Producer { buffer: arc.clone() }, Consumer { buffer: arc.clone() })
}

// #[test]
fn test_threaded() {
    let (mut p, mut c) = make_pool();
    let char_id_1 = create_char_id(1, 0);
    // let char_id_2 = create_char_id(2, 0);
    let mut wchar1 = WootChar::new(char_id_1.clone(), 'a', CharId::Beginning, CharId::Ending); // From site 1
    // let mut wchar2 = WootChar::new(char_id_2.clone(), 'b', CharId::Beginning, CharId::Ending); // From site 2
    let operation = Operation::Insert{w_char: wchar1.clone(), from_site: 1};
    let operation_clone = operation.clone();
    thread::spawn(move|| {
        // let op = c.pop();
        // match op {
        //     Some(v) => {
        //         assert_eq!(v, operation);
        //         println!("Found something!");
        //     },
        //     None => println!("Uh oh")
        // };
    });

    // let interval = Duration::from_millis(1000);
    // // Create a timer object
    // sleep(interval);
    println!("Pushing now");
    p.push(operation_clone);
    assert_eq!(p.size(), 1);
}
