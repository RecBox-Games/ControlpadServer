mod saws;
//use crate::saws;


fn main() -> Result<(), std::io::Error> {
    let server = saws::Server::new("192.168.0.100:3333")?;
    let mut conns: Vec<saws::Conn> = vec![];
    let mut msgs: Vec<(saws::ConnId, Vec<String>)> = vec![];
    loop {
	conns.append(&mut server.new_connections());
	for c in &mut conns {
	    msgs.push((c.id(), c.get_recved_msgs()));
	}
	for (id, mset) in msgs {
	    for m in mset {
		println!("{} said {}", id, m);
	    }
	}
	msgs = vec![];
	conns.retain(|x| ! x.is_dead());
    }
}
