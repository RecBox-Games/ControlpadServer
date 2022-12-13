extern crate file_lock;
use file_lock::{FileLock, FileOptions};
use std::error::Error;


/* 
Why not just use Filelock directly since we're using these "semaphores" to 
synchronize writing to files you ask?
Well things get funny when you're doing multiple things with a locked file. 
e.g. consume reads the file then deletes it.
When I tried to implement ipc using base FileLock I got race conditions (that 
were fixed by wrapping that same code in one of these systemlocks)
*/


// one use lock
pub struct Locked {
    lock: FileLock,
}
impl Locked {
    pub fn new(s: &str) -> Result<Self, Box<dyn Error>> {
        let options = FileOptions::new().write(true).create(true).append(true);
	let path = "/var/lock/sl_".to_string() + s;
        Ok(Locked {
            lock: FileLock::lock(&path, true /* should block */, options)?,
        })
    }

    pub fn unlock(self) -> Result<(), std::io::Error>{
        self.lock.unlock()
    }

}

