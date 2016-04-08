use crust::PeerId;
use std::collections::HashMap;

pub struct MsgPasser {
    //receivedMsgs: String,
    receivedBytes: usize,
    //peerId: PeerId,
    seqNum: u32,
    pub nodes: HashMap<PeerId, u32>,
}

impl MsgPasser {
    pub fn new() -> MsgPasser {
        MsgPasser {
            //nodes: HashMap::new(),
            //receivedMsgs: "",
            receivedBytes: 0,
            //peerId: null,
            seqNum: 0,
            nodes: HashMap::new(),
        }
    }
    /*
    pub fn getMsg(self) -> String {
        return self.receivedMsgs;
    }*/
    pub fn get_seq_num(&self) -> u32 {
        self.seqNum
    }

    pub fn nextSeqNum(&mut self) -> u32 {
        self.seqNum += 1;
        self.seqNum
    }

    pub fn getNewMsg(&mut self, msg: String) -> String {
        let newStr = self.seqNum.to_string() + " " + msg.as_str();
        self.incSeq();
        return newStr;
    }

    pub fn incSeq(&mut self) {
        self.seqNum += 1;
    }

    //pub fn handleBroadcast(&mut self, msg: String, id: &PeerId) -> bool {
    pub fn handleBroadcast(&mut self, msgSeq: u32) -> bool {
        if msgSeq < self.seqNum {
            return false;
        } else {
            self.incSeq();
            return true;
        }
    }

    pub fn trimMsg(msg: &String) -> Vec<&str> {
        let mut msgs = msg.trim_right_matches(|c| c == '\r' || c == '\n')
        .split(' ')
        .collect::<Vec<_>>();
        msgs
    }
}
