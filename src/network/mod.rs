#![allow(dead_code,unused_variables,unused_imports,unused_must_use)]
extern crate bincode;
extern crate crust;
extern crate core;
extern crate rustc_serialize;
pub mod bootstrap;

use self::core::iter::FromIterator;
use std::collections::{BTreeMap, HashMap};
use std::collections::hash_map::Entry;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use async_queue::AsyncQueue;

//sub-crate imports
use self::crust::{Event, PeerId,Service, ConnectionInfoResult, OurConnectionInfo, TheirConnectionInfo};
use self::bincode::rustc_serialize::{encode, decode};
use self::rustc_serialize::json;

//Aliases
use ::maidsafe_utilities::event_sender::MaidSafeEventCategory as EventCategory;
use ::maidsafe_utilities::event_sender::MaidSafeObserver as Observer;

type Am<T> = Arc<Mutex<T>>;

#[derive(RustcEncodable, RustcDecodable, Clone, Debug)]
pub enum MsgKind {
    Normal,
    Broadcast,
    PeerConnectionInfoRequest,
    PeerConnectionInfoResponse
}

#[derive(RustcEncodable, RustcDecodable, Clone, Debug)]
pub struct PeerConnectionInfoRequest {
    source_id: PeerId,
    bridge_id: PeerId
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct PeerConnectionInfoResponse {
    destination_id: PeerId,
    bridge_id: PeerId,
    info_id: PeerId,
    info: TheirConnectionInfo,
    responder_has_info: bool
}

#[derive(RustcEncodable, RustcDecodable, Clone, Debug)]
pub struct Message {
    pub seq_num: u32,
    pub source: PeerId,
    pub message: String,
    pub kind: MsgKind,
}

//TODO: implement error types correctly
pub trait MessagePasserT{
    fn send_msg(&self, dst: PeerId, msg: Message) -> Result<(), String>;
    fn recv_msg(&self) -> Result<Message, String>;
    fn try_recv_msg(&self) -> Result<Option<Message>, String>;
    fn next_seq_num(&self) -> u32;
    fn get_id(&self) -> PeerId;
    fn peer_exists(&self, peer_id: PeerId) -> bool;
    fn peers(&self) -> Vec<PeerId>;
    fn peers_to_bridge(&self, source_peer: PeerId) -> Vec<PeerId>;

    fn broadcast(&self, msg: String) -> Result<(), String>{
        let msg = Message{
            source: self.get_id(),
            message: msg,
            kind: MsgKind::Broadcast,
            seq_num: self.next_seq_num()};
        for peer in self.peers(){
            unwrap_result!(self.send_msg(peer, msg.clone()));
        }
        Ok(())
    }

    fn broadcast_from_bridge(&self, msg: String, kind: MsgKind, source_peer: PeerId) -> Result<(), String>{
        let msg = Message {
            source: self.get_id(),
            message: msg,
            kind: kind,
            seq_num: self.next_seq_num()
        };
        for peer in self.peers_to_bridge(source_peer) {
            println!("Sending new peer connection alert tp {:?}", peer.clone());
            unwrap_result!(self.send_msg(peer, msg.clone()));
        }
        Ok(())
    }

    fn send(&self, dst: PeerId, msg: String) -> Result<(), String>{
        let msg = Message {
            source: self.get_id(),
            message: msg,
            kind: MsgKind::Normal,
            seq_num: self.next_seq_num()};
        unwrap_result!(self.send_msg(dst, msg));
        Ok(())
    }
}

#[derive(Clone)]
pub struct MessagePasser{
    my_id: PeerId,
    ui_tx: Sender<UiEvent>,
    seq_num: Am<u32>,
    service: Am<Service>,
    recv_queue: Arc<AsyncQueue<Message>>,
    peer_seqs: Am<BTreeMap<PeerId, u32>>,
    conn_token: Am<u32>,
    conn_infos: Am<HashMap<u32,OurConnectionInfo>>,
    // temp_conn_infos intended to be used for full socket connection to store our connection infos sent for other peers
    temp_conn_infos: Am<HashMap<PeerId,u32>>
}

#[derive(Clone,Debug)]
enum UiEvent{
    Terminate
}

impl MessagePasser {
    pub fn new() -> (MessagePasser, JoinHandle<()>) {
        // Construct Service and start listening
        let (nw_tx, nw_rx) = channel();
        let (ui_tx, ui_rx) = channel();
        let (category_tx, category_rx) = channel();

        // register sender
        let nw_sender = Observer::new(nw_tx,
            EventCategory::Crust,
            category_tx.clone());

        let mut service = unwrap_result!(Service::new(nw_sender));
        unwrap_result!(service.start_listening_tcp());
        // unwrap_result!(service.start_listening_utp());

        // Enable listening and responding to peers searching for us.
        service.start_service_discovery();

        let mp = MessagePasser{
            my_id: service.id(),
            ui_tx: ui_tx,
            service: Arc::new(Mutex::new(service)),
            seq_num :Arc::new(Mutex::new(0)),
            recv_queue: Arc::new(AsyncQueue::new()),
            peer_seqs: Arc::new(Mutex::new(BTreeMap::new())),
            conn_token: Arc::new(Mutex::new(0)),
            conn_infos: Arc::new(Mutex::new(HashMap::new())),
            temp_conn_infos: Arc::new(Mutex::new(HashMap::new()))
        };

        let handler = {
            let mp = mp.clone();
            thread::Builder::new()
                .name("CrustNode event handler".to_string())
                .spawn(move || {
                for cat in category_rx.iter() {
                    if let (EventCategory::Crust,Ok(event)) = (cat.clone(),nw_rx.try_recv()){
                        mp.handle_event(event);
                    } else {
                        println!("\nReceived cat {:?} (not handled)", cat);
                    };
                    if let Ok(ui_event) = ui_rx.try_recv(){
                        match ui_event{
                            UiEvent::Terminate => break
                        }
                    }
                }
            })
        };
        (mp,unwrap_result!(handler))
    }

    pub fn prepare_connection_info(&self) -> u32{
        let conn_token_clone = self.conn_token.clone();
        let mut token = conn_token_clone.lock().unwrap();
        *token += 1;
        {
            unwrap_result!(self.service.lock()).prepare_connection_info(*token);
        }
        *token
    }

    pub fn wait_conn_info(&self, tok: u32) -> TheirConnectionInfo{
        loop {
            let mut conns = unwrap_result!(self.conn_infos.lock());
            match conns.entry(tok){
                Entry::Occupied(e) =>{ return e.get().to_their_connection_info();},
                Entry::Vacant(_) => {}
            }
        }
    }

    pub fn connect(&self, i:u32, their_info:TheirConnectionInfo){
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

    pub fn get_service(&self) -> Arc<Mutex<Service>>{
        self.service.clone()
    }

    fn drop(&mut self){
        unwrap_result!(self.ui_tx.send(UiEvent::Terminate));
    }

    fn on_recv_msg(&self, peer_id: PeerId, bytes: Vec<u8>){
        let msg: Message = decode(&bytes[..]).unwrap();
        match msg.kind{
            MsgKind::Normal =>{
                // Add to recv_queue
                self.recv_queue.enq(msg);
            },
            MsgKind::PeerConnectionInfoRequest => {
                let request: PeerConnectionInfoRequest = json::decode(&msg.message).unwrap();
                println!("Got PeerConnectionInfoRequest from {:?} for {:?}", request.bridge_id, request.source_id);
                if self.peer_exists(request.source_id.clone()) {
                    println!("Connection already exists for {:?}", request.source_id.clone());
                    return;
                }
                let mp = self.clone();
                let request_clone = request.clone();
                thread::Builder::new().spawn(move || {
                    let token = mp.prepare_connection_info();
                    let their_info = mp.wait_conn_info(token);
                    let mut conn_infos = unwrap_result!(mp.temp_conn_infos.lock());
                    conn_infos.insert(request.source_id.clone(), token);
                    let peer_info_response = PeerConnectionInfoResponse {
                        destination_id: request_clone.source_id.clone(),
                        bridge_id: request_clone.bridge_id.clone(),
                        info_id: mp.my_id,
                        info: their_info,
                        responder_has_info: false
                    };
                    let message_body = json::encode(&peer_info_response).unwrap();
                    println!("Sending response to {}", request_clone.bridge_id);
                    let msg = Message {
                        source: mp.my_id,
                        message: message_body,
                        kind: MsgKind::PeerConnectionInfoResponse,
                        seq_num: mp.next_seq_num()
                    };
                    mp.send_msg(request_clone.bridge_id, msg).unwrap();
                });
            },
            MsgKind::PeerConnectionInfoResponse => {
                println!("Got PeerConnectionInfoResponse");
                let response: PeerConnectionInfoResponse = json::decode(&msg.message).unwrap();
                let mp = self.clone();
                if self.my_id == response.destination_id {
                    println!("MyId == response's dest id");
                    if !response.responder_has_info {
                        println!("responder does not have my info");
                        thread::Builder::new().spawn(move || {
                            let token = mp.prepare_connection_info();
                            let their_info = mp.wait_conn_info(token);
                            let peer_info_response = PeerConnectionInfoResponse {
                                destination_id: response.info_id,
                                bridge_id: response.bridge_id.clone(),
                                info_id: mp.my_id,
                                info: their_info,
                                responder_has_info:true
                            };
                            let message_body = json::encode(&peer_info_response).unwrap();
                            println!("sending PeerConnectionInfoResponse to {:?}", response.bridge_id);
                            let msg = Message {
                                source: mp.my_id,
                                message: message_body,
                                kind: MsgKind::PeerConnectionInfoResponse,
                                seq_num: mp.next_seq_num()
                            };
                            mp.send_msg(response.bridge_id, msg).unwrap();
                            println!("sending connect with token {} ", token.clone());
                            mp.connect(token, response.info);
                        });
                    } else {
                        println!("responder has my info");
                        // get our connection info from the map from our peer Id
                        let mut conn_infos = unwrap_result!(mp.temp_conn_infos.lock());
                        let token = match  conn_infos.entry(response.destination_id) {
                            Entry::Occupied(e) => *(e.get()),
                            Entry::Vacant(_) => 1 as u32,
                        };
                        // connect(our connection info, their connection info)
                        println!("sending connect with token {} ", token);
                        mp.connect(token, response.info);
                    }
                } else if self.my_id == response.bridge_id  {
                    println!("MyId != response's dest id relaying the message");
                    println!("Dont forget! im the bridge");
                    // relay message to the destination
                    let msg_clone = msg.clone();
                    mp.send_msg(response.destination_id, msg_clone).unwrap();
                }

            },
            MsgKind::Broadcast =>{
                if msg.source == self.my_id {
                    return;
                }
                // update peer_seqs
                {
                    let mut peer_seqs = unwrap_result!(self.peer_seqs.lock());
                    let mut rec_seq = peer_seqs.entry(msg.source).or_insert(0);
                    if *rec_seq >= msg.seq_num{
                        // I already got it and forwarded it
                        return;
                    }
                    //Update the most recent seq_num
                    *rec_seq = msg.seq_num;
                }
                // Add to recv_queue
                self.recv_queue.enq(msg.clone());

                // Forward to those with cyclically greater peer_id values
                let peer_seqs = unwrap_result!(self.peer_seqs.lock());
                for peer in peer_seqs.keys()
                    .skip_while(|k| **k <= self.my_id)
                    .chain(peer_seqs.keys().take_while(|k| **k < msg.source))
                {
                    unwrap_result!(unwrap_result!(self.service.lock()).send(peer, bytes.clone()));
                }
            }
        }
    }

    fn handle_event(&self, event: Event){
        match event{
            // Invoked when a new message is received. Passes the message.
            Event::NewMessage(peer_id, bytes) => {
                println!("Received a new message");
                self.on_recv_msg(peer_id, bytes.clone());
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
            },
            Event::BootstrapConnect(peer_id) => {
                unwrap_result!(self.peer_seqs.lock()).insert(peer_id, 0);
                println!("received BootstrapConnect with peerid: {}", peer_id);
                {
                    let service = unwrap_result!(self.service.lock());
                    self.print_connected_nodes(&service);
                }
            },
            Event::BootstrapAccept(peer_id) => {
                {
                    unwrap_result!(self.peer_seqs.lock()).insert(peer_id, 0);
                }
                println!("received BootstrapAccept with peerid: {}", peer_id);
                {
                    let service = unwrap_result!(self.service.lock());
                    self.print_connected_nodes(&service);
                }
                let request = PeerConnectionInfoRequest {
                    source_id: peer_id,
                    bridge_id: self.my_id
                };
                let message_body = json::encode(&request).unwrap();
                println!("Sending the request to everyone except myself and {:?}", peer_id);
                self.broadcast_from_bridge(message_body, MsgKind::PeerConnectionInfoRequest, peer_id);
            },
            Event::BootstrapFinished =>{
                println!("Receieved BootstrapFinished");
            },
            // The event happens when we use "connect" cmd.
            Event::NewPeer(Ok(()), peer_id) => {
                {
                    unwrap_result!(self.peer_seqs.lock()).insert(peer_id, 0);
                }
                println!("peer connected {}", peer_id);
                {
                    let service = unwrap_result!(self.service.lock());
                    self.print_connected_nodes(&service);
                }
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

    pub fn print_connected_nodes(&self, service: &Service) {
        let peers_id = self.peers();
        println!("Node count: {}", peers_id.len());
        for id in peers_id.iter() {
            if let Some(conn_info) = service.connection_info(id) {
                println!("    [{}]   {} <--> {} [{}][{}]",
                         id, conn_info.our_addr, conn_info.their_addr, conn_info.protocol,
                         if conn_info.closed { "closed" } else { "open" }
                );
            }
        }

        println!("");
    }
}

impl MessagePasserT for MessagePasser {
    fn send_msg(&self, dst:PeerId, msg:Message) -> Result<(),String>{
        // println!("destination: {:?} Message: {:?}",dst, msg);
        let bytes = encode(&msg, bincode::SizeLimit::Infinite).unwrap();
        {
            unwrap_result!(unwrap_result!(self.service.lock()).send(&dst, bytes));
        }
        Ok(())
    }

    fn recv_msg(&self) -> Result<Message, String>{
        Ok(self.recv_queue.deq())
    }

    fn try_recv_msg(&self) -> Result<Option<Message>, String>{
        Ok(self.recv_queue.try_deq())
    }

    fn get_id(&self) -> PeerId {
        self.my_id
    }

    // Used to check existing peer id
    fn peer_exists(&self, peer_id: PeerId) -> bool {
        let mut peer_seqs = unwrap_result!(self.peer_seqs.lock());
        peer_seqs.contains_key(&peer_id)
    }

    fn peers(&self) -> Vec<PeerId>{
        let peer_seqs = unwrap_result!(self.peer_seqs.lock());
        Vec::from_iter(peer_seqs.keys().map(|k| *k))
    }

    fn peers_to_bridge(&self, source_peer: PeerId) -> Vec<PeerId>{
        let peer_seqs = unwrap_result!(self.peer_seqs.lock());
        Vec::from_iter(peer_seqs.keys().map(|k| *k).filter(|k| (*k != self.my_id && *k != source_peer)))
    }

    fn next_seq_num(&self) -> u32{
        let mut seq_num = unwrap_result!(self.seq_num.lock());
        *seq_num+=1;
        *seq_num
    }
}
