extern crate bincode;
extern crate crust;
extern crate core;

pub mod network_manager;
pub mod cmd_parser;
pub mod msg_passer;
pub mod bootstrap;

use self::core::iter::FromIterator;
use rustc_serialize::json;
use rustc_serialize::json::{as_json,as_pretty_json};
use std::collections::{BTreeMap, VecDeque, HashMap};
use std::collections::hash_map::Entry;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, Condvar};
use std::thread;
use rustc_serialize::json::Json;
use network::bootstrap::BootstrapHandler;

//sub-crate imports
use self::crust::{Event, PeerId,Service, OurConnectionInfo, ConnectionInfoResult, StaticContactInfo};
use self::bincode::rustc_serialize::{encode, decode};

//Aliases
use ::maidsafe_utilities::event_sender::MaidSafeEventCategory as EventCategory;
use ::maidsafe_utilities::event_sender::MaidSafeObserver as Observer;

type Am<T> = Arc<Mutex<T>>;

#[derive(RustcEncodable, RustcDecodable, Clone)]
pub enum MsgKind {
    Normal,
    Broadcast,
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
        let msg = Message{
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
    recv_cvar: Arc<Condvar>,
    recv_queue: Am<VecDeque<Message>>,
    peer_seqs: Am<BTreeMap<PeerId, u32>>,
    conn_token: Am<u32>,
    conn_infos: Am<HashMap<u32,OurConnectionInfo>>,
    bootstrap_handler: BootstrapHandler
}

#[derive(Clone,Debug)]
enum UiEvent{
    Terminate
}

impl MessagePasser {
    pub fn new(boot: BootstrapHandler) -> MessagePasser {
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
            recv_queue: Arc::new(Mutex::new(VecDeque::new())),
            peer_seqs: Arc::new(Mutex::new(BTreeMap::new())),
            recv_cvar: Arc::new(Condvar::new()),
            conn_token: Arc::new(Mutex::new(0)),
            conn_infos: Arc::new(Mutex::new(HashMap::new())),
            bootstrap_handler: boot};

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
        mp
    }

    pub fn prepare_connection_info(&self){
        let mut token = unwrap_result!(self.conn_token.lock());
        unwrap_result!(self.service.lock()).prepare_connection_info(*token);
        *token+=1;
    }

    pub fn connect(&self, i:u32, their_info:String){
        let mut infos = unwrap_result!(self.conn_infos.lock());
        match infos.entry(i){
            Entry::Occupied(oe)=>{
                let our_info = oe.remove();
                let their_info = unwrap_result!(json::decode(&their_info));
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
        self.ui_tx.send(UiEvent::Terminate);
    }

    fn on_recv_msg(&self, peer_id: PeerId, bytes: Vec<u8>){
        let msg: Message = decode(&bytes[..]).unwrap();
        match msg.kind{
            MsgKind::Normal =>{
                // Add to recv_queue
                unwrap_result!(self.recv_queue.lock()).push_back(msg);
                // Trigger the conditional variable
                self.recv_cvar.notify_one();
            },
            MsgKind::Broadcast =>{
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
                unwrap_result!(self.recv_queue.lock()).push_back(msg.clone());
                // Trigger the conditional variable
                self.recv_cvar.notify_one();

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
                let decoded_msg: Message = decode(&bytes[..]).unwrap();
                let kind = match decoded_msg.kind {
                    MsgKind::Broadcast => "Broadcast",
                    MsgKind::Normal => "Normal"
                };
                println!("message from {}: [{}] {}", peer_id, kind, decoded_msg.message);
                //println!("message from {}: {}", peer_id, String::from_utf8(bytes).unwrap());
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
                println!("Prepared connection info with id {}", result_token);

                let their_info = info.to_their_connection_info();
                //let info_json = unwrap_result!(json::encode(&their_info));
                println!("Share this info with the peer you want to connect to:");
                println!("{}", as_json(&their_info));
                let mut conn_infos = unwrap_result!(self.conn_infos.lock());
                conn_infos.insert(result_token, info);

                /*
                 *  Update config file.
                 */
                let info_json = unwrap_result!(json::encode(&their_info));
                                //
                let data = Json::from_str(info_json.as_str()).unwrap();
                let obj = data.as_object().unwrap();
                let foo = obj.get("static_contact_info").unwrap();

                let json_str: String = foo.to_string();

                let mut info: StaticContactInfo = json::decode(&json_str).unwrap();
                info.tcp_acceptors.remove(0);
                self.bootstrap_handler.update_config(info);
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
        let mut recv_q = unwrap_result!(self.recv_queue.lock());
        while let None = recv_q.front(){
            recv_q = unwrap_result!(self.recv_cvar.wait(recv_q));
        }
        Ok(recv_q.pop_front().unwrap())
    }

    fn try_recv_msg(&self) -> Result<Option<Message>, String>{
        let mut recv_q = unwrap_result!(self.recv_queue.lock());
        Ok(recv_q.pop_front())
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
