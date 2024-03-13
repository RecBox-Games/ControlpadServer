//==================================<===|===>=================================//
mod ipc;
mod saws;
mod systemlock;
mod util;
mod animal_names;
//
use saws::Msg;
use animal_names::{NUM_ANIMAL_NAMES, ANIMAL_NAMES};
//
use std::{str, collections::HashMap};
use unidecode::unidecode;
//
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

const MAX_NAME_CHARS: usize = 16;

// RPC without arguments should always be 2 bytes
const RPC_QUIT: &[u8] = &[0x99, 0x99];
const RPC_GETQR: &[u8] = &[0x98, 0x98];
// note: The term RPC is used loosely here. In this context it just means parts
//       of the controlpad message passing protocol that

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

fn reduce_to_length(s: &str, length: usize) -> String {
    s.chars().take(length).collect::<String>()
}

fn clean_name(name: &str) -> String {
    let ascii_equivalent = unidecode(name);
    let filtered = ascii_equivalent.chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect::<String>();
    let collapsed_whitespace = filtered.split_whitespace()
        .collect::<Vec<_>>().join(" ");
    let trimmed = reduce_to_length(collapsed_whitespace.trim_start(), MAX_NAME_CHARS)
        .trim_end().to_string();
    //
    trimmed
}

//================================ IPC Helpers ===============================//
// get the last two bytes of ip address from socket for identification
fn sawket_id_base(sawk: &saws::Sawket) -> String {
    let addr = sawk.addr();
    let ip = addr.split(":").next().unwrap();
    let id_bytes = ip.split(".").collect::<Vec<&str>>();
    return id_bytes[2..4].join("x");
}

// add to the list of connected clients
fn write_cp_client(client: &CPClient) -> Result<()> {
    let delin_id = client.id.clone() + str::from_utf8(&[0])?; // known to be valid utf8
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
fn read_msgs_for_client(id: &CPID) -> Result<Vec<String>> {
    let mut ret: Vec<String> = Vec::new();
    let ipc_name = id.clone() + "_out";
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
fn write_msgs_from_client(id: &CPID, msgs: Vec<String>) -> Result<()> {
    let mut s = String::new();
    for m in msgs {
        s += &m;
        s += str::from_utf8(&[0])?;
    }
    let ipc_name = id.clone() + "_in";
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
type CPID = String;
struct CPClient {
    id: CPID,
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

    fn has_id(&self, id: &str) -> bool {
        self.id == id
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


//================================== CPInfo ==================================//
struct CPInfo {
    next_cp_number: u64,
    name_from_id: HashMap<String, String>,
    id_from_lower_name: HashMap<String, String>,
}

impl CPInfo {
    fn new() -> Self {
        CPInfo {
            next_cp_number: 0,
            name_from_id: HashMap::new(),
            id_from_lower_name: HashMap::new(),
        }
    }
    
    fn add_client(&mut self, id: &CPID) {
        if self.name_from_id.contains_key(id) {
            println!("Warning: tried to add {} when it was already in \
                      name_from_id", id);
            return;
        }
        // Keep trying assigned names until we find one not already in use
        let mut name: String;
        loop {
            name = get_assigned_name(self.next_cp_number);
            self.next_cp_number += 1;
            // in the vast majority of cases we will break from the loop on the
            // first iteration
            if self.is_name_available(&name) {
                break;
            }
        }
        self.id_from_lower_name.insert(name.to_lowercase(), id.clone());
        self.name_from_id.insert(id.clone(), name);
    }

    fn try_change_name(&mut self, id: &str, name: String) {
        let lower_name = name.to_lowercase();
        let old_lower_name = if let Some(oln) = self.name_from_id.get(id) {
            oln.to_lowercase()
        } else {
                println!("Error: attempt to change name for {} which has no \
                          name currently", id);
                return;
        };
        // change names if the name either doesn't exist yet or if id already
        // owns the name (in which case we're changing capitalization)
        if let Some(owner_id) = self.id_from_lower_name.get(&lower_name) {
            if owner_id != id {
                println!("Note: Attempt to name change to a name owned by a \
                          different player");
                return;
            }
        }
        // change internal structures to represent the name change
        self.id_from_lower_name.remove(&old_lower_name);
        self.name_from_id.insert(id.to_string(), name);
        self.id_from_lower_name.insert(lower_name, id.to_string());        
    }

    fn is_name_available(&self, name: &str) -> bool {
        !self.id_from_lower_name.contains_key(&name.to_lowercase())
    }

    fn remove_client(&mut self, id: &str) {
        let name = if let Some(name) = self.name_from_id.remove(id) {
            name
        } else {
            println!("Warning: tried to remove {} when it was not in \
                      name_from_id", id);
            return;
        };
        let lower_name = name.to_lowercase();
        let _id = if let Some(id) = self.id_from_lower_name.remove(&lower_name) {
            id
        } else {
            println!("Warning: tried to remove {} when it was not in \
                      id_from_lower_name", lower_name);
            return;
        };
    }

    fn get_name(&mut self, id: &str) -> String {
        self.name_from_id.get(id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                println!("Warning: Returning a Nullephant");
                "Nullephant".to_string()
            })
    }

    fn print(&self) {
        println!("{:?}", self.name_from_id);
        println!("{:?}", self.id_from_lower_name);
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
    info: CPInfo,
}

impl CPServer {
    fn new(port: &str) -> Self {
        CPServer {
            server: saws::Server::new(port).unwrap(), // fatal
            clients: vec![],
            pending_sawkets: vec![],
            info: CPInfo::new(),
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
            self.info.add_client(&client.id);
            write_cp_client(&client)
                .expect("Failure writing to cp_clients for new client");
            self.clients.push(client);
            dbgprint!("clients: {:?}", self.clients.iter()
                      .map(|x| &x.id).collect::<Vec<&CPID>>());
        }
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
                  .map(|x| &x.id).collect::<Vec<&CPID>>());
    }

    fn send_message_to_client(&mut self, id: &str, msg: String) {
        // TODO: use a hashmap id->client for efficiency
        //
        // loop through our clients to find the one with id
        let maybe_client = self.clients.iter_mut().find(|c| c.has_id(id));
        // validate the client exists
        let client = if let Some(client) = maybe_client {
            client
        } else {
            println!("Warning: tried to send message to id that doesn't exist \
                      ({})", id);
            return;
        };
        // send
        dbgprint!("|> {}: '{}'", &client.id, &msg);
        client.send_msg(msg);
    }
    
    // for each "_out" ipc object that has new messages, send those messages
    // over websocket to the associated client
    pub fn handle_messages_from_target(&mut self) {
        // TODO: In the first pass of loop over self.clients, collect up the
        //       messages. In the second pass, send to all clients with that ID
        let mut gamenite_msgs = Vec::<(String, String)>::new();
        for client in &mut self.clients {
            let msgs = read_msgs_for_client(&client.id)
                .expect("Failure reading ipc from target");
            for m in msgs {
                dbgprint!("-> {}: '{}'", &client.id, m);
                if m.starts_with("_") {
                    gamenite_msgs.push((client.id.clone(), m));    // GameNite protocol message
                } else {
                    client.send_msg(m);    // game protocol message
                }
            }
        }
        // TODO: handle gamenite_messages
    }

    pub fn handle_subids(&mut self) {
        let mut subids: Vec<u8> = Vec::new();
        let mut got_subid = |sawket: &mut saws::Sawket| -> bool {
            let mut should = false;
            if let (Some(m), _) = sawket.recv_msg() {
                if let Msg::Bytes(v) = m {
                    dbgprint!("|< {} + {:?}", &sawket.addr(), &v);
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
                        dbgprint!("<- {}: '{}'", &client.id, &t);
                        if t.starts_with("_") {
                            gamenite_msgs.push((client.id.clone(), t));    // GameNite protocol message
                        } else {
                            game_msgs.push(t);    // game protocol message
                        }
                    }
                    Msg::Bytes(v) => {
                        dbgprint!("|< {} + {:?}", &client.id, &v);
                        handle_bytes_from_client(&v);
                    }
                }
            }
            if game_msgs.len() != 0 {
                write_msgs_from_client(&client.id, game_msgs)
                    .unwrap_or_else(|e| {
                        println!("Warning: Failure writing ipc from client to target. Error: {}", e);
                    });
            }
        }
        for (id, msg) in gamenite_msgs {
            self.handle_gamenite_message_from_client(&id, msg);
        }
    }

    fn _get_name(&mut self, id: &str, message: String) {
        if &message != "_get_name" {
            println!("Warning: invalid message {} should just be '_get_name'",
                     message);
            return;
        }
        let name = self.info.get_name(id);
        println!("-{}", name);
        self.send_message_to_client(id, format!("_name:{}", name));
    }

    fn _change_name(&mut self, id: &str, message: String) {
        let parts: Vec<&str> = message.split(":").collect();
        if parts.len() != 2 {
            println!("Warning: invalid message {} should be formatted \
                      '_change_name:<new-name>'", message);
            return;
        }
        self.handle_name_change_request(id, parts[1]);
        let name = self.info.get_name(id);
        println!("-{}", name);
        self.send_message_to_client(id, format!("_name:{}", name));
    }
    
    fn handle_gamenite_message_from_client(&mut self, id: &str, message: String) {
        if message.starts_with("_get_name") {
            self._get_name(&id, message);
        } else if message.starts_with("_change_name") {
            self._change_name(id, message);
        } else if message.starts_with("_print") {
            self.info.print();
        }
            // TODO ping
        else {
            println!("Warning: received invalid underscore message: {}", message);
        }
    }

    fn handle_name_change_request(&mut self, client: &str, name: &str) {
        let cleaned_name = clean_name(name);
        self.info.try_change_name(client, cleaned_name);
        
    }
    
    fn handle_gamenite_message_from_target(&mut self, message: String) {
        println!("TODO: handle gamenite message from target: {}", message);
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
