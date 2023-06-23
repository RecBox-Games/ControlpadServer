// Simple A** Web Sockets
use std::net::{TcpStream, TcpListener};
use tungstenite;
use tungstenite::{WebSocket, accept, ServerHandshake};
use tungstenite::handshake::{MidHandshake, server::NoCallback, HandshakeError};
use crate::util::Result;

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
    pub fn new(ws: WebSocket<TcpStream>) -> Result<Self> {
        let addr = match ws.get_ref().peer_addr() {
            Ok(sock_addr) => sock_addr.to_string(),
            Err(e) => {
                println!("failure getting websocket address: {}", e);
                "0.0.0.0".to_string()
            }
        };
        // TODO: potentially increase timeout
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
			    msgs.push(Msg::Bytes(v));
			}
			tungstenite::Message::Text(s) => {
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
		res = self.websocket.write_message(tungstenite::Message::Text(t));
	    }
	    Msg::Bytes(b) => {
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
    handshake_continuation: Option<MidHandshake<ServerHandshake<TcpStream, NoCallback>>>,
}

type HandshakeResult = std::result::Result<WebSocket<TcpStream>,
                              HandshakeError<ServerHandshake<TcpStream, NoCallback>>>;

impl Server {
    pub fn new(port: &str) -> Result<Self> {
	let server = TcpListener::bind("0.0.0.0:".to_string()+port).unwrap();
	server.set_nonblocking(true)?;
	Ok(Server {
	    server: server,
            handshake_continuation: None,
	})
    }

    fn websocket_from_handshake_result(&mut self, result: HandshakeResult) -> Option<WebSocket<TcpStream>> {
        match result {
	    Ok(websocket) => {
                Some(websocket)
	    }
	    Err(HandshakeError::Interrupted(mid_handshake)) => {
                self.handshake_continuation = Some(mid_handshake);
                None
	    }
	    Err(e) => {
		println!("Warning: Error during websocket handshake: {}", e);
                None
	    }
        }
    }

    fn get_next_websocket(&mut self) -> Option<WebSocket<TcpStream>> {
        // in-progress handshaking connection
        if let Some(mid_handshake) = self.handshake_continuation.take() {
            let result = mid_handshake.handshake();
            return self.websocket_from_handshake_result(result);
        } 
        // brand new connection
	match self.server.accept() {
	    Ok((stream, _)) => {
                if let Err(e) = stream.set_nonblocking(true) {
                    println!("Failed to set stream to nonblocking before accept(): {}", e);
                }
                let result = accept(stream);
                self.websocket_from_handshake_result(result)
	    }
	    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                None
	    }
            Err(e) => {
		println!("Warning: Unexpected error when trying to accept a connection: {}", e);
                None
            }                
	}
    }

    pub fn new_connections(&mut self) -> Vec<Conn> {
	let mut conns: Vec<Conn> = vec![];
	loop {
            if let Some(websocket) = self.get_next_websocket() {
                match Conn::new(websocket) {
		    Ok(conn) => {
 		        conns.push(conn);
		    }
		    Err(e) => {
		        println!("Warning: Error when trying to create a conn: {}", e);
		    }
	        }
            } else {
                break;
            }
	}
	return conns;
    }
}
