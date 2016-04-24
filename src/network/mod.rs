extern crate bincode;
extern crate crust;
extern crate core;

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

#[derive(RustcEncodable, RustcDecodable, Clone)]
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
    fn peers(&self) -> Vec<PeerId>;

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
    conn_infos: Am<HashMap<u32,OurConnectionInfo>>
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
        unwrap_result!(service.start_listening_utp());

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
            conn_infos: Arc::new(Mutex::new(HashMap::new()))};

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
        let mut token = unwrap_result!(self.conn_token.lock());
        unwrap_result!(self.service.lock()).prepare_connection_info(*token);
        let ret = *token;
        *token+=1;
        ret
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
                /*
                Spawn new thread,
            	prepare connection info
            	Wait for connection info to be available
            	Put connection_info in a map(sourceId, connection Info)
            	Send peer_info_response(sourceId, bridge_id, my Id, new info, false)
                */
                // thread.spawn(move || {
                //     let token = self.prepare_connection_info();
                //     let their_info = self.wait_conn_info(token);
                //
                // });
            },
            MsgKind::PeerConnectionInfoResponse => {

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
                let service = unwrap_result!(self.service.lock());
                self.print_connected_nodes(&service);
            },
            Event::BootstrapAccept(peer_id) => {
                unwrap_result!(self.peer_seqs.lock()).insert(peer_id, 0);
                println!("received BootstrapAccept with peerid: {}", peer_id);
                let service = unwrap_result!(self.service.lock());
                self.print_connected_nodes(&service);
            },
            Event::BootstrapFinished =>{
                println!("Receieved BootstrapFinished");
            },
            // The event happens when we use "connect" cmd.
            Event::NewPeer(Ok(()), peer_id) => {
                unwrap_result!(self.peer_seqs.lock()).insert(peer_id, 0);
                println!("peer connected {}", peer_id);
                let service = unwrap_result!(self.service.lock());
                self.print_connected_nodes(&service);
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
        let bytes = encode(&msg, bincode::SizeLimit::Infinite).unwrap();
        unwrap_result!(unwrap_result!(self.service.lock()).send(&dst, bytes));
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

    fn peers(&self) -> Vec<PeerId>{
        let peer_seqs = unwrap_result!(self.peer_seqs.lock());
        Vec::from_iter(peer_seqs.keys().map(|k| *k))
    }

    fn next_seq_num(&self) -> u32{
        let mut seq_num = unwrap_result!(self.seq_num.lock());
        *seq_num+=1;
        *seq_num
    }
}
