// atomic named pipe
mod ipc;
use std::str;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;


pub type ClientHandle = String;

/// Returns true if and only if a client has been added, dropped, or refreshed
pub fn clients_changed() -> Result<bool> {
    Ok(ipc::has_new("cp_clients")?)
}

/// Returns a vector of ClientHandles corresponding to the control pad clients
/// currently connected to the local control pad server
pub fn get_client_handles() -> Result<Vec<ClientHandle>> {
    let mut ret: Vec<ClientHandle> = Vec::new();
    let clients_string = ipc::read("cp_clients")?;
    let parts = clients_string.split(str::from_utf8(&[0])?);
    for p in parts {
	ret.push(String::from(p));
    }
    Ok(ret)
}

/// Send an atomic message to the specified control pad client
pub fn send_message(client: &ClientHandle, msg: &str) -> Result<()> {
    let ipc_name = client.to_string() + "_out";
    println!("sent {}", msg);
    ipc::write(&ipc_name, msg)?;
    ipc::write(&ipc_name, str::from_utf8(&[0])?)?;
    Ok(())
}

/// Returns a vector of all messages that have been received from the
/// specified control pad client since the last call to this function for that
/// client
pub fn get_messages(client: &ClientHandle) -> Result<Vec<String>> {
    let mut ret: Vec<String> = Vec::new();
    let ipc_name = client.to_string() + "_in";
    let msgs_string = ipc::consume(&ipc_name)?;
    let parts = &msgs_string.split(str::from_utf8(&[0])?).collect::<Vec<&str>>()[1..];
    for p in parts {
	println!("got {}", p);
	ret.push(String::from(*p));
    }
    Ok(ret)
}
