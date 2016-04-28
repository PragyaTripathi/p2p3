#![allow(dead_code)]
pub mod bootstrap;

use std::collections::{BTreeMap, HashMap};
use std::collections::hash_map::Entry;
use std::fmt::Debug;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, Condvar};
use std::thread;
use std::thread::JoinHandle;
use async_queue::AsyncQueue;
use crust::{Event, PeerId,Service, ConnectionInfoResult, OurConnectionInfo, TheirConnectionInfo};
use bincode;
use bincode::rustc_serialize::{encode, decode};
use rustc_serialize::{Encodable, Decodable};

//Aliases
use ::maidsafe_utilities::event_sender::MaidSafeEventCategory as EventCategory;
use ::maidsafe_utilities::event_sender::MaidSafeObserver as Observer;

type Am<T> = Arc<Mutex<T>>;

pub trait Message: Encodable + Decodable + Clone + Debug + Send + Sized + 'static {}

#[derive(RustcEncodable, RustcDecodable, Clone, Debug)]
enum InnerMessage<T:Message> {
    Outside(T),
    //Source, Bridge
    PeerConnInfoRequest(PeerId, PeerId),
    //Dest, Bridge, Source, SourceInfo, ReplWithInfo?
    PeerConnInfoResponse(PeerId, PeerId, PeerId, Vec<u8>, bool),
}

#[derive(RustcEncodable, RustcDecodable, Clone, Debug)]
enum Protocol {
    Normal,
    Broadcast,
}

#[derive(RustcEncodable, RustcDecodable, Clone, Debug)]
pub struct Packet<T:Message>{
    seq_num: u32,
    source: PeerId,
    message: InnerMessage<T>,
    protocol: Protocol,
}

impl<T:Message> Packet<T>{
    pub fn seq_num(&self) -> u32 {self.seq_num}
    pub fn source(&self) -> PeerId {self.source}
    pub fn message(&self) -> T {
        if let InnerMessage::Outside(t) = self.message.clone(){
            t
        } else {
            panic!("AHHHHHH");
        }
    }
}

//TODO: implement error types correctly
pub trait MessagePasserT<T:Message>: Send{
    fn recv(&self) -> Packet<T>;
    fn try_recv(&self) -> Option<Packet<T>>;
    fn get_id(&self) -> &PeerId;
    fn broadcast(&self, msg: T);
    fn send(&self, dst: &PeerId, msg: T);
}

#[derive(Clone)]
pub struct MessagePasser<T:Message>{
    my_id: PeerId,
    seq_num: Am<u32>,
    service: Am<Service>,
    recv_queue: Arc<AsyncQueue<Packet<T>>>,
    peer_seqs: Am<BTreeMap<PeerId, u32>>,
    conn_token: Am<u32>,
    conn_infos: Am<HashMap<u32,OurConnectionInfo>>,
    conn_cvar: Arc<Condvar>,
    // temp_conn_infos intended to be used for full socket connection to store our connection infos sent for other peers
    temp_conn_infos: Am<HashMap<PeerId,u32>>,
    on_disconnect: Am<Box<FnMut(&PeerId) + Send>>
}

impl<T:Message> MessagePasser<T> {
    pub fn new() -> (MessagePasser<T>, JoinHandle<()>) {
        // Construct Service and start listening
        let (nw_tx, nw_rx) = channel();
        let (category_tx, category_rx) = channel();

        // register sender
        let nw_sender = Observer::new(nw_tx,
            EventCategory::Crust,
            category_tx.clone());

        let mut service = unwrap_result!(Service::new(nw_sender));
        unwrap_result!(service.start_listening_tcp());
        unwrap_result!(service.start_listening_utp());

        // Enable listening and responding to peers searching for us.
        service.start_service_discovery();

        let mp = MessagePasser{
            my_id: service.id(),
            service: Arc::new(Mutex::new(service)),
            seq_num :Arc::new(Mutex::new(0)),
            recv_queue: Arc::new(AsyncQueue::new()),
            peer_seqs: Arc::new(Mutex::new(BTreeMap::new())),
            conn_token: Arc::new(Mutex::new(0)),
            conn_cvar: Arc::new(Condvar::new()),
            conn_infos: Arc::new(Mutex::new(HashMap::new())),
            temp_conn_infos: Arc::new(Mutex::new(HashMap::new())),
            on_disconnect: Arc::new(Mutex::new(Box::new(|_:&PeerId|{})))
        };

        let handler = {
            let mp = mp.clone();
            thread::spawn(move || {
                for cat in category_rx.iter() {
                    if let (EventCategory::Crust,Ok(event)) = (cat.clone(),nw_rx.try_recv()){
                        mp.handle_event(event);
                    } else {
                        println!("\nReceived cat {:?} (not handled)", cat);
                    };
                }
            })
        };
        (mp,handler)
    }

    pub fn prepare_connection_info(&self) -> u32{
        let mut token = unwrap_result!(self.conn_token.lock());
        *token += 1;
        unwrap_result!(self.service.lock()).prepare_connection_info(*token);
        *token
    }

    pub fn wait_conn_info(&self, tok: u32) -> TheirConnectionInfo{
        let mut conns = unwrap_result!(self.conn_infos.lock());
        while !conns.contains_key(&tok){
            conns = unwrap_result!(self.conn_cvar.wait(conns));
        }
        match conns.entry(tok){
            Entry::Occupied(e) =>{
                return e.get().to_their_connection_info();
            },
            Entry::Vacant(_) => {panic!("This really shouldn't happen!")}
        }
    }

    pub fn connect(&self, i: u32, their_info:TheirConnectionInfo){
        let mut infos = unwrap_result!(self.conn_infos.lock());
        match infos.entry(i){
            Entry::Occupied(oe)=>{
                let our_info = oe.remove();
                let service = unwrap_result!(self.service.lock());
                service.connect(our_info, their_info);
            },
            Entry::Vacant(_) => {}
        }
    }

    fn on_info_req(&self, _: Packet<T>, src: PeerId, bridge: PeerId){
        println!("Got PeerConnectionInfoRequest from {:?} for {:?}", &src, &bridge);

        if unwrap_result!(self.peer_seqs.lock()).contains_key(&src) {
            println!("Connection already exists for {:?}", &src);
            return;
        }
        let mp = self.clone();
        thread::spawn(move || {
            let token = mp.prepare_connection_info();
            let their_info = mp.wait_conn_info(token);
            let mut conn_infos = unwrap_result!(mp.temp_conn_infos.lock());
            if conn_infos.contains_key(&src) {
                println!("Temp connection for {:?} already exists", &src);
                return;
            }
            conn_infos.insert(src.clone(), token);

            let resp = InnerMessage::PeerConnInfoResponse(
                src, bridge, mp.my_id,
                unwrap_result!(encode(&their_info, bincode::SizeLimit::Infinite)), false);
            println!("Sending response to {}", bridge);
            mp.send_inner(&bridge, resp);
        });
    }

    // fired when
    fn on_info_resp(&self, pkt: Packet<T>, dest: PeerId, bridge: PeerId, src: PeerId, src_info: TheirConnectionInfo, has_info: bool){
        println!("Got PeerConnectionInfoResponse");
        if self.my_id == dest {
            println!("MyId == response's dest id");
            if !has_info {
                println!("responder does not have my info");
                let mp = self.clone();
                thread::spawn(move || {
                    let token = mp.prepare_connection_info();
                    let their_info = mp.wait_conn_info(token);
                    let mut conn_infos = unwrap_result!(mp.temp_conn_infos.lock());
                    if conn_infos.contains_key(&src) {
                        println!("Temp connection for {:?} already exists", &src);
                        return;
                    }
                    conn_infos.insert(src.clone(), token);
                    let resp = InnerMessage::PeerConnInfoResponse(
                        src.clone(), bridge.clone(), mp.my_id.clone(),
                        unwrap_result!(encode(&their_info, bincode::SizeLimit::Infinite)), true);
                    println!("sending PeerConnectionInfoResponse to {:?}", bridge);
                    mp.send_inner(&bridge, resp);
                    let peer_seqs = unwrap_result!(mp.peer_seqs.lock());
                    if peer_seqs.contains_key(&src) {
                        println!("Connection already exists with {:?}",&src);
                    } else {
                        println!("sending connect with token {} ", &token);
                        mp.connect(token, src_info);
                    }
                });
            } else {
                println!("responder has my info");
                // get our connection info from the map from our peer Id
                let mut conn_infos = unwrap_result!(self.temp_conn_infos.lock());
                println!("Length of temp conn infos {}", conn_infos.len());
                match conn_infos.entry(src) {
                    Entry::Occupied(e) => {
                        let token = e.remove();
                        if unwrap_result!(self.peer_seqs.lock()).contains_key(&src) {
                            println!("Connection already exists with {:?}", &src);
                        } else {
                            println!("sending connect with token {} ", &token);
                            self.connect(token, src_info);
                        }
                    },
                    Entry::Vacant(_) => panic!("No token in temp conn map for src {:?}", src),
                };
            }
        } else if self.my_id == bridge  {
            println!("MyId == bridge, relaying message");
            // relay message to the destination
            self.send_inner(&dest, pkt.message);
        }
    }

    // fired when message is to be added to queue
    fn on_recv_enq(&self, pkt: Packet<T>){
        match pkt.clone().message{
            InnerMessage::Outside(_) => self.recv_queue.enq(pkt),
            InnerMessage::PeerConnInfoRequest(src,bridge)=>{
                if src != self.my_id{
                    self.on_info_req(pkt, src, bridge);
                }
            },
            InnerMessage::PeerConnInfoResponse(dst,bridge,src,src_info, repl_info)=>{
                self.on_info_resp(pkt, dst, bridge, src, unwrap_result!(decode(&src_info[..])), repl_info);
            }
        }
    }

    // fired whenever a message is received
    fn on_recv_pkt(&self, _: PeerId, pkt: Packet<T>){
        match pkt.protocol {
            Protocol::Normal =>{
                self.on_recv_enq(pkt);
            },
            Protocol::Broadcast =>{
                if pkt.source == self.my_id {
                    return;
                }
                // update peer_seqs
                {
                    let mut peer_seqs = unwrap_result!(self.peer_seqs.lock());
                    let mut rec_seq = peer_seqs.entry(pkt.source).or_insert(0);
                    if *rec_seq >= pkt.seq_num{
                        // I already got it and forwarded it
                        return;
                    }
                    //Update the most recent seq_num
                    *rec_seq = pkt.seq_num;
                }
                // Add to recv_queue
                self.on_recv_enq(pkt.clone());

                // Forward to those with cyclically greater peer_id values
                let peer_seqs = unwrap_result!(self.peer_seqs.lock());
                for peer in peer_seqs.keys()
                    .skip_while(|k| **k <= self.my_id)
                    .chain(peer_seqs.keys().take_while(|k| **k < pkt.source))
                {
                    self.send_pkt(peer, &pkt)
                }
            }
        }
    }

    fn handle_event(&self, event: Event){
        match event{
            // Invoked when a new message is received. Passes the message.
            Event::NewMessage(peer_id, bytes) => {
                let pkt = unwrap_result!(decode(&bytes[..]));
                self.on_recv_pkt(peer_id, pkt);
            },
            // Result to the call of Service::prepare_contact_info.
            Event::ConnectionInfoPrepared(result) => {
                let ConnectionInfoResult {
                    result_token, result } = result;
                let info = match result {
                    Ok(i) => i,
                    Err(e) => {
                        println!("Failed to prepare connection info\ncause: {}", e);
                        return;
                    }
                };

                let mut conn_infos = unwrap_result!(self.conn_infos.lock());
                conn_infos.insert(result_token, info);
                self.conn_cvar.notify_all();
            },
            Event::BootstrapConnect(peer_id) => {
                unwrap_result!(self.peer_seqs.lock()).insert(peer_id, 0);
                println!("received BootstrapConnect with peerid: {}", peer_id);
                self.print_connected_nodes();
            },
            Event::BootstrapAccept(peer_id) => {
                unwrap_result!(self.peer_seqs.lock()).insert(peer_id, 0);
                println!("received BootstrapAccept with peerid: {}", peer_id);
                self.print_connected_nodes();
                let request = InnerMessage::PeerConnInfoRequest(peer_id, self.my_id);
                self.broadcast_inner(request);
            },
            Event::BootstrapFinished =>{
                println!("Receieved BootstrapFinished");
            },
            // The event happens when we use "connect" cmd.
            Event::NewPeer(Ok(()), peer_id) => {
                unwrap_result!(self.peer_seqs.lock()).insert(peer_id, 0);
                println!("peer connected {}", peer_id);
                self.print_connected_nodes();
            },
            Event::LostPeer(peer_id) => {
                unwrap_result!(self.peer_seqs.lock()).remove(&peer_id);
                println!("peer disconnected {}", peer_id);
            },
            e => {
                println!("\nReceived event {:?} (not handled)", e);
            }
        }
    }

    pub fn print_connected_nodes(&self) {
        let service = unwrap_result!(self.service.lock());
        println!("Node count: {}", unwrap_result!(self.peer_seqs.lock()).len());
        for id in self.peers() {
            if let Some(conn_info) = service.connection_info(&id) {
                println!("    [{}]   {} <--> {} [{}][{}]",
                         id, conn_info.our_addr, conn_info.their_addr, conn_info.protocol,
                         if conn_info.closed { "closed" } else { "open" }
                );
            }
        }
        println!("");
    }

    fn broadcast_inner(&self, msg: InnerMessage<T>){
        let pkt = Packet{
            source: self.get_id().clone(),
            message: msg,
            protocol: Protocol::Broadcast,
            seq_num: self.next_seq_num()};
        for peer in self.peers(){
            self.send_pkt(&peer, &pkt);
        }
    }

    fn send_inner(&self, dst: &PeerId, msg: InnerMessage<T>){
        let pkt = Packet{
            source: self.get_id().clone(),
            message: msg,
            protocol: Protocol::Normal,
            seq_num: self.next_seq_num()};
        self.send_pkt(&dst, &pkt);
    }

    fn send_pkt(&self, dst: &PeerId, msg: &Packet<T>){
        let bytes = encode(msg, bincode::SizeLimit::Infinite).unwrap();
        unwrap_result!(unwrap_result!(self.service.lock()).send(&dst, bytes));
    }

    pub fn peers(&self) -> Vec<PeerId>{
        let peer_seqs = unwrap_result!(self.peer_seqs.lock());
        peer_seqs.keys().map(|k| *k).collect()
    }

    fn next_seq_num(&self) -> u32{
        let mut seq_num = unwrap_result!(self.seq_num.lock());
        *seq_num+=1;
        *seq_num
    }

    fn set_on_disconnect(&self, fun: Box<FnMut(&PeerId) + Send>)
    {
        let mut on_dis = unwrap_result!(self.on_disconnect.lock());
        *on_dis = fun;
    }
}

impl<T:Message> MessagePasserT<T> for MessagePasser<T>{
    fn broadcast(&self, msg: T){
        self.broadcast_inner(InnerMessage::Outside(msg))
    }

    fn send(&self, dst: &PeerId, msg: T){
        self.send_inner(dst, InnerMessage::Outside(msg));
    }

    fn recv(&self) -> Packet<T>{
        self.recv_queue.deq()
    }

    fn try_recv(&self) -> Option<Packet<T>>{
        self.recv_queue.try_deq()
    }

    fn get_id(&self) -> &PeerId {
        &self.my_id
    }
}

#[cfg(test)]
mod test{
    use super::*;
    use std::time::Instant;

    #[derive(RustcEncodable, RustcDecodable, Clone, Debug, PartialEq)]
    struct TestMsg(String);

    impl Message for TestMsg{}

    #[ignore]
    #[test]
    fn two_nodes(){
        let (mp,_) = MessagePasser::new();
        let (mp2,_) = MessagePasser::new();
        let instant = Instant::now();
        while mp.peers().len() == 0 {
            assert!(instant.elapsed().as_secs() < 20);
        }
        while mp2.peers().len() == 0 {
            assert!(instant.elapsed().as_secs() < 20);
        }
        assert!(mp.peers().len() == 1 && mp2.peers().len() == 1);

        mp.send(mp2.get_id(), TestMsg("message1".to_string()));
        mp2.send(mp.get_id(), TestMsg("message2".to_string()));
        assert_eq!(mp2.recv().message(), TestMsg("message1".to_string()));
        assert_eq!(mp.recv().message(), TestMsg("message2".to_string()));
    }

    #[test]
    fn three_nodes(){
        let (mp,_) = MessagePasser::new();
        let (mp2,_) = MessagePasser::new();
        let (mp3,_) = MessagePasser::new();
        let instant = Instant::now();
        while mp.peers().len() < 2 {
            assert!(instant.elapsed().as_secs() < 20);
        }
        while mp2.peers().len() < 2 {
            assert!(instant.elapsed().as_secs() < 20);
        }
        while mp3.peers().len() < 2 {
            assert!(instant.elapsed().as_secs() < 20);
        }

        assert_eq!(mp.peers().len(),2);
        assert_eq!(mp2.peers().len(),2);
        assert_eq!(mp3.peers().len(),2);

        mp.send(mp2.get_id(), TestMsg("message1".to_string()));
        mp2.send(mp3.get_id(), TestMsg("message2".to_string()));
        mp3.send(mp.get_id(), TestMsg("message3".to_string()));
        assert_eq!(mp2.recv().message(), TestMsg("message1".to_string()));
        assert_eq!(mp3.recv().message(), TestMsg("message2".to_string()));
        assert_eq!(mp.recv().message(), TestMsg("message3".to_string()));
    }
}
