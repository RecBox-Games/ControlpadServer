// Simple A** Web Sockets

use std::net::{TcpStream, TcpListener};
use tungstenite::{WebSocket, accept};
use tungstenite::protocol::Message;

pub type ConnId = String; // TODO: change to ID from browser

pub struct Conn {
    websocket: WebSocket<TcpStream>,
    id: ConnId,
    dead: bool,
}

impl Conn {

    pub fn new(mut ws: WebSocket<TcpStream>, addr: String) -> Result<Self, std::io::Error> {
	ws.get_mut().set_nonblocking(true)?;
	Ok(Conn {
	    websocket: ws,
	    id: addr,
	    dead: false,
	})
    }

    pub fn id(&self) -> ConnId {
	self.id.clone()
    }

    pub fn is_dead(&self) -> bool {
	self.dead
    }

    pub fn get_recved_msgs(&mut self) -> Vec<String> {
	if self.dead {
	    return vec![];
	}
	let mut msgs: Vec<String> = vec![];
	loop {
	    match self.websocket.read_message() {
		Ok(Message::Text(s)) => {
		    msgs.push(s);
		}
		Ok(Message::Close(_)) => {
		    self.dead = true;
		    break;
		}
		Ok(other) => {
		    println!("Warning: Conn {} recved non-string message: {:?}", self.id, other);
		}
		Err(tungstenite::error::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
		    break;
		}
		Err(e) => {
		    println!("Warning: Conn {} is dying because: <{}>", self.id, e); // TODO: die
		    self.dead = true;
		    break;
		}
	    }
	}
	msgs
    }

    pub fn send_msg(&self, msg: String) {
	println!("can't send {}", msg);
    }

}

pub struct Server {
    server: TcpListener,
}

impl Server {
    pub fn new(addr: &str) -> Result<Self, std::io::Error>{
	let server = TcpListener::bind(addr).unwrap();
	server.set_nonblocking(true)?;
	Ok(Server {
	    server: server,
	})
    }

    pub fn new_connections(&self) -> Vec<Conn> {
	let mut conns: Vec<Conn> = vec![];
	loop {
	    match self.server.accept() {
		Ok((stream, addr)) => {
		    match accept(stream) {
			Ok(websocket) => {
			    match Conn::new(websocket, addr.to_string()) {
				Ok(conn) => {
 				    conns.push(conn);
				}
				Err(e) => {
				    println!("Warning: Error when trying to create a conn: {}", e);
				}
			    }
			}
			Err(e) => {
			    println!("Warning: Error when trying to validate a connection: {}", e);
			}
		    }
		}
		Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
		    break;
		}
		Err(e) => {
		    println!("Warning: Unexpected error when trying to accept a connection: {}", e);
		    break;
		}
	    }
	}
	return conns;
    }
}
