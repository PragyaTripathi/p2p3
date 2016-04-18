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

mod commit;
mod compile;
mod logger;
pub mod network;
mod permission;
pub mod storage;
mod ui;
mod woot;

use std::{thread,env};
use getopts::Options;
use storage::storage_helper::GitAccess;
use woot::static_site::site_singleton;
use woot::operation_thread::run;
use permission::permissions_handler::get_permission_level;
use permission::permissions_handler::PermissionLevel;

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
    let local_path = matches.opt_str("f").unwrap();
    if matches.free.len() > 0 {
        print_usage(&program, opts);
        return;
    };
    let file_path = "permissions.txt";
    let mut git_access = GitAccess::new(git_url.clone(), local_path.clone(), git_username.clone(), git_password.clone());
    let mut static_site = site_singleton(site_id);
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

}
