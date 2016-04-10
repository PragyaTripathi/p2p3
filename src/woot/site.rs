#![allow(dead_code)]

use super::clock::Clock;
use super::sequence::Sequence;
use super::operation::Operation;
use super::woot_char::WootChar;
use super::char_id::CharId;

pub fn create_char_id(site_id: u32, unique_id: u32) -> CharId {
    CharId::Regular {site_id: site_id, unique_id: unique_id}
}

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
        let prev_wchar_id = match self.sequence.ith_visible(pos) {
            Some(wchar) => wchar.clone().id,
            None => CharId::Beginning
        };
        let next_wchar_id  = match self.sequence.ith_visible(pos+1) {
            Some(wchar) => wchar.clone().id,
            None => CharId::Ending
        };
        let new_wchar = WootChar::new(create_char_id(self.site_id, self.logical_clock.value.get()), alpha, prev_wchar_id, next_wchar_id);
        let cloned_wchar = new_wchar.clone();
        self.sequence.integrate_ins(new_wchar, cloned_wchar.prev_id, cloned_wchar.next_id);
        if broadcast {
            // broadcast
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
            //broadcast
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
                if self.sequence.exists(&id) {
                    if self.can_integrate_id(&w_char.prev_id) && self.can_integrate_id(&w_char.next_id) {
                        self.sequence.integrate_ins(new_value, prev_id, next_id)
                    } else {
                        self.pool.push(given_operation);
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
                        self.pool.push(given_operation);
                    }
                }
            }
        }
    }

    fn reception(&mut self, operation: Operation) {
        self.pool.push(operation);
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
fn test_site() {
    let mut site = Site::new(1);
    let file_contents = "fn main() { \n println!(\"Hello, P2P3!\"); \n }";
    site.parse_given_string(file_contents);
    let value = site.sequence.value();
    assert_eq!(value, file_contents);
}
