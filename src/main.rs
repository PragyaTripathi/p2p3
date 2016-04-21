#![allow(dead_code,unused_variables,unused_imports)]

extern crate crust;
extern crate time;
extern crate git2;
#[macro_use]
extern crate maidsafe_utilities;
extern crate rustc_serialize;
extern crate docopt;
extern crate rand;
extern crate getopts;
extern crate ws;
extern crate url;

mod commit;
mod compile;
mod logger;
pub mod network;
mod permission;
pub mod storage;
mod ui;
mod woot;
mod utils;

use std::{thread,env};
use getopts::Options;
use storage::storage_helper::GitAccess;
use woot::static_site::site_singleton;
use woot::operation_thread::run;
use permission::permissions_handler::get_permission_level;
use permission::permissions_handler::PermissionLevel;
use compile::run_c;
use ui::{UiHandler, Command, FnCommand, open_url, static_ui_handler};
use utils::p2p3_globals;
use std::io::stdin;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} FILE [options]", program);
    print!("{}", opts.usage(&brief));
}

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
        print_usage(&program, opts);
        return;
    };
    let git_url = matches.opt_str("u").unwrap();
    let git_username = matches.opt_str("n").unwrap();
    let git_password = matches.opt_str("p").unwrap();
    let site_id_str = matches.opt_str("s").unwrap();
    let site_id = site_id_str.parse::<u32>().unwrap();
    let port = matches.opt_str("d").unwrap();
    let port_number = port.parse::<u16>().unwrap();
    let local_path = matches.opt_str("f").unwrap();
    let p = env::current_dir().unwrap();
    let p2p3_url = format!("file://{}/front-end/index.html",p.display());

    let mut globals = p2p3_globals();
    globals.init_globals(site_id, port_number, p2p3_url);
    if matches.free.len() > 0 {
        print_usage(&program, opts);
        return;
    };
    let file_path = "c_code.c";
    let git_access = GitAccess::new(git_url.clone(), local_path.clone(), git_username.clone(), git_password.clone());
    let static_site = site_singleton(globals.get_site_id());
    match git_access.clone_repo(&local_path) {
        Ok(()) => {},
        Err(e) => {
            println!("The folder already exits");
        },
    };

    let permission_level = get_permission_level(&git_access);
    match permission_level {
        PermissionLevel::Editor => println!("The user is an editor"),
        PermissionLevel::Viewer => println!("The user is a viewer"),
    };
    let operation_thread = thread::spawn(move || {
        run(site_id);
    });
    let file_name = &(local_path+file_path);
    // Initialize editor content
    {
        let initial_file_content = read_file(file_name);
        let site_clone = static_site.inner.clone();
        let mut site = site_clone.lock().unwrap();
        site.parse_given_string(&initial_file_content);
    }

    let static_ui = static_ui_handler(globals.get_port(), globals.get_url());
    fn recieve_commands() -> FnCommand {
        Box::new(|comm| {
            match comm {
                Compile => {
                    // need site and ui from environment TODO
                    let p2p3_globals = p2p3_globals().clone();
                    let site_clone = site_singleton(p2p3_globals.get_site_id()).inner.clone();
                    let mut site = site_clone.lock().unwrap();
                    let ui_clone = static_ui_handler(p2p3_globals.get_port(), p2p3_globals.get_url()).inner.clone();
                    let mut ui = ui_clone.lock().unwrap();
                    match run_c(&site.content()){
                        Ok(o) => ui.send_command(Command::Output(o)),
                        //Ok(o) => ui.send_command(Command::Output(o)), TODO
                        Err(e) => println!("error {}", e),
                    };
                },
            }
            Ok("".to_string())
        })
    };
    let command_func = recieve_commands();
    {
        let ui_inner = static_ui.inner.clone();
        let ui = ui_inner.lock().unwrap();
        ui.add_listener(command_func);
        let mut content = String::new();
        {
            let site_clone = static_site.inner.clone();
            let mut site = site_clone.lock().unwrap();
            let mut borrowed_content = &mut content;
            *borrowed_content = site.content();
        }
        ui.send_command(Command::Insert(0, content));
    }
    println!("Connection with front-end initialized.");
    let mut x = String::new();
    stdin().read_line(&mut x).unwrap();
}

fn read_file(url: &str) -> String {
    let path = Path::new(url);
    let mut file = match File::open(&path) {
        Err(_) => panic!("could not open {}", url),
        Ok(file) => file,
    };
    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(_) => panic!("Could not read"),
        Ok(_) => return s,
    }
}

fn init_editor(initial_content: &str, ui: &UiHandler) {
    ui.send_command(Command::Insert(0, initial_content.to_string()));
}
