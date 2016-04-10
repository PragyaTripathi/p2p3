#![allow(dead_code)]

use std::cmp::Ordering;

#[derive(Clone,PartialEq,Debug)]
pub enum CharId {
    Beginning,
    Ending,
    Regular {site_id: u32, unique_id: u32},// UniqueId is the logicalClock value at the time of creation
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
