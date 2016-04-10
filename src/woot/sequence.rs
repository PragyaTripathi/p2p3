#![allow(dead_code)]
use super::woot_char::WootChar;
use super::char_id::CharId;

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

    pub fn exists(&self, id: &CharId) -> bool {
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
        let mut_id = id.clone();
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
