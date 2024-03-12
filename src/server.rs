//==================================<===|===>=================================//
mod ipc;
mod saws;
mod systemlock;
mod util;
mod animal_names;

use saws::Msg;
use animal_names::{NUM_ANIMAL_NAMES, ANIMAL_NAMES};

use std::{str, collections::HashMap};
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

//================================= Constants ================================//
// note: The term RPC is used loosely here. In this context it just means parts
//       of the controlpad message passing protocol that

// RPC without arguments should always be 2 bytes
const RPC_QUIT: &[u8] = &[0x99, 0x99];
const RPC_GETQR: &[u8] = &[0x98, 0x98];

// RPC with a single vector of variable length data should always use a three
// byte header
//const RPC_EXAMPLE_HEADER: &[u8] = &[0xA0, 0xA0, 0xA0];


//================================== Helpers =================================//
// assign an animal for a new client
fn get_assigned_name(cp_number: u64) -> String {
    let suffix_num = cp_number/NUM_ANIMAL_NAMES;
    let suffix = if suffix_num == 0 { "".to_string() } else { suffix_num.to_string() };
    let animal_index = cp_number % NUM_ANIMAL_NAMES;
    let prefix = ANIMAL_NAMES[animal_index as usize];
    format!("{}{}", prefix, suffix)
}

// get the last two bytes of ip address from socket for identification
fn sawket_id_base(sawk: &saws::Sawket) -> String {
    let addr = sawk.addr();
    let ip = addr.split(":").next().unwrap();
    let id_bytes = ip.split(".").collect::<Vec<&str>>();
    return id_bytes[2..4].join("x");
}

// add to the list of connected clients
fn write_cp_client(client: &CPClient) -> Result<()> {
    let delin_id = client.id() + str::from_utf8(&[0])?; // known to be valid utf8
    ipc::write("cp_clients", &delin_id)?;
    Ok(())
}

// update the list of connected clients
fn rewrite_cp_clients(clients: &Vec<CPClient>) -> Result<()> {
    ipc::consume("cp_clients")?;
    for c in clients {
        write_cp_client(c)?;
    }
    Ok(())
}

// read outbound messages from the game destined for client with id
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

// write inbound messages from the client with id for the game to receive
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

// write GameNite protocol messages for SystemApps to handle
fn write_rpc_message(data: &Vec<u8>) -> Result<()> {
    let ipc_name = "rpc_in";
    if *data == RPC_QUIT {
        let s = "quit".to_string() + str::from_utf8(&[0])?;        
        ipc::write(ipc_name, &s)?;        
    } else if *data == RPC_GETQR {
        let s = "getqr".to_string() + str::from_utf8(&[0])?;        
        ipc::write(ipc_name, &s)?;
    } else {
        println!("Warning: invalid rpc message: {:?}", data);
    }
    //
    Ok(())
}

// handle GameNite protocol messages (passed as byte vector on sawkets)
fn handle_bytes_from_client(data: &Vec<u8>) {
    if data.len() == 2 {
        write_rpc_message(data)
            .unwrap_or_else(|e| println!("Warning: Failed to write rpc message \
                                          with error: {}", e));
        return;
    } else {
        println!("Warning: received byte vector less than 2 bytes long: {:?}", data);
    }
}


//================================= CPClient =================================//
struct CPClient {
    id: String,
    sawkets: Vec<saws::Sawket>,
}

impl CPClient {
    fn new(sawket: saws::Sawket, subid: u8) -> Self {
        let id_base = sawket_id_base(&sawket);
        let id = id_base + "-" + &subid.to_string();
        let mut sawkets = Vec::new();
        sawkets.push(sawket);
        CPClient {
            id,
            sawkets,
        }
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn send_msg(&mut self, msg: String) {
        for sawk in &mut self.sawkets {
            sawk.send_msg(Msg::Text(msg.clone()));
        }
    }

    fn recv_msgs(&mut self) -> Vec<Msg> {
        let mut msgs = Vec::new();
        for sawk in &mut self.sawkets {
            msgs.append(&mut sawk.recv_msgs());
        }
        return msgs;
    }

    fn is_dead(&self) -> bool {
        for sawket in &self.sawkets {
            if !sawket.is_dead() {
                return false;
            }
        }
        return true;
    }

    fn clear_dead_sawkets(&mut self) {
        self.sawkets.retain(|sawket| !sawket.is_dead());
    }

    fn add_sawket(&mut self, sawk: saws::Sawket) {
        self.sawkets.push(sawk);
    }
}

struct CPDescriptor {
    cp_number: u64,
    name: String,
}

impl CPDescriptor {
    fn new(cp_number: u64) -> Self {
        CPDescriptor {
            cp_number,
            name: get_assigned_name(cp_number),
        }
    }
}


//================================= CPServer =================================//
struct CPServer {
    server: saws::Server,
    // pending_sawkets: The Sawkets that have not yet sent a subid and
    // therefore have not become valid CPClients yet
    pending_sawkets: Vec<saws::Sawket>,
    clients: Vec<CPClient>,
    // contains data about clients like the associated name
    descriptors: HashMap<String, CPDescriptor>,
    next_cp_number: u64,
    
}

impl CPServer {
    fn new(port: &str) -> Self {
        CPServer {
            server: saws::Server::new(port).unwrap(), // fatal
            clients: vec![],
            pending_sawkets: vec![],
            next_cp_number: 0,
            descriptors: HashMap::new(),
        }
    }

    // If an existing CPClient exists with this ID then add this sawket to that
    // cpclient, otherwise the ID is unique so create a new cpclient to hold
    // the sawket
    fn incorporate_new_sawket(&mut self, sawket: saws::Sawket, subid: u8) {
        let id_base = sawket_id_base(&sawket);
        let new_sawk_id = id_base + "-" + &subid.to_string();
        if let Some(client) = self.clients.iter_mut().find(|client| client.id == new_sawk_id) {
            client.add_sawket(sawket);
        } else {
            let client = CPClient::new(sawket, subid);
            self.insert_descriptor(client.id());
            self.next_cp_number += 1;
            write_cp_client(&client)
                .expect("Failure writing to cp_clients for new client");
            self.clients.push(client);
            dbgprint!("clients: {:?}", self.clients.iter()
                      .map(|x| x.id()).collect::<Vec<String>>());
        }
    }

    pub fn insert_descriptor(&mut self, id: String) {
        if self.descriptors.contains_key(&id) {
            return;
        }
        let descriptor = CPDescriptor::new(self.next_cp_number);
        self.next_cp_number += 1;
        self.descriptors.insert(id, descriptor);
    }

    pub fn remove_descriptor(&mut self, id: &str) {
        self.descriptors.remove(id);
    }

    pub fn get_name(&mut self, id: &str) -> String {
        self.descriptors.get(id)
            .map(|d| d.name.to_string())
            .unwrap_or("Nullephant".to_string())
    }
    
    pub fn accept_new_sawkets(&mut self) {
        self.pending_sawkets.append(&mut self.server.new_connections());
    }

    // For websockets that have died, remove the CPClient from our list and
    // update the cp_clients ipc object to reflect that
    pub fn clear_dead_clients(&mut self) {
        self.clients.iter_mut().for_each(|x| x.clear_dead_sawkets());
        let old_len = self.clients.len();
        self.clients.retain(|x| ! x.is_dead());
        if self.clients.len() == old_len {
            return;
        }
        rewrite_cp_clients(&self.clients)
            .expect("Failure rewriting cp_clients");
        dbgprint!("clients: {:?}", self.clients.iter()
                  .map(|x| x.id()).collect::<Vec<String>>());
    }

    // for each "_out" ipc object that has new messages, send those messages
    // over websocket to the associated client
    pub fn handle_messages_from_target(&mut self) {
        // TODO: In the first pass of loop over self.clients, collect up the
        //       messages. In the second pass, send to all clients with that ID
        for client in &mut self.clients {
            let msgs = read_msgs_for_client(client.id())
                .expect("Failure reading ipc from target");
            for m in msgs {
                dbgprint!("-> {}: '{}'", client.id(), m);
                client.send_msg(m);
            }
        }
    }

    pub fn handle_subids(&mut self) {
        let mut subids: Vec<u8> = Vec::new();
        let mut got_subid = |sawket: &mut saws::Sawket| -> bool {
            let mut should = false;
            if let (Some(m), _) = sawket.recv_msg() {
                if let Msg::Bytes(v) = m {
                    dbgprint!("<~ {} + {:?}", &sawket.addr(), &v);
                    if v.len() > 0 {
                        subids.push(v[0]);
                        should = true;
                    } 
                    if v.len() != 1 {
                        println!("Warning: invalid subid: {:?}", v);
                    }
                } else if let Msg::Text(t) = m {
                    println!("Warning: received Msg::Text from pending sawket : {:?}", t);
                } else {
                    println!("Warning: should be unreachable 932845");
                }
            }
            return should
        };
        let mut i = 0;
        let mut unpended_sawkets: Vec<saws::Sawket> = Vec::new();        
        while i < self.pending_sawkets.len() {
            if got_subid(&mut self.pending_sawkets[i]) {
                unpended_sawkets.push(self.pending_sawkets.remove(i));
            } else {
                i += 1;
            }
        }
        while unpended_sawkets.len() > 0 {
            self.incorporate_new_sawket(unpended_sawkets.remove(0), subids.remove(0));
        }
    }
    
    // for each websocket that had new messages, write those messages to the
    // associated  "_in" ipc object
    pub fn handle_messages_from_clients(&mut self) {
        let mut gamenite_msgs = Vec::<(String, String)>::new();
        for client in &mut self.clients {
            let msgs = client.recv_msgs();
            let mut game_msgs = Vec::<String>::new();
            for m in msgs {
                match m {
                    Msg::Text(t) => {
                        dbgprint!("<- {}: '{}'", &client.id(), &t);
                        if t.starts_with("_") {
                            gamenite_msgs.push((client.id(), t));    // GameNite protocol message
                        } else {
                            game_msgs.push(t);    // game protocol message
                        }
                    }
                    Msg::Bytes(v) => {                        
                        handle_bytes_from_client(&v);
                    }
                }
            }
            if game_msgs.len() != 0 {
                write_msgs_from_client(client.id(), game_msgs)
                    .unwrap_or_else(|e| {
                        println!("Warning: Failure writing ipc from client to target. Error: {}", e);
                    });
            }
        }
        for (client, msg) in gamenite_msgs {
            self.handle_gamenite_message_from_client(client, msg);
        }
    }

    fn handle_gamenite_message_from_client(&mut self, client: String, message: String) {
        if message.starts_with("_name") {
            let parts: Vec<&str> = message.split(":").collect();
            if parts.len() != 2 {
                println!("Warning: _name message formatted incorrectly: {}", message);
                
            }
        }
        // TODO ping
        else {
            println!("Warning: received invalid underscore message: {}", message);
        }
    }

    fn handle_gamenite_message_from_target(&mut self, message: String) {

    }
    
    //pub fn handle_text_messages(&mut self, messages: &Vec<String>, 

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

    // notify all clients that they should refresh their webpage
    pub fn send_reloads_to_clients(&mut self)  {
        let should_reload = self.read_reload().unwrap_or_else( |e| {
            panic!("Failed to read reload message with error {}", e);
        });
        if !should_reload {
            return;
        }
        // go through clients and send vec![0x1] which means reload
        for client in &mut self.clients {
            for sawk in & mut client.sawkets {
                sawk.send_msg(Msg::Bytes(vec![0x1]));
            }
        }
    }
}

//=================================== main ===================================//
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
        cpserver.accept_new_sawkets();
        cpserver.handle_subids();
        cpserver.handle_messages_from_target();
        cpserver.handle_messages_from_clients();
        cpserver.clear_dead_clients();
        cpserver.send_reloads_to_clients();
        std::thread::sleep(std::time::Duration::from_micros(1500));
    }
}

//==================================<===|===>=================================//
