#![allow(dead_code,unused_variables,unused_imports,unused_must_use, unused_mut, unused_assignments)]
extern crate p2p3;
extern crate crust;
extern crate docopt;
extern crate rustc_serialize;
extern crate time;
extern crate git2;
#[macro_use]
extern crate maidsafe_utilities;
extern crate rand;
extern crate getopts;
extern crate ws;
extern crate url;

mod cmd_parser;
use cmd_parser::*;

use std::{thread,env};
use getopts::Options;
use p2p3::utils::p2p3_globals;
use p2p3::woot::static_site::site_singleton;
use p2p3::network::{Message,MessagePasser, MessagePasserT};
use std::io::Write;
use std::io;
use rustc_serialize::json;
use p2p3::network::bootstrap::BootstrapHandler;
use p2p3::storage::storage_helper::GitAccess;
use std::str::FromStr;
use self::crust::PeerId;
use self::rand::random;

#[derive(RustcEncodable,RustcDecodable,Clone,Debug)]
struct Msg(String);

impl Message for Msg{}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("u", "", "URL to the git repo to connect to", "URL");
    opts.optopt("n", "", "Username", "Username");
    opts.optopt("p", "", "Password", "Password");
    opts.optopt("s", "", "Site id", "SiteId");
    opts.optopt("f", "", "File path to clone the git repo", "FilePath");
    opts.optopt("d", "port", "Port number", "PortNumber");
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };
    if matches.opt_present("h") {
        print_usage();
        return;
    };
    let git_url = matches.opt_str("u").unwrap();
    let git_username = matches.opt_str("n").unwrap();
    let git_password = matches.opt_str("p").unwrap();
    let port = matches.opt_str("d").unwrap();
    let port_number = port.parse::<u16>().unwrap();
    let local_path = matches.opt_str("f").unwrap();
    let p = env::current_dir().unwrap();
    let p2p3_url = format!("file://{}/front-end/index.html",p.display());
    let file_path = "c_code.c";
    let git_access = GitAccess::new(git_url.clone(), local_path.clone(), file_path.to_string().clone(), git_username.clone(), git_password.clone());

    {
        let id: PeerId = random();
        let globals = p2p3_globals().inner.clone();
        let mut values = globals.lock().unwrap();
        values.init(id, port_number, p2p3_url.clone(), git_access.clone());
    }
    match git_access.clone_repo() {
        Ok(()) => {},
        Err(e) => {
            println!("The folder already exits");
        },
    };

    println!("Starting bootstrap");
    let boot = BootstrapHandler::bootstrap_load();
    let (mp,_) = MessagePasser::new();
    boot.update_config(mp.clone());
    println!("###############################");
    println!("My id is {:?}", mp.get_id());
    println!("###############################");

    if matches.free.len() > 0 {
        print_usage();
        return;
    };
    let static_site = site_singleton(mp.get_id().clone());

    // Get the four parameters from the front-end.
    // let repo_url: String = "https://github.com/KajoAyame/p2p3_test.git".to_string();
    // let local_url: String = "temp/".to_string();
    // let file_path: String = "file1.rs".to_string();
    // let username: String = "zhou.xinghao.1991@gmail.com".to_string();
    // let password: String = "123456abc".to_string();


    // let git = GitAccess::new(repo_url, local_url, file_path, username, password);
    // match git.clone_repo() {
    //     Ok(()) => (),
    //     Err(e) => {
    //         println!("{}, stop clone and read the config file in that directory", e);
    //     }
    // }
    //
    // println!("Starting bootstrap");
    //
    // // Get file name from the front end.
    // let boot = BootstrapHandler::bootstrap_load(git);
    //
    // // Network
    // print_usage();
    // let (mp,_) = p2p3::network::MessagePasser::new();
    // boot.update_config(mp.clone());
    // println!("###############################");
    // println!("My id is {:?}", mp.get_id());
    // println!("###############################");
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
                let tok = mp.prepare_connection_info();
                let con = mp.wait_conn_info(tok);
                println!("Share this with other client:\n{}",json::encode(&con).unwrap());
            }
            UserCommand::Connect(our_info_index, their_info) => {
                let their_info = unwrap_result!(json::decode(&their_info));
                let index = u32::from_str(our_info_index.as_str()).unwrap();
                mp.connect(index, their_info);
            }
            UserCommand::Send(index, message) => {
                let peers = mp.peers();
                //let index = usize::from_str(peer_index).unwrap();
                mp.send(&peers[index], Msg(message));
            }
            UserCommand::SendAll(message) => {
                mp.broadcast(Msg(message));
            }
            UserCommand::List => {
                mp.print_connected_nodes();
            }
            UserCommand::Broadcast(message) => {
                mp.broadcast(Msg(message));
            }
            UserCommand::Test => {
                println!("Hello");
            }
            UserCommand::Stop => {
                break;
            }
        }
    }
}
