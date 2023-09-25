mod ipc;
mod saws;
mod systemlock;
mod util;

use saws::Msg;

use std::str;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;


#[cfg(debug_assertions)]
macro_rules! dbgprint {
    ($( $args:expr ),*) => { println!($( $args),* ) }
}

#[cfg(not(debug_assertions))]
macro_rules! dbgprint {
    ($( $args:expr ),*) => { }
}


// ----------------------------
// vvv ipc helper functions vvv

fn write_cp_clients(clients: &Vec<CPClient>) -> Result<()> {
    for c in clients {
        let delin_id = c.id() + str::from_utf8(&[0])?; // known to be valid utf8
        ipc::write("cp_clients", &delin_id)?;
    }
    Ok(())
}

fn rewrite_cp_clients(clients: &Vec<CPClient>) -> Result<()> {
    ipc::consume("cp_clients")?;
    write_cp_clients(clients)?;
    Ok(())
}

fn read_msgs_for_client(id: String) -> Result<Vec<String>> {
    let mut ret: Vec<String> = Vec::new();
    let ipc_name = id + "_out";
    let msgs_string = ipc::consume(&ipc_name)?;
    if msgs_string.len() == 0 {
        return Ok(vec![]);
    }
    let mut parts = msgs_string.split(str::from_utf8(&[0])?).collect::<Vec<&str>>();
    parts.pop(); // there will be nothing after last null byte
    for p in &parts {
        ret.push(String::from(*p));
    }
    Ok(ret)
}

fn write_msgs_from_client(id: String, msgs: Vec<String>) -> Result<()> {
    let mut s = String::new();
    for m in msgs {
        s += &m;
        s += str::from_utf8(&[0])?;
    }
    let ipc_name = id + "_in";
    ipc::write(&ipc_name, &s)?;
    Ok(())
}

// ^^^ ipc helper functions ^^^
// ----------------------------


// --------------------------
// vvv Control Pad Server vvv

struct CPClient {
    id: String,
    conn: saws::Conn,
}

impl CPClient {
    fn new(conn: saws::Conn) -> Self {
        let addr = conn.addr();
        let ip = addr.split(":").next().unwrap();
        let id_bytes = ip.split(".").collect::<Vec<&str>>();
        let id = id_bytes[2..4].join("x") + "-0";
        CPClient {
            id: id,
            conn: conn,
        }
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn is_dead(&self) -> bool {
        self.conn.is_dead()
    }

    fn update_subid(&mut self, subid: u8) {
        let base_id = self.id.split("-").collect::<Vec<&str>>()[0];
        self.id = base_id.to_string() + "-" + &subid.to_string();
    }

}

struct CPServer {
    server: saws::Server,
    clients: Vec<CPClient>,
    
}

impl CPServer {
    fn new(port: &str) -> Self {
        CPServer {
            server: saws::Server::new(port).unwrap(), // fatal
            clients: vec![],
        }
    }

    // create websockets based on inbound websocket creation requests and
    // update the cp_clients ipc object with the list of currently connected clients
    pub fn accept_new_clients(&mut self) {
        let new_conns = self.server.new_connections();
        if new_conns.len() == 0 {
            return;
        }
        let mut new_clients = new_conns.into_iter()
            .map(|c| CPClient::new(c)).collect::<Vec<CPClient>>();
        write_cp_clients(&new_clients)
            .expect("Failure writing to cp_clients");
        self.clients.append(&mut new_clients);
        dbgprint!("clients: {:?}", self.clients.iter()
                  .map(|x| x.id()).collect::<Vec<String>>());
    }

    // For websockets that have died, remove the CPClient from our list and
    // update the cp_clients ipc object to reflect that
    pub fn clear_dead_clients(&mut self) {
        let old_len = self.clients.len();
        self.clients.retain(|x| ! x.is_dead());
        if self.clients.len() == old_len {
            return;
        }
        dbgprint!("clients: {:?}", self.clients.iter()
                  .map(|x| x.id()).collect::<Vec<String>>());
        rewrite_cp_clients(&self.clients)
            .expect("Failure rewriting cp_clients");
    }

    // for each "_out" ipc object that has new messages, send those messages
    // over websocket to the associated client
    pub fn send_messages_to_clients(&mut self) {
        for c in &mut self.clients {
            let msgs = read_msgs_for_client(c.id())
                .expect("Failure reading ipc from target");
            for m in msgs {
                dbgprint!("-> {}: '{}'", c.id(), m);
                c.conn.send_msg(Msg::Text(m));
            }
        }
    }

    // for each websocket that had new messages, write those messages to the
    // associated  "_in" ipc object
    pub fn recv_messages_for_target(&mut self) {
        let mut new_subids = false;
        for c in &mut self.clients {
            let msgs = c.conn.get_recved_msgs();
            let mut tmsgs = Vec::<String>::new();
            let mut subid: Option<u8> = None;
            for m in msgs {
                match m {
                    Msg::Text(t) => {
                        dbgprint!("<- {}: '{}'", &c.id(), &t);
                        tmsgs.push(t);
                    }
                    Msg::Bytes(v) => {
                        dbgprint!("<- {}: {:?}", &c.id(), &v);
                        if v.len() > 0 {
                            subid = Some(v[0]);
                        } 
                        if v.len() != 1 {
                            println!("Warning: invalid subid: {:?}", v);
                        }
                    }
                }
            }
            if let Some(n) = subid {
                c.update_subid(n);
                new_subids = true;
            }
            
            if tmsgs.len() != 0 {
                
                write_msgs_from_client(c.id(), tmsgs)
                    .expect("Failure writing ipc to target");
            }
        }

        if new_subids {
            dbgprint!("clients: {:?}", self.clients.iter()
                      .map(|x| x.id()).collect::<Vec<String>>());
            rewrite_cp_clients(&self.clients)
                .expect("Failure rewriting cp_clients");
        } 
    }

    // Out: whether or not all clients should be reloaded
    pub fn read_reload(&mut self) -> Result<bool> {
        let ipc_name = "rpc_out";
        //read here
        let rpc_contents = ipc::consume(&ipc_name)?;

        if rpc_contents.len() == 0 {
            return Ok(false);
        }
        
        let mut parts = rpc_contents.split(str::from_utf8(&[0])?).collect::<Vec<&str>>();
        parts.pop();

        for message in parts {
            if message == "reload" {
                return Ok(true)
            }
        }
        return Ok(false);
    }


    pub fn send_reloads_to_clients(&mut self)  {
        let should_reload = self.read_reload().unwrap_or_else( |e| {
            panic!("Failed to read reload message with error {}", e);
        });

        if should_reload {
            // go through clients and send vec![0x1]
            for c in &mut self.clients {
                c.conn.send_msg(Msg::Bytes(vec![0x1]));
                println!("message sent!");
            }
        }
    }
}

// ^^^ Control Pad Server ^^^
// --------------------------



fn main() {

    // do not allow runnning as root (this check only works on windows)
    if let Ok(env_var) = std::env::var("USER") {
        if env_var.eq("root") {
            println!("ERROR: You must not run the controlpad server as root");
            std::process::exit(1);
        }
    }
    // TODO: do we need to do an admin check for Windows?^^^
    
    // create expected directories for various modules
    ipc::initialize();
    systemlock::initialize();
    
    // start server
    let mut cpserver = CPServer::new("50079");
    loop {
        cpserver.accept_new_clients();
        cpserver.send_messages_to_clients();
        cpserver.recv_messages_for_target();
        cpserver.clear_dead_clients();
        cpserver.send_reloads_to_clients();
        std::thread::sleep(std::time::Duration::from_micros(1500));
    }
}

