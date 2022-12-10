// atomic named pipe
mod ipc;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;


pub type ClientHandle = u32;

/// Returns true if and only if a client has been added, dropped, or refreshed
pub fn clients_changed() -> Result<bool> {
    Ok(ipc::has_new("clients")?)
}

/// Returns a vector of ClientHandles corresponding to the control pad clients
/// currently connected to the local control pad server
pub fn get_client_handles() -> Result<Vec<ClientHandle>> {
    let mut ret: Vec<ClientHandle> = Vec::new();
    
    Ok(vec![])
}

/// Send an atomic message to the specified control pad client
pub fn send_message(client: ClientHandle, msg: &str) -> Result<()> {
    Ok(())
}

/// Returns a vector of all messages that have been received from the
/// specified control pad client since the last call to this function for that
/// client
pub fn get_messages(client: ClientHandle) -> Result<Vec<String>> {
    Ok(vec![])
}
