mod systemlock;
use systemlock::Locked;
use std::fs::File;
use std::io::Write;
use rand::Rng;


fn write_to_foo(s: &str) {
    //let lock = Locked::new("q").unwrap(); {
	let mut f = File::options().create(true).append(true).open("foo.txt").unwrap();
    //std::thread::sleep(std::time::Duration::from_millis(1000));
	f.write_all(s.as_bytes()).unwrap();
//} lock.unlock().unwrap();
}

fn main() {
    let mut rng = rand::thread_rng();
    let mut q: u8 = rng.gen();
    q %= 10;
    for i in 0..5 {
	write_to_foo(&format!("{} ABC {}\n", q, i));
    }
}
