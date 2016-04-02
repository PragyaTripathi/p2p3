/*
    run "cargo run --example network" in different terminals to test the network module.
*/

extern crate rustc_serialize;
extern crate docopt;
#[macro_use]
extern crate maidsafe_utilities; // macro unwrap!()
//
//extern crate term;
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
use std::thread;
use std::str::FromStr;




fn main() {
//    unwrap_result!(maidsafe_utilities::log::init(true));

    let args: Args = Docopt::new(USAGE)
                         .and_then(|docopt| docopt.decode())
                         .unwrap_or_else(|error| error.exit());

//    let mut stdout = stdout();
//    let mut stdout_copy = stdout();

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

    if args.flag_disable_tcp {
        config.enable_tcp = false;
        config.tcp_acceptor_port = None;
    }
    if args.flag_disable_utp {
        config.enable_utp = false;
        config.utp_acceptor_port = None;
    }

    config.service_discovery_port = args.flag_discovery_port;

    let mut service = unwrap_result!(Service::with_config(event_sender, &config));
    if !args.flag_disable_tcp {
        unwrap_result!(service.start_listening_tcp());
    }
    if !args.flag_disable_utp {
        unwrap_result!(service.start_listening_utp());
    }
    service.start_service_discovery();
    let service = Arc::new(Mutex::new(service));
    let service_cloned = service.clone();

    let network = Arc::new(Mutex::new(Network::new()));
    let network2 = network.clone();

    // Start event-handling thread
    let running_speed_test = args.flag_speed.is_some();

    let handler = match thread::Builder::new().name("CrustNode event handler".to_string())
                                              .spawn(move || {
        let service = service_cloned;
        for it in category_rx.iter() {
            match it {
                ::maidsafe_utilities::event_sender::MaidSafeEventCategory::Crust => {
                    if let Ok(event) = channel_receiver.try_recv() {
                        match event {
                            crust::Event::NewMessage(peer_id, bytes) => {
                            //    stdout_copy = cyan_foreground(stdout_copy);
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
                            //    stdout_copy = cyan_foreground(stdout_copy);
                                println!("\nBootstrapConnect with peer {:?}", peer_id);
                                let peer_index = handle_new_peer(&unwrap_result!(service.lock()), network2.clone(), peer_id);
                                let _ = bs_sender.send(peer_index);
                            },
                            crust::Event::BootstrapAccept(peer_id) => {
                            //    stdout_copy = cyan_foreground(stdout_copy);
                                println!("\nBootstrapAccept with peer {:?}", peer_id);
                                let peer_index = handle_new_peer(&unwrap_result!(service.lock()), network2.clone(), peer_id);
                                let _ = bs_sender.send(peer_index);
                            },
                            crust::Event::NewPeer(Ok(()), peer_id) => {
                            //    stdout_copy = cyan_foreground(stdout_copy);
                                println!("\nConnected to peer {:?}", peer_id);
                                let _ = handle_new_peer(&unwrap_result!(service.lock()), network2.clone(), peer_id);
                            }
                            crust::Event::LostPeer(peer_id) => {
                            //    stdout_copy = yellow_foreground(stdout_copy);
                                println!("\nLost connection to peer {:?}",
                                         peer_id);
                            //    stdout_copy = cyan_foreground(stdout_copy);
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

                    //    stdout_copy = reset_foreground(stdout_copy);
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
    //        stdout = red_foreground(stdout);
            println!("Failed to start event-handling thread: {}", e);
    //        let _ = reset_foreground(stdout);
            std::process::exit(5);
        },
    };


        print_usage();

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



static CLI_USAGE: &'static str = "
Usage:
  cli prepare-connection-info
  cli connect <our-info-id> <their-info>
  cli send <peer> <message>...
  cli send-all <message>...
  cli list
  cli clean
  cli stop
  cli help
";

fn print_usage() {
    static USAGE: &'static str = r#"\
# Commands:
    prepare-connection-info                       - Prepare a connection info
    connect <our-info-id> <their-info>            - Initiate a connection to the remote peer
    send <peer> <message>                         - Send a string to the given peer
    send-all <message>                            - Send a string to all connections
    list                                          - List existing connections and UDP sockets
    stop                                          - Exit the app
    help                                          - Print this help
# Where
    <our-file>      - The file where we'll read/write our connection info
    <their-file>    - The file where we'll read their connection info.
    <connection-id> - ID of a connection as listed using the `list` command
"#;
    println!("{}", USAGE);
}


#[derive(RustcDecodable, Debug)]
struct CliArgs {
    cmd_prepare_connection_info: bool,
    cmd_connect: bool,
    cmd_send: bool,
    cmd_send_all: bool,
    cmd_list: bool,
    //cmd_clean: bool,
    cmd_stop: bool,
    cmd_help: bool,
    arg_peer: Option<usize>,
    arg_message: Vec<String>,
    arg_our_info_id: Option<String>,
    arg_their_info: Option<String>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
enum UserCommand {
    Stop,
    PrepareConnectionInfo,
    Connect(String, String),
    Send(usize, String),
    SendAll(String),
    List,
    //Clean,
}

fn parse_user_command(cmd: String) -> Option<UserCommand> {
    let docopt: Docopt = Docopt::new(CLI_USAGE).unwrap_or_else(|error| error.exit());

    let mut cmds = cmd.trim_right_matches(|c| c == '\r' || c == '\n')
                      .split(' ')
                      .collect::<Vec<_>>();

    cmds.insert(0, "cli");

    let args: CliArgs = match docopt.clone().argv(cmds.into_iter()).decode() {
        Ok(args) => args,
        Err(error) => {
            match error {
                docopt::Error::Decode(what) => println!("{}", what),
                _ => println!("Invalid command."),
            };
            return None;
        }
    };

    if args.cmd_connect {
        let our_info_id = unwrap_option!(args.arg_our_info_id, "Missing our_info_id");
        let their_info = unwrap_option!(args.arg_their_info, "Missing their_info");
        Some(UserCommand::Connect(our_info_id, their_info))
    } else if args.cmd_send {
        let peer = unwrap_option!(args.arg_peer, "Missing peer");
        let msg = args.arg_message.join(" ");
        Some(UserCommand::Send(peer, msg))
    } else if args.cmd_send_all {
        let msg = args.arg_message.join(" ");
        Some(UserCommand::SendAll(msg))
    } else if args.cmd_prepare_connection_info {
        Some(UserCommand::PrepareConnectionInfo)
    } else if args.cmd_list {
        Some(UserCommand::List)
    } /* else if args.cmd_clean {
        Some(UserCommand::Clean)
    } */ else if args.cmd_stop {
        Some(UserCommand::Stop)
    } else if args.cmd_help {
        print_usage();
        None
    } else {
        None
    }
}



///////
static USAGE: &'static str = "
Usage:
  crust_peer [options]

The crust peer will run, using any \
                              config file it can find to try and bootstrap
off any provided \
                              peers.  Locations for the config file are specified at
\
                              http://maidsafe.net/crust/master/crust/file_handler/struct.\
                              FileHandler.html#method.read_file

An example of a config file can \
                              be found at
\
                              https://github.com/maidsafe/crust/blob/master/installer/sample.\
                              config
This could be copied to the \"target/debug/examples\" \
                              directory of this project
for example (assuming a debug build) and \
                              modified to suit.

If a config file can't be located or it contains \
                              no contacts, or if connecting
to all of the peers fails, the UDP \
                              beacon will be used.

If no beacon port is specified in the config \
                              file, port 5484 will be chosen.

If no listening ports are \
                              supplied, a random port for each supported protocol
will be chosen.

\
                              Options:
  --discovery-port=PORT      Set the port for local network service discovery
  --disable-tcp              Disable tcp
  --disable-utp              Disable utp
  -s RATE, --speed=RATE      Keep sending random \
                              data at a maximum speed of RATE
                             \
                              bytes/second to the first connected peer.
  -h, --help                 \
                              Display this help message.
";

#[derive(RustcDecodable, Debug)]
struct Args {
    flag_discovery_port: Option<u16>,
    flag_speed: Option<u64>,
    flag_disable_tcp: bool,
    flag_disable_utp: bool,
    flag_help: bool,
}
