extern crate crust;
use woot::operation::Operation;
use network::Message;
use crust::PeerId;

#[derive(RustcEncodable,RustcDecodable, Clone, Debug)]
pub enum Msg{
    String(String),
    // row, col
    Cursor(PeerId, u32, u32),
    WootOperation(Operation)
}

impl Message for Msg{}
