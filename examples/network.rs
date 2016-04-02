/*
    run "cargo run --example network" in different terminals to test the network module.
*/

extern crate rustc_serialize;
extern crate docopt;
#[macro_use]
extern crate maidsafe_utilities; // macro unwrap!()
//
extern crate crust;
extern crate p2p3;

use rustc_serialize::{Decodable, Decoder, json};
use docopt::Docopt;
use std::io;
//
use std::sync::mpsc::channel;
use crust::{Service, Protocol, Endpoint, ConnectionInfoResult,
            SocketAddr, OurConnectionInfo,
            PeerId};
use std::sync::{Arc, Mutex};
use p2p3::network::network_manager::Network;
use p2p3::network::network_manager::handle_new_peer;
use p2p3::network::cmd_parser;
use p2p3::network::cmd_parser::UserCommand;
use p2p3::network::cmd_parser::parse_user_command;
use std::thread;
use std::str::FromStr;




fn main() {

    // Construct Service and start listening
    let (channel_sender, channel_receiver) = channel();
    let (category_tx, category_rx) = channel();

    let (bs_sender, bs_receiver) = channel();
    let crust_event_category =
        ::maidsafe_utilities::event_sender::MaidSafeEventCategory::Crust;
    let event_sender =
        ::maidsafe_utilities::event_sender::MaidSafeObserver::new(channel_sender,
                                                                  crust_event_category,
                                                                  category_tx);
    let mut config = unwrap_result!(::crust::read_config_file());


    let mut service = unwrap_result!(Service::with_config(event_sender, &config));
    unwrap_result!(service.start_listening_tcp());
    unwrap_result!(service.start_listening_utp());
    service.start_service_discovery();

    let service = Arc::new(Mutex::new(service));
    let service_cloned = service.clone();

    let network = Arc::new(Mutex::new(Network::new()));
    let network2 = network.clone();

    // Start event-handling thread

    let handler = match thread::Builder::new().name("CrustNode event handler".to_string())
                                              .spawn(move || {
        let service = service_cloned;
        for it in category_rx.iter() {
            match it {
                ::maidsafe_utilities::event_sender::MaidSafeEventCategory::Crust => {
                    if let Ok(event) = channel_receiver.try_recv() {
                        match event {
                            crust::Event::NewMessage(peer_id, bytes) => {
                                let message_length = bytes.len();
                                let mut network = unwrap_result!(network2.lock());
                                network.record_received(message_length);
                                println!("\nReceived from {:?} message: {}",
                                         peer_id,
                                         String::from_utf8(bytes)
                                         .unwrap_or(format!("non-UTF-8 message of {} bytes",
                                                            message_length)));
                            },
                            crust::Event::ConnectionInfoPrepared(result) => {
                                let ConnectionInfoResult {
                                    result_token, result } = result;
                                let info = match result {
                                    Ok(i) => i,
                                    Err(e) => {
                                        println!("Failed to prepare connection info\ncause: {}", e);
                                        continue;
                                    }
                                };
                                println!("Prepared connection info with id {}", result_token);
                                let their_info = info.to_their_connection_info();
                                let info_json = unwrap_result!(json::encode(&their_info));
                                println!("Share this info with the peer you want to connect to:");
                                println!("{}", info_json);
                                let mut network = unwrap_result!(network2.lock());
                                if let Some(_) = network.our_connection_infos.insert(result_token, info) {
                                    panic!("Got the same result_token twice!");
                                };
                            },
                            crust::Event::BootstrapConnect(peer_id) => {
                                println!("\nBootstrapConnect with peer {:?}", peer_id);
                                let peer_index = handle_new_peer(&unwrap_result!(service.lock()), network2.clone(), peer_id);
                                let _ = bs_sender.send(peer_index);
                            },
                            crust::Event::BootstrapAccept(peer_id) => {
                                println!("\nBootstrapAccept with peer {:?}", peer_id);
                                let peer_index = handle_new_peer(&unwrap_result!(service.lock()), network2.clone(), peer_id);
                                let _ = bs_sender.send(peer_index);
                            },
                            crust::Event::NewPeer(Ok(()), peer_id) => {
                                println!("\nConnected to peer {:?}", peer_id);
                                let _ = handle_new_peer(&unwrap_result!(service.lock()), network2.clone(), peer_id);
                            }
                            crust::Event::LostPeer(peer_id) => {
                                println!("\nLost connection to peer {:?}",
                                         peer_id);
                                let mut index = None;
                                {
                                    let network = unwrap_result!(network2.lock());
                                    for (i, id) in network.nodes.iter() {
                                        if id == &peer_id {
                                            index = Some(*i);
                                            break;
                                        }
                                    }
                                }
                                let mut network = unwrap_result!(network2.lock());
                                if let Some(index) = index {
                                    let _ = unwrap_option!(network.nodes.remove(&index), "index should definitely be a key in this map");
                                };
                                network.print_connected_nodes(&unwrap_result!(service.lock()));
                            }
                            e => {
                                println!("\nReceived event {:?} (not handled)", e);
                            }
                        }

                    } else {
                        break;
                    }
                },
                _ => unreachable!("This category should not have been fired - {:?}", it),
            }
        }
    }) {
        Ok(join_handle) => join_handle,
        Err(e) => {
            println!("Failed to start event-handling thread: {}", e);
            std::process::exit(5);
        },
    };


        cmd_parser::print_usage();

        loop {
            use std::io::Write; // For flush().

            print!("> ");
            assert!(io::stdout().flush().is_ok());

            let mut command = String::new();
            assert!(io::stdin().read_line(&mut command).is_ok());

            let cmd = match parse_user_command(command) {
                Some(cmd) => cmd,
                None => continue,
            };

            match cmd {
                UserCommand::PrepareConnectionInfo => {
                    let mut network = unwrap_result!(network.lock());
                    let token = network.next_connection_info_index();
                    unwrap_result!(service.lock()).prepare_connection_info(token);
                }
                UserCommand::Connect(our_info_index, their_info) => {
                    let mut network = unwrap_result!(network.lock());
                    let our_info_index = match u32::from_str(&our_info_index) {
                        Ok(info) => info,
                        Err(e) => {
                            println!("Invalid connection info index: {}", e);
                            continue;
                        },
                    };
                    let our_info = match network.our_connection_infos.remove(&our_info_index) {
                        Some(info) => info,
                        None => {
                            println!("Invalid connection info index");
                            continue;
                        },
                    };
                    let their_info = match json::decode(&their_info) {
                        Ok(info) => info,
                        Err(e) => {
                            println!("Error decoding their connection info");
                            println!("{}", e);
                            continue;
                        },
                    };
                    unwrap_result!(service.lock()).connect(our_info, their_info);
                }
                UserCommand::Send(peer_index, message) => {
                    let network = unwrap_result!(network.lock());
                    match network.get_peer_id(peer_index) {
                        Some(ref mut peer_id) => {
                            unwrap_result!(unwrap_result!(service.lock()).send(peer_id, message.into_bytes()));
                        }
                        None => println!("Invalid connection #"),
                    }
                }
                UserCommand::SendAll(message) => {
                    let mut network = unwrap_result!(network.lock());
                    let msg = message.into_bytes();
                    for (_, peer_id) in network.nodes.iter_mut() {
                        unwrap_result!(unwrap_result!(service.lock()).send(peer_id, msg.clone()));
                    }
                }
                UserCommand::List => {
                    let network = unwrap_result!(network.lock());
                    network.print_connected_nodes(&unwrap_result!(service.lock()));
                }
                /*
                UserCommand::Clean => {
                    let mut network = network.lock().unwrap();
                    network.remove_disconnected_nodes();
                    network.print_connected_nodes();
                }
                */
                UserCommand::Stop => {
                    break;
                }
            }

    }

    drop(service);
    assert!(handler.join().is_ok());
}
