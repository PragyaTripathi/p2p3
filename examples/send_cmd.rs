extern crate p2p3;

use p2p3::ui::{UiHandler, Command};
use std::io::stdin;

fn main(){
    let ui = UiHandler::new(4242);
    for i in 0..10{
        ui.send_command(Command::Insert(i,"Hello there".to_string()));
    }
    println!("Connection with front-end initialized.");
    let mut x = String::new();
    stdin().read_line(&mut x).unwrap();
}
