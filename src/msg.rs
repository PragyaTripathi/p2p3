use woot::operation::Operation;
use network::Message;

#[derive(RustcEncodable,RustcDecodable, Clone, Debug)]
pub enum Msg{
    String(String),
    // row, col
    Cursor(u32, u32),
    WootOperation(Operation)
}

impl Message for Msg{}
