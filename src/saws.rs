//========================== Simple A** Web Sockets ==========================//

/*
 * Copyright 2022-2024 RecBox, Inc.
 *
 * This file is part of the ControlpadServer program of the GameNite project.
 *
 * ControlpadServer is free software: you can redistribute it and/or modify it 
 * under the terms of the GNU General Public License as published by the Free 
 * Software Foundation, either version 3 of the License, or (at your option) 
 * any later version.
 * 
 * ControlpadServer is distributed in the hope that it will be useful, but 
 * WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY 
 * or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for 
 * more details.
 * 
 * You should have received a copy of the GNU General Public License along with 
 * ControlpadServer. If not, see <https://www.gnu.org/licenses/>.
 */

//==================================<===|===>=================================//
use std::net::{TcpStream, TcpListener};
use tungstenite;
use tungstenite::{WebSocket, accept, ServerHandshake};
use tungstenite::handshake::{MidHandshake, server::NoCallback, HandshakeError};
use crate::util::Result;


//================================== Sawket ==================================//
pub enum Msg {
    Text(String),
    Bytes(Vec<u8>),
}

pub type SawketId = String; // TODO: change to ID from browser

pub struct Sawket {
    websocket: WebSocket<TcpStream>,
    addr: String,
    dead: bool,
}

impl Sawket {
    pub fn new(websocket: WebSocket<TcpStream>) -> Result<Self> {
        let addr = match websocket.get_ref().peer_addr() {
            Ok(sock_addr) => sock_addr.to_string(),
            Err(e) => {
                println!("failure getting websocket address: {}", e);
                "0.0.0.0".to_string()
            }
        };
        // TODO: potentially increase timeout
	    Ok(Sawket {
	        websocket,
	        addr,
	        dead: false,
	    })
    }

    pub fn addr(&self) -> SawketId {
	    self.addr.clone()
    }

    pub fn is_dead(&self) -> bool {
	    self.dead
    }

    // Out: (a message if theres a valid one, whether there might still be messages left)
    pub fn recv_msg(&mut self) -> (Option<Msg>, bool) {
	    if self.dead {
	        return (None, false);
	    }
	    match self.websocket.read_message() {
		    Ok(m) => {
		        match m {
			        tungstenite::Message::Close(_) => {
			            self.dead = true;
			            return (None, false)
			        }
			        tungstenite::Message::Binary(v) => {
			            return (Some(Msg::Bytes(v)), true);
			        }
			        tungstenite::Message::Text(s) => {
			            return (Some(Msg::Text(s)), true);
			        }
			        other => {
			            println!("Warning: Sawket {} recved non-string non-binary \
				                  message: {:?}",
				                 self.addr, other);
                        return (None, true)
			        }
		        }
		    }
		    // no more messages available case
		    Err(tungstenite::error::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return (None, false);
		    }
		    Err(e) => {
		        println!("Warning: Sawket {} is dying because: <{}>",
			             self.addr, e);
		        self.dead = true;
                return (None, false);
		    }
	    }
    }

    pub fn recv_msgs(&mut self) -> Vec<Msg> {
	    if self.dead {
	        return vec![];
	    }
	    let mut msgs: Vec<Msg> = vec![];
	    loop {
            let (msg, contin) = self.recv_msg();
            if let Some(m) = msg {
                msgs.push(m);
            }
            if !contin {
                break;
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
	        println!("Warning: {} websocket.write_message returned an Err {}", &self.addr, e);
	    }
    }
}


//================================== Server ==================================//
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
	        server,
            handshake_continuation: None,
	    })
    }

    fn websocket_from_handshake_result(&mut self, result: HandshakeResult) ->
        Option<WebSocket<TcpStream>> {
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

    pub fn new_connections(&mut self) -> Vec<Sawket> {
	    let mut sawkets: Vec<Sawket> = vec![];
	    loop {
            if let Some(websocket) = self.get_next_websocket() {
                match Sawket::new(websocket) {
		            Ok(sawket) => {
 		                sawkets.push(sawket);
		            }
		            Err(e) => {
		                println!("Warning: Error when trying to create a sawket: {}", e);
		            }
	            }
            } else {
                break;
            }
	    }
	    return sawkets;
    }
}
//==================================<===|===>=================================//
