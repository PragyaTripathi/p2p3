#![allow(dead_code)]

extern crate rustc_serialize;
use rustc_serialize::json;
use super::clock::Clock;
use super::sequence::Sequence;
use super::operation::Operation;
use super::woot_char::WootChar;
use super::char_id::CharId;
use super::char_id::create_char_id;

pub struct Site {
    site_id: u32,
    logical_clock: Clock,
    sequence: Sequence,
    pool: Vec<Operation>
}

impl Site {
    pub fn new(id: u32) -> Site {
        Site {site_id: id, logical_clock: Clock::new(), sequence: Sequence::new(), pool: Vec::default()}
    }

    pub fn parse_given_string(&mut self, file_contents: &str) {
        for (i, c) in file_contents.chars().enumerate() {
            self.generate_insert(i, c, false);
        }
    }

    pub fn value(&mut self) -> String {
        return self.sequence.value();
    }

    pub fn generate_insert(&mut self, pos: usize, alpha: char, broadcast: bool) {
        self.logical_clock.increment();
        let mut position = pos;
        if !(pos == 0 && pos == self.sequence.list.len()) {
            position -= 1;
        }
        let prev_wchar_id = match self.sequence.ith_visible(position) {
            Some(wchar) => wchar.clone().id,
            None => CharId::Beginning
        };
        let next_wchar_id  = match self.sequence.ith_visible(position+1) {
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
        let given_operation = operation.clone();
        match operation {
            Operation::Insert {w_char, from_site} => {
                let new_value = w_char.clone();
                let prev_id = w_char.prev_id.clone();
                let next_id = w_char.next_id.clone();
                let id = w_char.id;
                // Insert only if the id doesn't exist
                if !self.sequence.exists(&id) {
                    if self.can_integrate_id(&w_char.prev_id) && self.can_integrate_id(&w_char.next_id) {
                        self.sequence.integrate_ins(new_value, prev_id, next_id)
                    } else {
                        self.pool.push(given_operation); // if the operation is not executable, push it back to queue
                        // This is assuming that the loop which processes operations in driver mod will pop them out of queue while calling this function
                    }
                }
            },
            Operation::Delete {w_char, from_site} => {
                let exists = self.sequence.exists(&w_char.id);
                if exists {
                    let can_integrate = self.can_integrate_id(&w_char.prev_id) && self.can_integrate_id(&w_char.next_id);
                    if can_integrate {
                        self.sequence.integrate_del(&w_char);
                    } else {
                        self.pool.push(given_operation); // if the operation is not executable, push it back to queue
                        // This is assuming that the loop which processes operations in driver mod will pop them out of queue while calling this function
                    }
                }
            }
        }
    }

    fn broadcast(&self, operation: Operation) {
        // Serialize
        let encoded = json::encode(&operation).unwrap();
        // Call network manager to broadcast
    }

    fn reception(&mut self, encoded: String) {
        // Deserialize
        let decoded: Operation = json::decode(&encoded).unwrap();
        self.pool.push(decoded);
    }

    fn can_integrate_id(&self, id: &CharId) -> bool {
        match *id {
            CharId::Beginning => true,
            CharId::Ending => true,
            CharId::Regular {site_id, unique_id} => self.sequence.exists(id)
        }
    }
}

#[test]
fn test_generate_insert() {
    let mut site = Site::new(1);
    site.generate_insert(0, 'H', false);
    let val = "H";
    assert_eq!(site.value(), val);
}

#[test]
fn test_generate_del() {
    let mut site = Site::new(1);
    site.generate_insert(0, 'H', false);
    site.generate_del(0);
    assert_eq!(site.value(), "");
}

#[test]
fn test_operation() {
    let mut site = Site::new(1);
    let mut site2 = Site::new(2);
    let char_id_1 = create_char_id(1, 0);
    let char_id_2 = create_char_id(2, 0);
    let char_id_3 = create_char_id(1, 1);
    let char_id_4 = create_char_id(1, 2);
    let char_id_5 = create_char_id(2, 1);
    let mut wchar1 = WootChar::new(char_id_1.clone(), 'a', CharId::Beginning, CharId::Ending); // From site 1
    let mut wchar2 = WootChar::new(char_id_2.clone(), 'b', CharId::Beginning, CharId::Ending); // From site 2
    let mut wchar3 = WootChar::new(char_id_3.clone(), 'c', char_id_1.clone(), CharId::Ending); // From site 1
    let mut wchar4 = WootChar::new(char_id_4.clone(), 'd', char_id_3.clone(), CharId::Ending); // From site 1
    let mut wchar5 = WootChar::new(char_id_5.clone(), 'e', char_id_2.clone(), CharId::Ending); // From site 2
    site.sequence.integrate_ins(wchar1.clone(), CharId::Beginning, CharId::Ending);
    site2.implement_operation(Operation::Insert{w_char: wchar1.clone(), from_site: 1});
    assert_eq!(site2.value(), "a");
    site2.implement_operation(Operation::Delete{w_char: wchar1.clone(), from_site: 1});
    assert_eq!(site2.value(), "");
    site2.implement_operation(Operation::Delete{w_char: wchar5.clone(), from_site: 1});
    assert_eq!(site2.pool.len(), 0);
}

#[test]
fn test_site() {
    let mut site = Site::new(1);
    let file_contents = "fn main() { \n println!(\"Hello, P2P3!\"); \n }";
    site.parse_given_string(file_contents);
    let value = site.value();
    assert_eq!(value, file_contents);
}