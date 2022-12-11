use controlpads;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pads = Vec::<controlpads::ClientHandle>::new();
    loop {
	if controlpads::clients_changed()? {
	    pads = controlpads::get_client_handles()?;
	    println!("pads: {}", &pads);
	}
	for pad in &pads {
	    let msgs = controlpads::get_messages(pad)?;
	    for m in &msgs {
		println!("Got '{}'", m);
		let resp = format!("You said {}", m);
		controlpads::send_message(&pad, &resp)?;
	    }
	}
    }
}
