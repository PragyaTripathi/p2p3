extern crate config_file_handler;
extern crate crust;
extern crate socket_addr;

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::prelude::*;
use ::storage::storage_helper::GitAccess;
use self::crust::{TheirConnectionInfo,StaticContactInfo};
use self::socket_addr::SocketAddr;
use rustc_serialize::json;
use rustc_serialize::json::Json;
use rustc_serialize::json::as_pretty_json;
use super::{MessagePasser,Message};
use utils::p2p3_globals;

#[derive(PartialEq, Eq, Debug, RustcDecodable, RustcEncodable, Clone)]
pub struct Config {
    pub hard_coded_contacts: Vec<StaticContactInfo>,
    pub enable_tcp: bool,
    pub enable_utp: bool,
    pub tcp_acceptor_port: Option<u16>,
    pub utp_acceptor_port: Option<u16>,
    pub udp_mapper_servers: Vec<SocketAddr>,
    pub tcp_mapper_servers: Vec<SocketAddr>,
    pub service_discovery_port: Option<u16>,
    pub bootstrap_cache_name: Option<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            hard_coded_contacts: vec![], // No hardcoded endpoints
            enable_tcp: true,
            enable_utp: true,
            tcp_acceptor_port: None,
            utp_acceptor_port: None,
            udp_mapper_servers: vec![],
            tcp_mapper_servers: vec![],
            service_discovery_port: None,
            bootstrap_cache_name: None,
        }
    }
}

#[derive(Clone)]
pub struct BootstrapHandler {
    pub config: Config,
    pub config_file: String,
    pub full_path: String
}

impl BootstrapHandler {
    pub fn bootstrap_load() -> BootstrapHandler{
        // Load the p2p3 config file in the same directory of the working file.
        let mut git_local_url = String::new();
        {
            let globals = p2p3_globals().inner.clone();
            let mut values = globals.lock().unwrap();
            git_local_url = values.get_git_access().local_url.clone();
        }

        let p2p3_file_name = String::from("config.p2p3");
        let p2p3_file_url = git_local_url.clone() + &p2p3_file_name;
        println!("git_local_url: {} p2p3_file_url: {}", git_local_url, p2p3_file_url.clone());
        let mut p2p3_file = File::open(p2p3_file_url.clone()).unwrap();
        let mut file_str = String::new();
        p2p3_file.read_to_string(&mut file_str).unwrap();

        // Construct the Config object.
        let conf: Config = json::decode(&file_str).unwrap();

        BootstrapHandler {
            config: conf,
            config_file: p2p3_file_name,
            full_path: p2p3_file_url,
        }
    }

    fn static_info_from_their(their_info: TheirConnectionInfo) -> StaticContactInfo{
        let info_json = unwrap_result!(json::encode(&their_info));
        let data = Json::from_str(info_json.as_str()).unwrap();
        let obj = data.as_object().unwrap();
        let foo = obj.get("static_contact_info").unwrap();

        let json_str: String = foo.to_string();

        let info: StaticContactInfo = json::decode(&json_str).unwrap();
        info
    }

    pub fn update_config<T:Message>(&self, mp: MessagePasser<T>) {
        let tok = mp.prepare_connection_info();
        let their_info = mp.wait_conn_info(tok);
        let mut info = BootstrapHandler::static_info_from_their(their_info);
        info.tcp_acceptors.remove(0);

        /*
         *  Because the crust can only connect to the first TCP acceptor in the config file,
         *  we need to insert the new node's info in the first position.
         */
        let mut boot_clone = self.clone();
        boot_clone.config.hard_coded_contacts[0].tcp_acceptors.insert(0, info.tcp_acceptors[0]);
        let update_str = as_pretty_json(&boot_clone.config);
        let pretty_json_str = update_str.to_string();

        let mut git_access = GitAccess::default();
        {
            let globals = p2p3_globals().inner.clone();
            let mut values = globals.lock().unwrap();
            git_access = values.get_git_access();
        }

        // Get the p2p3 config file path and store the new config infomation it in that path.
        let path_str = &self.full_path;
        let path = Path::new(&path_str);
        let mut file = File::create(path.clone()).unwrap();
        let file_byte = pretty_json_str.into_bytes();
        file.write_all(&file_byte).unwrap();

        println!("commiting {} ", &self.config_file);
        match git_access.commit_config("Update config file.", &self.config_file) {
            Ok(()) => (),
            Err(e) => {
                println!("Commit error: {}", e);
            }
        }

        match git_access.push() {
            Ok(()) => (),
            Err(e) => {
                println!("Push error: {}", e);
            }
        }
    }
}

/*
 *  file.suffix -> file.crust.config
 */
pub fn get_crust_config() -> Result<::std::ffi::OsString, self::crust::Error> {
    let mut name = try!(config_file_handler::exe_file_stem());
    name.push(".crust.config");
    Ok(name)
}

/*
 *  file.suffix -> file.p2p3
 */
pub fn get_p2p3_config(file_name: &String) -> String {
    let path = PathBuf::from(file_name);
    let mut name = path.file_stem().unwrap().to_os_string();
    name.push(".p2p3");
    let result = match name.into_string() {
        Ok(e) => e,
        Err(e) => {panic!("Couldn't make .p2p3 file for {} because {:?}", file_name, e);}
    };
    result
}
