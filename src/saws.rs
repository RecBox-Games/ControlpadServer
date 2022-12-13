// Simple A** Web Sockets

use std::net::{TcpStream, TcpListener};
use tungstenite::{WebSocket, accept};
use tungstenite;
use crate::util::Result;
    

#[cfg(debug_assertions)]
macro_rules! dbgprint {
    ($( $args:expr ),*) => { println!($( $args),* ) }
}

#[cfg(not(debug_assertions))]
macro_rules! dbgprint {
    ($( $args:expr ),*) => { }
}


pub enum Msg {
    Text(String),
    Bytes(Vec<u8>),
}


pub type ConnId = String; // TODO: change to ID from browser

pub struct Conn {
    websocket: WebSocket<TcpStream>,
    addr: String,
    dead: bool,
}

impl Conn {

    pub fn new(mut ws: WebSocket<TcpStream>, addr: String) -> Result<Self> {
	ws.get_mut().set_nonblocking(true)?;
	Ok(Conn {
	    websocket: ws,
	    addr: addr,
	    dead: false,
	})
    }

    pub fn addr(&self) -> ConnId {
	self.addr.clone()
    }

    pub fn is_dead(&self) -> bool {
	self.dead
    }

    pub fn get_recved_msgs(&mut self) -> Vec<Msg> {
	if self.dead {
	    return vec![];
	}
	let mut msgs: Vec<Msg> = vec![];
	loop {
	    match self.websocket.read_message() {
		Ok(m) => {
		    match m {
			tungstenite::Message::Close(_) => {
			    self.dead = true;
			    break;
			}
			tungstenite::Message::Binary(v) => {
			    dbgprint!("<- {}: {:?}", &self.addr, &v);
			    msgs.push(Msg::Bytes(v));
			}
			tungstenite::Message::Text(s) => {
			    dbgprint!("<- {}: '{}'", &self.addr, &s);
			    msgs.push(Msg::Text(s));
			}
			other => {
			    println!("Warning: Conn {} recved non-string non-binary \
				      message: {:?}",
				     self.addr, other);
			}
		    }
		}
		// no more messages available case
		Err(tungstenite::error::Error::Io(e))
		    if e.kind() == std::io::ErrorKind::WouldBlock => {
		    break;
		}
		Err(e) => {
		    println!("Warning: Conn {} is dying because: <{}>",
			     self.addr, e);
		    self.dead = true;
		    break;
		}
	    }
	}
	msgs
    }

    pub fn send_msg(&mut self, msg: Msg) {
	let res;
	match msg {
	    Msg::Text(t) => {
		//dbgprint!("-> {}: '{}'", &self.id, &t);
		res = self.websocket.write_message(tungstenite::Message::Text(t));
	    }
	    Msg::Bytes(b) => {
		//dbgprint!("-> {}: {:?}", &self.id, &b);
		res = self.websocket.write_message(tungstenite::Message::Binary(b));
	    }
	}
	if let Err(e) = res {
	    println!("Warning: write_message returned an Err {}", e);
	}
    }

}

pub struct Server {
    server: TcpListener,
}

impl Server {
    pub fn new(port: &str) -> Result<Self> {
	let server = TcpListener::bind("0.0.0.0:".to_string()+port).unwrap();
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
