extern crate p2p3;
extern crate rustc_serialize;
extern crate docopt;
#[macro_use]
extern crate maidsafe_utilities; // macro unwrap!()

use rustc_serialize::{Decodable, Decoder, json};
use docopt::Docopt;
use std::io;
use p2p3::network::cmd_parser;

fn main() {
    cmd_parser::print_usage();
    loop {
        use std::io::Write; // For flush().

        print!("> ");
        assert!(io::stdout().flush().is_ok());

        let mut command = String::new();
        assert!(io::stdin().read_line(&mut command).is_ok());

        let cmd = match cmd_parser::parse_user_command(command) {
            Some(cmd) => cmd,
            None => continue,
        };
        println!("{:?}", cmd);
    }
}
