mod ipc;
mod systemlock;
use std::str;
type GenErr = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, GenErr>;


pub type ClientHandle = String;

/// Returns true if and only if a client has been added, dropped, or refreshed
/// since the last call to get_client_handles
pub fn clients_changed() -> Result<bool> {
    ipc::has_new("cp_clients").or_else(|e| {
        Err(format!("Failed to check has_new: {}", e).into())
    })
}

/// Returns a vector of ClientHandles corresponding to the control pad clients
/// currently connected to the local control pad server
pub fn get_client_handles() -> Result<Vec<ClientHandle>> {
    let mut ret: Vec<ClientHandle> = Vec::new();
    let clients_string = ipc::read("cp_clients").or_else(|e| {
        Err::<_, GenErr>(format!("Failed to read: {}", e).into())
    })?;
    let mut parts = clients_string.split(str::from_utf8(&[0])?);
    
    while let Some(p) = parts.next() {
	// don't take whatever is after the last null byte because nothing should be
	// past the last null byte and we don't want to add an empty string
	if p.len() != 0 {
	    ret.push(String::from(p));
	}
    }
    Ok(ret)
}

/// Send an atomic message to the specified control pad client
pub fn send_message(client: &ClientHandle, msg: &str) -> Result<()> {
    let ipc_name = client.to_string() + "_out";
    //println!("sent {}", msg);
    let delin_msg = msg.to_string() + str::from_utf8(&[0])?;
    ipc::write(&ipc_name, &delin_msg).or_else(|e| {
        Err::<_, GenErr>(format!("Failed to write: {}", e).into())
    })?;
    Ok(())
}

/// Returns a vector of all messages that have been received from the
/// specified control pad client since the last call to this function for that
/// client
pub fn get_messages(client: &ClientHandle) -> Result<Vec<String>> {
    let mut ret: Vec<String> = Vec::new();
    let ipc_name = client.to_string() + "_in";
    let msgs_string = ipc::consume(&ipc_name).or_else(|e| {
        Err::<_, GenErr>(format!("Failed to consume: {}", e).into())
    })?;
    if msgs_string.len() == 0 {
	return Ok(vec![]);
    }
    //println!("{}", msgs_string.replace(str::from_utf8(&[0])?, "0"));
    let mut parts = msgs_string.split(str::from_utf8(&[0])?).collect::<Vec<&str>>();
    parts.pop(); // there will be nothing after last null byte
    for p in &parts {
	//println!("got {}", p);
	ret.push(String::from(*p));
    }
    Ok(ret)
}
