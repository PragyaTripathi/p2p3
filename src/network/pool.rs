use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use woot::operation::Operation;
use std::sync::{Arc,Mutex};
use std::{thread,env};
use woot::char_id::create_char_id;
use woot::char_id::CharId;
use woot::woot_char::WootChar;

pub struct Pool {
    operations: VecDeque<Operation>
}

unsafe impl Send for Pool {}
unsafe impl Sync for Pool {}

impl Pool {
    pub fn new() -> Pool {
        Pool { operations: VecDeque::new() }
    }

    pub fn size(&self) -> usize {
        self.operations.len()
    }

    pub fn try_pop(&mut self) -> Option<Operation> {
        let current_head = AtomicUsize::new(0);
        let current_tail = AtomicUsize::new(self.operations.len());
        return if current_head.load(Ordering::Acquire) == current_tail.load(Ordering::Acquire) {
            None
        } else {
            let op = self.operations.pop_front();
            current_head.fetch_add(1, Ordering::Release);
            op
        }
    }

    pub fn push(&mut self, op: Operation) {
        let current_tail = AtomicUsize::new(self.operations.len());
        current_tail.load(Ordering::Acquire);
        self.operations.push_back(op);
        current_tail.fetch_add(1, Ordering::Release);
    }

    pub fn pop(&mut self) -> Operation {
        loop {
            match self.try_pop()  {
                None => {
                    // println!("Nothing found!");
                },
                Some(v) => return v
            }
            // println!("Waiting");
        }
    }
}

pub struct Consumer {
    buffer: Arc<Pool>,
}

// /// A handle to the queue which allows adding values onto the buffer
// pub struct Producer {
//     buffer: Arc<Pool>,
// }
//
// impl Producer {
//     pub fn push(&mut self, op: Operation) {
//         (*Arc::make_mut(&mut self.buffer)).push(op);
//     }
//
//     pub fn size(&self) -> usize {
//         (*self.buffer).size()
//     }
// }
//
// impl Consumer {
//     pub fn pop(&mut self) -> Operation {
//         (*Arc::make_mut(&mut self.buffer)).pop()
//     }
//
//     pub fn try_pop(&mut self) -> Option<Operation> {
//         (*Arc::make_mut(&mut self.buffer)).try_pop()
//     }
//
//     pub fn size(&self) -> usize {
//         (*self.buffer).size()
//     }
// }

// pub fn make_pool() -> (Producer, Consumer) {
//     let arc = Arc::new(Pool::new());
//     (Producer { buffer: arc.clone() }, Consumer { buffer: arc.clone() })
// }

#[test]
fn test_arc() {
    let arc = Arc::new(Mutex::new(Pool::new()));
    let mut producer = arc.clone();
    let mut consumer = arc.clone();
    let char_id_1 = create_char_id(1, 0);
    let mut wchar1 = WootChar::new(char_id_1.clone(), 'a', CharId::Beginning, CharId::Ending);
    let operation = Operation::Insert{w_char: wchar1.clone(), from_site: 1};
    let operation_clone = operation.clone();
    thread::spawn(move|| {
        println!("In the thread");
        let mut local_consumer = consumer.lock().unwrap();
        let op = local_consumer.pop();
        println!("Found something!");
        assert_eq!(op, operation);
    });
    println!("Pushing now");
    let mut local_producer = producer.lock().unwrap();
    local_producer.push(operation_clone);
    // assert_eq!(local_producer.size(), 1);
}

// fn test_threaded() {
//     let (mut p, mut c) = make_pool();
//     let char_id_1 = create_char_id(1, 0);
//     // let char_id_2 = create_char_id(2, 0);
//     let mut wchar1 = WootChar::new(char_id_1.clone(), 'a', CharId::Beginning, CharId::Ending); // From site 1
//     // let mut wchar2 = WootChar::new(char_id_2.clone(), 'b', CharId::Beginning, CharId::Ending); // From site 2
//     let operation = Operation::Insert{w_char: wchar1.clone(), from_site: 1};
//     let operation_clone = operation.clone();
//     thread::spawn(move|| {
//         println!("In the thread");
//         let op = c.pop();
//         println!("Found something!");
//         assert_eq!(op, operation);
//         // match op {
//         //     Some(v) => {
//         //         assert_eq!(v, operation);
//         //         println!("Found something!");
//         //     },
//         //     None => println!("Uh oh")
//         // };
//     });
//
//     // let interval = Duration::from_millis(1000);
//     // // Create a timer object
//     // sleep(interval);
//     println!("Pushing now");
//     p.push(operation_clone);
//     assert_eq!(p.size(), 1);
// }
