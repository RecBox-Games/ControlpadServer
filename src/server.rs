mod ipc;
mod saws;

use std::str;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// ----------------------------
// vvv ipc helper functions vvv

fn write_cp_clients(conns: &Vec<saws::Conn>) -> Result<()> {
    for conn in conns {
	ipc::write("cp_clients", &conn.id())?;
	ipc::write("cp_clients", str::from_utf8(&[0])?)?;
    }
    Ok(())
}

fn rewrite_cp_clients(conns: &Vec<saws::Conn>) -> Result<()> {
    ipc::consume("cp_clients")?;
    write_cp_clients(conns)?;
    Ok(())
}

fn read_msgs_for_client(id: saws::ConnId) -> Result<Vec<String>> {
    let mut ret: Vec<String> = Vec::new();
    let ipc_name = id + "_out";
    let msgs_string = ipc::consume(&ipc_name)?;
    let parts = msgs_string.split(str::from_utf8(&[0])?);
    for p in parts {
	ret.push(String::from(p));
    }
    Ok(ret)
}

fn write_msgs_from_client(id: saws::ConnId, msgs: Vec<String>) -> Result<()> {
    let mut s = String::new();
    for m in msgs {
	s += &m;
	s += str::from_utf8(&[0])?;
    }
    let ipc_name = id + "_in";
    println!("--- {}", &s);
    ipc::write(&ipc_name, &s)?;
    Ok(())
}

// ^^^ ipc helper functions ^^^
// ----------------------------


// --------------------------
// vvv Control Pad Server vvv

struct CPServer {
    server: saws::Server,
    conns: Vec<saws::Conn>,
 
}

impl CPServer {
    fn new(port: &str) -> Self {
	CPServer {
	    server: saws::Server::new(port).unwrap(), // fatal
	    conns: vec![],
	}
    }

    pub fn accept_new_clients(&mut self) {
	let mut new_conns = self.server.new_connections();
	if new_conns.len() == 0 {
	    return;
	}
	write_cp_clients(&new_conns)
	    .expect("Failure writing to cp_clients");
	    
	self.conns.append(&mut new_conns);
    }
    
    pub fn clear_dead_clients(&mut self) {
	let old_len = self.conns.len();
	self.conns.retain(|x| ! x.is_dead());
	if self.conns.len() < old_len {
	    rewrite_cp_clients(&self.conns)
		.expect("Failure rewriting cp_clients");
	}
    }

    pub fn send_messages_to_clients(&mut self) {
	for c in &mut self.conns {
	    let msgs = read_msgs_for_client(c.id())
		.expect("Failure reading ipc from target");
	    for m in msgs {
		c.send_msg(m+str::from_utf8(&[0]).unwrap()); // [0] known to be valid utf8
	    }
	}
    }

    pub fn recv_messages_for_target(&mut self) {
	for c in &mut self.conns {
	    let msgs = c.get_recved_msgs();
	    if msgs.len() != 0 {
		println!("got {:?} from {}", msgs, c.id());
		write_msgs_from_client(c.id(), msgs)
		    .expect("Failure writing ipc to target");
	    }
	}
    }
}

// ^^^ Control Pad Server ^^^
// --------------------------



fn main() {
    let mut cpserver = CPServer::new("50079");

    loop {
	cpserver.accept_new_clients();
	cpserver.send_messages_to_clients();
	cpserver.recv_messages_for_target();
	cpserver.clear_dead_clients();	
    }
}
