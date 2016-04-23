extern crate config_file_handler;
extern crate crust;
extern crate socket_addr;

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::prelude::*;
use ::storage::storage_helper::GitAccess;
use self::crust::StaticContactInfo;
use self::socket_addr::SocketAddr;
use rustc_serialize::json;



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
    pub git: GitAccess,
    pub file_name: String
}

impl BootstrapHandler {
    pub fn bootstrap_load(git: GitAccess) -> BootstrapHandler{
        let p2p3_file_name: String = get_p2p3_config(&git.file_url);
        let url = git.local_url.clone() + &p2p3_file_name;
        let mut file = File::open(url).unwrap();
        let mut file_str = String::new();
        file.read_to_string(&mut file_str).unwrap();
        // Get the config file path
        let file_name = get_crust_config().unwrap().into_string().unwrap();
        let path_str = "target/debug/".to_string() + &file_name; // "target/debug/" in stead of "/target/debug/"

        // Store it in the path
        let path = Path::new(&path_str);
        let mut f = File::create(path.clone()).unwrap();

        let file_byte = file_str.into_bytes();
        f.write_all(&file_byte).unwrap();

        // Read it
        let mut f = File::open(path).unwrap();
        let mut config_str = String::new();
        f.read_to_string(&mut config_str).unwrap();

        // Read it into Config

        let conf: Config = json::decode(&config_str).unwrap();

        BootstrapHandler {
            config: conf,
            git: git,
            file_name: p2p3_file_name
        }
    }

    pub fn update_config(&self, info: StaticContactInfo) {
        let mut boot_clone = self.clone();
        boot_clone.config.hard_coded_contacts[0].tcp_acceptors.insert(0, info.tcp_acceptors[0]);
        let update_str = json::encode(&boot_clone.config).unwrap();

        // Get the config file path
        let path_str = self.git.local_url.clone() + &self.file_name; // "target/debug/" in stead of "/target/debug/"

        // Store it in the path
        let path = Path::new(&path_str);
        let mut file = File::create(path.clone()).unwrap();

        let file_byte = update_str.into_bytes();
        file.write_all(&file_byte).unwrap();

        let p2p3_config_file = get_p2p3_config(&self.file_name);
        match self.git.commit_config("Update config file.", &self.file_name) {
            Ok(()) => (),
            Err(e) => {
                println!("Commit error: {}", e);
            }
        }

        match self.git.push() {
            Ok(()) => (),
            Err(e) => {
                println!("Push error: {}", e);
            }
        }
    }
}


pub fn get_crust_config() -> Result<::std::ffi::OsString, self::crust::Error> {
    let mut name = try!(config_file_handler::exe_file_stem());
    name.push(".crust.config");
    Ok(name)
}


pub fn get_p2p3_config(file_name: &String) -> String {
    let path = PathBuf::from(file_name);
    let mut name = path.file_stem().unwrap().to_os_string();
    name.push(".p2p3");
    let result = match name.into_string() {
        Ok(e) => e,
        Err(e) => file_name.to_string()
    };
    result
}
