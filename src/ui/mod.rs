use rustc_serialize::json;
use std::thread;
use ws::{listen, Handler, Sender, Result, Message, Handshake, CloseCode, Error};
use std::result::Result as Res;
use std::sync::mpsc::channel;
use std::sync::{Arc,Mutex};


#[derive(Clone,RustcDecodable,RustcEncodable)]
pub enum Command{
    Insert(u32, String),
    Delete(u32, u32),
    Commit,
}

type FnCommand = Box<Fn(&Command)->Res<String, String> + Send + Sync>;

#[allow(dead_code)]
#[derive(Clone)]
struct UiHandler{
    listeners: Arc<Mutex<Vec<FnCommand>>>,
    out: Arc<Sender>
}

impl Handler for UiHandler {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::Text(txt) => {
                let cmd: Command = json::decode(&txt).unwrap();
                for listener in self.listeners.lock().unwrap().iter() {
                    let res = listener(&cmd);
                    match res{
                        Ok(_)=> {},
                        Err(_)=>{panic!("Oh noooo!")}
                    }
                }
                Ok(())
            },
            Message::Binary(_) => {panic!("What the heck do I do with a binary message.")}
        }
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        match code {
            CloseCode::Normal => println!("The client is done with the connection."),
            CloseCode::Away   => println!("The client is leaving the site."),
            CloseCode::Abnormal => println!(
                "Closing handshake failed! Unable to obtain closing status from client."),
            _ => println!("The client encountered an error: {}", reason),
        }
    }

    fn on_error(&mut self, err: Error) {
        println!("The server encountered an error: {:?}", err);
    }
}

impl UiHandler{
    #[allow(dead_code)]
    fn new(port: u16) -> UiHandler{
        let (tx,rx) = channel::<UiHandler>();
        thread::spawn(move||{
            listen(format!("127.0.0.1:{}",port).as_str(),
                 |out|{
                     let ui = UiHandler {
                         out: Arc::new(out),
                         listeners: Arc::new(Mutex::new(vec!()))};
                     tx.send(ui.clone()).unwrap_or(());
                     ui
                 }).unwrap();
        });
        rx.recv().unwrap()
    }

    #[allow(dead_code)]
    fn add_listener(&self, f: FnCommand){
        self.listeners.lock().unwrap().push(f);
    }

    #[allow(dead_code)]
    fn send_command(&self, cmd: Command){
        self.out.send(Message::Text(json::encode(&cmd).unwrap())).unwrap();
    }
}
