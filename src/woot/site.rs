#![allow(dead_code)]
use rustc_serialize::json;
use std::collections::VecDeque;
use super::clock::Clock;
use super::sequence::Sequence;
use super::operation::Operation;
use super::woot_char::WootChar;
use super::char_id::CharId;
use super::char_id::create_char_id;
use network::MessagePasserT;
use crust::PeerId;
use ui::Command;
use msg::Msg;
use std::sync::{Arc, Mutex};

pub type UISend = Box<Fn(Command) + Send + Sync>;

#[derive(Clone)]
pub struct Site {
    site_id: PeerId,
    logical_clock: Clock,
    sequence: Sequence,
    pub pool: VecDeque<Operation>,
    message_passer: Arc<Mutex<Box<MessagePasserT<Msg>>>>,
    ui_send: Arc<UISend>
}

impl Site {
    pub fn new(site_id: PeerId, mp: Box<MessagePasserT<Msg>>, ui_send: Arc<UISend>) -> Site {
        Site {
            site_id: site_id,
            logical_clock: Clock::new(),
            sequence: Sequence::new(),
            pool: VecDeque::default(),
            message_passer: Arc::new(Mutex::new(mp)),
            ui_send: ui_send}
    }

    pub fn implement_pool(&mut self) {
        loop {
            match self.pool.pop_front() {
                Some(operation) => {
                    self.implement_operation(operation.clone());
                },
                None => { break }
            }
        }
    }

    pub fn parse_given_string(&mut self, file_contents: &str) {
        for (i, c) in file_contents.chars().enumerate() {
            self.generate_insert(i, c, false);
        }
    }

    pub fn content(&mut self) -> String {
        return self.sequence.content();
    }

    pub fn generate_insert(&mut self, pos: usize, alpha: char, broadcast: bool) {
        self.logical_clock.increment();
        let mut position = !0;
        if pos != 0 {
            position = pos - 1;
        }
        let prev_wchar_id = match self.sequence.ith_visible(position) {
            Some(wchar) => wchar.clone().id,
            None => CharId::Beginning
        };
        let next_wchar_id  = match self.sequence.ith_visible(pos) {
            Some(wchar) => wchar.clone().id,
            None => CharId::Ending
        };
        let new_wchar = WootChar::new(create_char_id(self.site_id, self.logical_clock.value.get()), alpha, prev_wchar_id, next_wchar_id);
        let cloned_wchar = new_wchar.clone();
        self.sequence.integrate_ins(new_wchar, cloned_wchar.prev_id.clone(), cloned_wchar.next_id.clone());
        if broadcast {
            self.broadcast(Operation::Insert { w_char: cloned_wchar, from_site: self.site_id })
        }
    }

    pub fn generate_del(&mut self, pos: usize) {
        let mut new_wchar: WootChar = WootChar::new(CharId::Beginning, 'a', CharId::Beginning, CharId::Ending);
        let value_present = match self.sequence.ith_visible(pos) {
            Some(wchar) => {
                new_wchar = wchar.clone();
                true
            },
            None => false
        };
        if value_present {
            self.sequence.integrate_del(&new_wchar);
            self.broadcast(Operation::Delete{ w_char: new_wchar.clone(), from_site: self.site_id })
        }
    }

    pub fn implement_operation(&mut self, operation: Operation) {
        println!("Trying to implement_operation");
        let given_operation = operation.clone();
        match operation {
            Operation::Insert {w_char, from_site:_} => {
                let new_value = w_char.clone();
                let prev_id = w_char.prev_id.clone();
                let next_id = w_char.next_id.clone();
                let id = w_char.id;
                // Insert only if the id doesn't exist
                if !self.sequence.exists(&id) {
                    println!("Id doesn't exist");
                    if self.can_integrate_id(&w_char.prev_id) && self.can_integrate_id(&w_char.next_id) {
                        println!("Can integrate");
                        self.sequence.integrate_ins(new_value, prev_id, next_id);
                        let visible_index = self.sequence.visible_index_of_id(&id);
                        (*self.ui_send)(Command::InsertChar(visible_index, w_char.value));
                    } else {
                        println!("Putting operation in queue");
                        self.pool.push_back(given_operation); // if the operation is not executable, push it back to queue
                        // This is assuming that the loop which processes operations in driver mod will pop them out of queue while calling this function
                    }
                }
            },
            Operation::Delete {w_char, from_site:_} => {
                let exists = self.sequence.exists(&w_char.id);
                let visible_index = self.sequence.visible_index_of_id(&w_char.id);
                if exists {
                    // let can_integrate = self.can_integrate_id(&w_char.prev_id) && self.can_integrate_id(&w_char.next_id);
                    // if can_integrate {
                        self.sequence.integrate_del(&w_char);
                        (*self.ui_send)(Command::DeleteChar(visible_index));
                    // } else {
                        // self.pool.push_back(given_operation); // if the operation is not executable, push it back to queue
                        // This is assuming that the loop which processes operations in driver mod will pop them out of queue while calling this function
                    // }
                }
            }
        }
    }

    fn broadcast(&self, operation: Operation) {
        // Call network manager to broadcast
        unwrap_result!(self.message_passer.lock()).broadcast(Msg::WootOperation(operation));
    }

    pub fn reception(&mut self, encoded: String) {
        // Deserialize
        let decoded: Operation = json::decode(&encoded).unwrap();
        self.pool.push_back(decoded);
    }

    fn can_integrate_id(&self, id: &CharId) -> bool {
        match *id {
            CharId::Beginning => true,
            CharId::Ending => true,
            CharId::Regular {site_id:_, unique_id:_} => self.sequence.exists(id)
        }
    }
}


#[cfg(test)]
mod test{
    use super::*;
    use crust::PeerId;
    use msg::Msg;
    use rand::random;
    use network::{MessagePasserT, Packet};
    use woot::operation::Operation;
    use woot::woot_char::WootChar;
    use woot::char_id::CharId;
    use woot::char_id::create_char_id;
    use std::sync::Arc;
    use std::boxed::Box;

    struct MpNull{
        id: PeerId
    }

    impl MessagePasserT<Msg> for MpNull{
        fn recv(&self) -> Packet<Msg>{
            panic!("unimplemented");
        }
        fn try_recv(&self) -> Option<Packet<Msg>>{
            panic!("unimplemented");
        }
        fn get_id(&self) -> &PeerId{
            &self.id
        }
        fn broadcast(&self, _: Msg){}
        fn send(&self, _: &PeerId, _: Msg){}
    }

    fn create_test_site() -> Site {
        let id: PeerId = random();
        create_test_site_with_id(id)
    }

    fn create_test_site_with_id(id: PeerId) -> Site {
        let mp = MpNull { id: random()};
        let ui_send: UISend = Box::new(move|_| {});
        Site::new(id, Box::new(mp), Arc::new(ui_send))
    }

    #[test]
    fn test_generate_insert() {
        let mut site = create_test_site();
        site.generate_insert(0, 'H', false);
        let val = "H";
        assert_eq!(site.content(), val);
    }

    #[test]
    fn test_generate_del() {
        let mut site = create_test_site();
        site.generate_insert(0, 'A', false);
        site.generate_insert(1, 'P', false);
        site.generate_insert(2, 'R', false);
        site.generate_insert(3, 'A', false);
        site.generate_insert(4, 'P', false);
        site.generate_insert(5, 'R', false);
        assert_eq!(site.content(), "APRAPR");
        site.generate_del(0);
        assert_eq!(site.content(), "PRAPR");
        site.generate_del(3);
        assert_eq!(site.content(), "PRAR");
        site.generate_del(0);
        assert_eq!(site.content(), "RAR");
        site.generate_insert(0, 'P', false);
        assert_eq!(site.content(), "PRAR");
        site.generate_insert(3, 'P', false);
        assert_eq!(site.content(), "PRAPR");
        site.generate_insert(0, 'A', false);
        assert_eq!(site.content(), "APRAPR");
    }

    #[test]
    fn test_operation() {
        let id1: PeerId = random();
        let id2: PeerId = random();
        let id3: PeerId = random();
        let mut site = create_test_site_with_id(id1.clone());
        let mut site2 = create_test_site_with_id(id2.clone());
        let char_id_1 = create_char_id(id1.clone(), 0);
        let char_id_2 = create_char_id(id2.clone(), 0);
        let char_id_3 = create_char_id(id1.clone(), 1);
        let char_id_4 = create_char_id(id3.clone(), 0);
        let char_id_5 = create_char_id(id2.clone(), 1);
        let wchar1 = WootChar::new(char_id_1.clone(), 'a', CharId::Beginning, CharId::Ending); // From site 1
        let wchar2 = WootChar::new(char_id_2.clone(), 'b', CharId::Beginning, CharId::Ending); // From site 2
        let wchar3 = WootChar::new(char_id_3.clone(), 'c', char_id_1.clone(), CharId::Ending); // From site 1
        let wchar4 = WootChar::new(char_id_4.clone(), 'd', CharId::Beginning, CharId::Ending); // From site 3
        let wchar5 = WootChar::new(char_id_5.clone(), 'e', char_id_2.clone(), CharId::Ending); // From site 2
        site.sequence.integrate_ins(wchar1.clone(), CharId::Beginning, CharId::Ending);
        println!("Implementing insert operation from site 1");
        site2.implement_operation(Operation::Insert{w_char: wchar1.clone(), from_site: id1.clone()});
        assert_eq!(site2.content(), site.content());
        println!("Implementing site 2 insert operation");
        site2.sequence.integrate_ins(wchar2.clone(), CharId::Beginning, CharId::Ending);
        site.implement_operation(Operation::Insert{w_char: wchar2.clone(), from_site: id2.clone()});
        assert_eq!(site2.content(), site.content());
        println!("Implementing site 1 insert operation");
        site2.implement_operation(Operation::Insert{w_char: wchar3.clone(), from_site: id1.clone()});
        site.sequence.integrate_ins(wchar3.clone(), char_id_1.clone(), CharId::Ending);
        assert_eq!(site2.content(), site.content());
        println!("Implementing site 3 insert operation");
        site2.implement_operation(Operation::Insert{w_char: wchar4.clone(), from_site: id3.clone()});
        site.sequence.integrate_ins(wchar4.clone(), CharId::Beginning, CharId::Ending);
        assert_eq!(site2.content(), site.content());
        println!("Implementing site 1 delete operation");
        site2.implement_operation(Operation::Delete{w_char: wchar1.clone(), from_site: id1.clone()});
        site.sequence.integrate_del(&wchar1.clone());
        assert_eq!(site2.content(), site.content());
        site2.implement_operation(Operation::Insert{w_char: wchar5.clone(), from_site: id1.clone()});
        site.sequence.integrate_ins(wchar5.clone(), char_id_2.clone(), CharId::Ending);
        assert_eq!(site2.content(), site.content());
    }

    #[test]
    fn test_site() {
        let mut site = create_test_site();
        let file_contents = "fn main() { \n println!(\"Hello, P2P3!\"); \n }";
        site.parse_given_string(file_contents);
        let value = site.content();
        assert_eq!(value, file_contents);
    }
}
