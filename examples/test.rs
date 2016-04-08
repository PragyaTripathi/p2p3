use std::env;
//use std::path::{Path, PathBuf};
use std::path::*;
use std::ffi::*;

fn main() {
//    let exe_path = env::current_exe();
//    println!("Path of this executable is: {}",
//                              exe_path);

    let exe_path = env::current_exe().unwrap(); /* {
        Ok(exe_path) => println!("Path of this executable is: {}",
                                  exe_path.display()),
        Err(e) => println!("failed to get current exe path: {}", e),
    };*/

    let msg = "1 2 3 4 5 6";

    let mut msgs = msg.trim_right_matches(|c| c == '\r' || c == '\n')
    .split(' ')
    .collect::<Vec<_>>();

    let u: Vec<_> = msgs.drain(2..).collect();

    println!("{:?}", msgs);
    println!("{:?}", u);
    /*
    println!("Path of this executable is: {}",
                              exe_path.display());
    let file_stem = exe_path.file_stem().unwrap();
    println!("file_stem: {}", file_stem.to_str().unwrap());
    let mut name = file_stem.to_os_string();
    name.push(".crust.config");
    println!("name: {}", name.to_str().unwrap());
    */
}
