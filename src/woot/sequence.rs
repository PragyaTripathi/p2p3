#![allow(dead_code)]
use super::woot_char::WootChar;
use super::char_id::CharId;
use super::char_id::create_char_id;

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
        let mut val = false;
        match *id {
            CharId::Beginning => {
                val = true;
            },
            CharId::Ending => {
                val = true;
            },
            CharId::Regular {site_id, unique_id} => {
                for wchar in self.list.iter() {
                    if wchar.id == *id {
                        val = true;
                    }
                }
            }
        }
        val
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
                if prev_position <= given_prev_position && given_next_position <= next_postion  {
                    list.push(elem.clone());
                }
            }
            match self.wchar_by_id(&next_id) {
                Some(x) => list.push(x.clone()),
                None => println!("Beginning/Ending found")
            }
            let mut index = 0;
            while index < (list.len() - 1 ) && list[index].id < wchar.id {
                index += 1;
            }
            let guessed_prev_wchar: WootChar = list[index - 1].clone();
            let guessed_next_wchar: WootChar = list[index].clone();
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
        print!("At integrate_del position {}", position);
        self.hide(position);
    }

    fn position_of_wchar(&self, w_char: &WootChar) -> usize {
        let mut val = !0;
        for (i, c) in self.list.iter().enumerate() {
            println!("Wchar Position {}", i);
            if c.value == w_char.value {
                val = i;
            }
        }
        val
    }

    fn position_of_id(&self, id: &CharId) -> usize {
        let mut val = !0;
        match *id {
            CharId::Beginning => {
                val = 0;
            },
            CharId::Ending => {
                val = self.list.len();
            },
            CharId::Regular {site_id, unique_id} => {
                for (i, c) in self.list.iter().enumerate() {
                    if c.id == *id {
                        val = i;
                    }
                }
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
fn test_integrate_del() {
    let mut seq = Sequence::new();
    let char_id_1 = create_char_id(1, 0);
    let char_id_2 = create_char_id(1, 1);
    let char_id_3 = create_char_id(1, 2);
    let char_id_4 = create_char_id(1, 3);
    let char_id_5 = create_char_id(1, 4);
    let mut wchar1 = WootChar::new(char_id_1.clone(), 'a', CharId::Beginning, CharId::Ending);
    let mut wchar2 = WootChar::new(char_id_2.clone(), 'b', char_id_1.clone(), CharId::Ending);
    let mut wchar3 = WootChar::new(char_id_3.clone(), 'c', char_id_2.clone(), CharId::Ending);
    let mut wchar4 = WootChar::new(char_id_4.clone(), 'd', char_id_2.clone(), char_id_3.clone());
    let mut wchar5 = WootChar::new(char_id_5.clone(), 'e', CharId::Beginning, CharId::Ending);
    seq.integrate_ins(wchar1, CharId::Beginning, CharId::Ending);
    assert_eq!(seq.value(), "a");
    seq.integrate_ins(wchar2.clone(), char_id_1, CharId::Ending);
    assert_eq!(seq.value(), "ab");
    seq.integrate_del(&wchar2);
    assert_eq!(seq.value(), "a");
    seq.integrate_ins(wchar3, char_id_2.clone(), CharId::Ending);
    assert_eq!(seq.value(), "ac");
    seq.integrate_ins(wchar4, char_id_2, char_id_3);
    assert_eq!(seq.value(), "adc");
}

#[test]
fn test_integrate_ins() {
    let mut seq = Sequence::new();
    let char_id_1 = create_char_id(1, 0);
    let char_id_2 = create_char_id(1, 1);
    let char_id_3 = create_char_id(1, 2);
    let char_id_4 = create_char_id(1, 3);
    let mut wchar1 = WootChar::new(char_id_1.clone(), 'a', CharId::Beginning, CharId::Ending);
    let mut wchar2 = WootChar::new(char_id_2.clone(), 'b', char_id_1.clone(), CharId::Ending);
    let mut wchar3 = WootChar::new(char_id_3.clone(), 'c', char_id_2.clone(), CharId::Ending);
    let mut wchar4 = WootChar::new(char_id_4.clone(), 'd', char_id_2.clone(), char_id_3.clone());
    seq.integrate_ins(wchar1, CharId::Beginning, CharId::Ending);
    assert_eq!(seq.value(), "a");
    seq.integrate_ins(wchar2, char_id_1, CharId::Ending);
    assert_eq!(seq.value(), "ab");
    seq.integrate_ins(wchar3, char_id_2.clone(), CharId::Ending);
    assert_eq!(seq.value(), "abc");
    seq.integrate_ins(wchar4, char_id_2, char_id_3);
    assert_eq!(seq.value(), "abdc");
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
fn test_sub_sequence() {
    let mut seq = Sequence::new();
    let char_id_1 = create_char_id(1, 0);
    let char_id_2 = create_char_id(1, 1);
    let char_id_3 = create_char_id(1, 2);
    let char_id_4 = create_char_id(1, 3);
    let mut wchar1 = WootChar::new(char_id_1.clone(), 'a', CharId::Beginning, CharId::Ending);
    let mut wchar2 = WootChar::new(char_id_2.clone(), 'b', char_id_1.clone(), CharId::Ending);
    let mut wchar3 = WootChar::new(char_id_3.clone(), 'c', char_id_2.clone(), CharId::Ending);
    let mut wchar4 = WootChar::new(char_id_4.clone(), 'd', char_id_2.clone(), char_id_3.clone());
    seq.integrate_ins(wchar1.clone(), CharId::Beginning, CharId::Ending);
    seq.integrate_ins(wchar2.clone(), char_id_1.clone(), CharId::Ending);
    seq.integrate_ins(wchar3.clone(), char_id_2.clone(), CharId::Ending);
    seq.integrate_ins(wchar4.clone(), char_id_2.clone(), char_id_3.clone());
    let sub_seq_1 = seq.sub_sequence(&wchar1.prev_id, &wchar1.next_id);
    assert_eq!(sub_seq_1.len(), 4);
    let sub_seq_2 = seq.sub_sequence(&char_id_1, &wchar1.next_id);
    assert_eq!(sub_seq_2.len(), 3);
    let sub_seq_3 = seq.sub_sequence(&char_id_1, &char_id_4);
    assert_eq!(sub_seq_3.len(), 2);
}