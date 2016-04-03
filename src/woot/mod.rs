#![allow(dead_code)]
use std::cmp::Ordering;
use std::cell::Cell;
static mut wootLogicalClock: u32 = 1;

#[derive(Clone,PartialEq,Debug)]
pub enum Operation {
    Insert {w_char: WootChar, from_site: u32},
    Delete {w_char: WootChar, from_site: u32}
}

#[derive(Clone, PartialEq)]
pub struct Clock {
    value: Cell<u32>,
}

impl Clock {
    pub fn new() -> Clock {
        Clock { value: Cell::new(1)}
    }
    pub fn increment(&self) {
        self.value.set(self.value.get() + 1);
    }
}

impl PartialOrd<Clock> for Clock {
    fn lt(&self, other: &Clock) -> bool {
        return self.value.get() < other.value.get();
    }

    fn partial_cmp(&self, other: &Clock) -> Option<Ordering> {
        if self.value.get() < other.value.get() {
            return Some(Ordering::Less);
        } else if self.value.get() < other.value.get() {
            return Some(Ordering::Greater);
        }
        return Some(Ordering::Equal);
    }
}

#[derive(Clone,PartialEq,Debug)]
pub enum CharId {
    Beginning,
    Ending,
    Regular {site_id: u32, unique_id: u32},// UniqueId is the logicalClock value at the time of creation
}

pub fn create_char_id(site_id: u32, unique_id: u32) -> CharId {
    CharId::Regular {site_id: site_id, unique_id: unique_id}
}

impl PartialOrd<CharId> for CharId {
    fn lt(&self, other: &CharId) -> bool {
        let cloned_self = self.clone();
        let cloned_other = other.clone();
        return match cloned_other {
            CharId::Beginning => false,
            CharId::Ending => true,
            CharId::Regular {site_id, unique_id} => {
                let other_site_id = site_id;
                let other_unique_id = unique_id;
                match cloned_self {
                    CharId::Beginning => true,
                    CharId::Ending => false,
                    CharId::Regular {site_id, unique_id} => {
                        if (site_id < other_site_id) || (site_id == other_site_id && unique_id < other_unique_id) {
                            true
                        } else {
                            false
                        }
                    }
                }
            }
        }
    }

    fn partial_cmp(&self, other: &CharId) -> Option<Ordering> {
        let cloned_self = self.clone();
        let cloned_other = other.clone();
        return match cloned_other {
            CharId::Beginning => Some(Ordering:: Greater),
            CharId::Ending => Some(Ordering:: Less),
            CharId::Regular {site_id, unique_id} => {
                let other_site_id = site_id;
                let other_unique_id = unique_id;
                match cloned_self {
                    CharId::Beginning => Some(Ordering:: Less),
                    CharId::Ending => Some(Ordering:: Greater),
                    CharId::Regular {site_id, unique_id} => {
                        if (site_id < other_site_id) || (site_id == other_site_id && unique_id < other_unique_id) {
                            Some(Ordering:: Less)
                        } else {
                            Some(Ordering:: Greater)
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone,PartialEq,Debug)]
pub struct WootChar {
    id: CharId,
    pub visible: bool,
    value: char,
    prev_id: CharId,
    next_id: CharId,
}

impl WootChar {
    pub fn new(id: CharId, value: char, prev_id: CharId, next_id: CharId) -> WootChar {
        WootChar {id: id, visible: true, value: value, prev_id: prev_id, next_id: next_id}
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }
}

#[test]
fn test_sequence_ith_visible() {
    let mut seq = Sequence::new();
    let regId = create_char_id(1, 1);
    let mut wchar = WootChar::new(regId, 'a', CharId::Beginning, CharId::Beginning);
    seq.list.push(wchar);
    let mut wchar2 = WootChar::new(CharId::Beginning, 'a', CharId::Beginning, CharId::Beginning);
    wchar2.visible = false;
    seq.list.push(wchar2);
    assert_eq!(None, seq.ith_visible(1));
}

#[test]
fn test_sequence_ith_visible2() {
    let mut seq = Sequence::new();
    let regId = create_char_id(1, 1);
    let mut wchar = WootChar::new(regId, 'a', CharId::Beginning, CharId::Beginning);
    seq.list.push(wchar);
    let mut wchar2 = WootChar::new(CharId::Beginning, 'a', CharId::Beginning, CharId::Beginning);
    wchar2.visible = false;
    seq.list.push(wchar2);
    let option = seq.ith_visible(0);
    // let borrow_seq = seq;
    // assert_eq!(Some(&seq.list[0]), option);
}

pub struct Sequence{
    pub list: Vec<WootChar>,
}

impl Sequence {
    pub fn new() -> Sequence {
        Sequence { list: Vec::default()}
    }

    pub fn value(&self) -> String {
        let mut return_string = String::new();
        for wchar in self.list.iter() {
            if wchar.visible {
                return_string.push(wchar.value);
            }
        }
        return_string
    }

    fn exists(&self, id: &CharId) -> bool {
        for wchar in self.list.iter() {
            if wchar.id == *id {
                return true;
            }
        }
        return false;
    }

    pub fn ith_visible(&mut self, i: usize) -> Option<&WootChar> {
        if i >= self.list.len() { // Prevent checking if given index is out of bounds
            return None;
        }
        let mut index_for_visible = 0;
        for wchar in self.list.iter() {
            if wchar.visible {
                if index_for_visible == i {
                    // let mut
                    return Some(wchar);
                }
                index_for_visible += 1;
            }
        }
        return None;
    }

    fn insert(&mut self, wchar: WootChar, position: usize) {
        self.list.insert(position, wchar)
    }

    pub fn integrate_ins(&mut self, wchar: WootChar, prev_id: CharId, next_id: CharId) {
        let sub_sequence = self.sub_sequence(&prev_id, &next_id);
        if sub_sequence.len() == 0 {
            let index_of_next_id = self.position_of_id(&next_id);
            self.insert(wchar, index_of_next_id);
        } else {
            let mut list: Vec<WootChar> = Vec::new();
            match self.wchar_by_id(&prev_id) {
                Some(x) => list.push(x.clone()),
                None => println!("Beginning/Ending found")
            }
            let given_prev_position = self.position_of_id(&prev_id);
            let given_next_position = self.position_of_id(&next_id);
            for elem in sub_sequence.iter() {
                let prev_position = self.position_of_id(&elem.prev_id);
                let next_postion = self.position_of_id(&elem.next_id);
                if prev_position <= given_prev_position && next_postion <= given_next_position {
                    list.push(elem.clone());
                }
            }
            match self.wchar_by_id(&next_id) {
                Some(x) => list.push(x.clone()),
                None => println!("Beginning/Ending found")
            }
            let mut index = 1;
            while index < (list.len() - 1 ) && list.get(index).unwrap().id < wchar.id {
                index += 1;
            }
            let guessed_prev_wchar: WootChar = list.get(index - 1).unwrap().clone();
            let guessed_next_wchar: WootChar = list.get(index).unwrap().clone();
            self.integrate_ins(wchar, guessed_prev_wchar.id.clone(), guessed_next_wchar.id.clone());
         }
    }

    pub fn hide(&mut self, position: usize) {
        match self.list.get_mut(position) {
            Some(x) => x.hide(),
            None => println!("No element found at position {}", position)
        }
    }

    pub fn integrate_del(&mut self, wchar: &WootChar) {
        let position = self.position_of_wchar(wchar);
        self.hide(position);
    }

    fn position_of_wchar(&self, w_char: &WootChar) -> usize {
        let mut val = !0;
        for (i, c) in self.list.iter().enumerate() {
            if c.value == w_char.value {
                val = i;
            }
        }
        val
    }

    fn position_of_id(&self, id: &CharId) -> usize {
        let mut val = !0;
        for (i, c) in self.list.iter().enumerate() {
            if c.id == *id {
                val = i;
            }
        }
        val
    }

    fn wchar_by_id(&self, id: &CharId) -> Option<&WootChar> {
        let mut mut_id = id.clone();
        match mut_id {
            CharId::Beginning => {
                return None;
            },
            CharId::Ending => {
                return None;
            },
            CharId::Regular {site_id, unique_id} => {
                for wchar in self.list.iter() {
                    if wchar.id == mut_id {
                        return Some(wchar);
                    }
                }
            }
        }
        return None;
    }

    /// Returns the part of the sequence between Character represented by prevId and nextId, both not included
    fn sub_sequence(&self, prev_id: &CharId, next_id: &CharId) -> Vec<WootChar> {
        self.list.iter().cloned().filter(|c: &WootChar| (prev_id < &c.id && &c.id < next_id)).collect()
    }
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

    pub fn parse_given_file(&mut self) {

    }

    pub fn generate_insert(&mut self, pos: usize, alpha: char) {
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
        // broadcast
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
