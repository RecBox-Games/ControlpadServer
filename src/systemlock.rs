//==================================<===|===>===================================
extern crate file_lock;
use file_lock::{FileLock, FileOptions};
use std::error::Error;

//=================================== Notes ====================================
/* 
Why not just use Filelock directly since we're using these "semaphores" to 
synchronize writing to files you ask?
Well things get funny when you're doing multiple things with a locked file. 
e.g. consume reads the file then deletes it.
When I tried to implement ipc using base FileLock I got race conditions (that 
were fixed by wrapping that same code in one of these systemlocks)
*/

//================================= Constants ==================================
#[cfg(debug_assertions)]
const LOCK_DIR: &str = "/var/lock";
#[cfg(not(debug_assertions))]
const LOCK_DIR: &str = "/home/requin/lock";
//
const LOCK_PREFIX: &str = "/sl_";
    
//================================== Helpers ===================================
#[allow(dead_code)]
pub fn initialize() -> Result<(), Box<dyn Error>> {
    //#[cfg(debug_assertions)] println!("locked initialize");
    if !std::path::Path::new(LOCK_DIR).exists() {
	std::fs::create_dir(LOCK_DIR)?;
    }
    // TODO: delete previous Lock data in the dir
    Ok(())
}

//================================== Locked ====================================
// one use lock
pub struct Locked {
    lock: FileLock,
}
impl Locked {
    pub fn new(s: &str) -> Result<Self, Box<dyn Error>> {
        let options = FileOptions::new().write(true).create(true).append(true);
	let path = String::from(LOCK_DIR) + LOCK_PREFIX + s;
        Ok(Locked {
            lock: FileLock::lock(&path, true /* should block */, options)?,
        })
    }

    pub fn unlock(self) -> Result<(), std::io::Error>{
        self.lock.unlock()
    }

}

//==================================<===|===>===================================
