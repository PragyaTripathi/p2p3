extern crate p2p3;
extern crate rustc_serialize;
#[macro_use]
extern crate maidsafe_utilities;

extern crate crust;
use crust::ConnectionInfoResult;

use std::io::Write;
use std::io;
use rustc_serialize::json;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use p2p3::network::network_manager::Network;
use p2p3::network::network_manager::handle_new_peer;
use p2p3::network::cmd_parser;
use p2p3::network::cmd_parser::UserCommand;
use p2p3::network::cmd_parser::parse_user_command;
use p2p3::network::MessagePasser;
use p2p3::network::MessagePasserT;
use p2p3::network::bootstrap::BootstrapHandler;
use p2p3::storage::storage_helper::GitAccess;
use std::thread;
use std::str::FromStr;

fn main() {

    // Get the four parameters from the front-end.
    let repo_url: String = "https://github.com/KajoAyame/p2p3_test.git".to_string();
    let local_url: String = "temp/".to_string();
    let username: String = "zhou.xinghao.1991@gmail.com".to_string();
    let password: String = "123456abc".to_string();


    let git = GitAccess::new(repo_url, local_url, username, password);
    match git.clone_repo() {
        Ok(()) => (),
        Err(e) => {
            println!("{}, stop clone and read the config file in that directory", e);
        }
    }

    println!("Starting bootstrap");

    // Get file name from the front end.
    let file_name: String = "file1.p2p3".to_string(); // Hardcode the file name.
    let mut boot = BootstrapHandler::bootstrap_load(git, file_name);


    // Network
    cmd_parser::print_usage();
    let mp = p2p3::network::MessagePasser::new(boot);

    mp.prepare_connection_info();
    loop {
        print!("> ");
        assert!(io::stdout().flush().is_ok());

        // Get the command line from user input
        let mut command = String::new();
        assert!(io::stdin().read_line(&mut command).is_ok());

        let cmd = match parse_user_command(command) {
            Some(cmd) => cmd,
            None => continue,
        };

        match cmd {
            UserCommand::PrepareConnectionInfo => {
                mp.prepare_connection_info();
            }
            UserCommand::Connect(our_info_index, their_info) => {
                let index = u32::from_str(our_info_index.as_str()).unwrap();
                mp.connect(index, their_info);
            }
            UserCommand::Send(index, message) => {
                let peers = mp.peers();
                //let index = usize::from_str(peer_index).unwrap();
                mp.send(peers[index], message);
            }
            UserCommand::SendAll(message) => {
                mp.broadcast(message);
            }
            UserCommand::List => {
                let peers = mp.peers();
                let service_am = mp.get_service();
                let service = unwrap_result!(service_am.lock());
                let mut i = 0;
                for peer in peers.iter(){
                    if let Some(conn_info) = service.connection_info(peer) {
                        println!("    [{}] {}   {} <--> {} [{}][{}]",
                                 i, peer, conn_info.our_addr, conn_info.their_addr, conn_info.protocol,
                                 if conn_info.closed { "closed" } else { "open" }
                        );
                    }
                    i+=1;
                }
            }
            UserCommand::Broadcast(message) => {
                mp.broadcast(message);
            }
            UserCommand::Test => {
                println!("Hello");
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

    drop(mp);

}
