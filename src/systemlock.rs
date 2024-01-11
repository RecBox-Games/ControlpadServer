//==================================<===|===>===================================
extern crate fs2;
use fs2::FileExt;
use std::fs::{OpenOptions, File};
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
#[cfg(target_os = "macos")]
const LOCK_DIR: &str = "/var/tmp";
#[cfg(target_os = "linux")]
const LOCK_DIR: &str = "/var/lock";
#[cfg(target_os = "windows")]
const LOCK_DIR: &str = "C:\\Users\\gamenite";
//
#[cfg(target_os = "macos")]
const LOCK_PREFIX: &str = "/sl_";
#[cfg(target_os = "linux")]
const LOCK_PREFIX: &str = "/sl_";
#[cfg(target_os = "windows")]
const LOCK_PREFIX: &str = "\\sl_";
    
//================================== Helpers ===================================
#[allow(dead_code)]
pub fn initialize() {
    //#[cfg(debug_assertions)] println!("locked initialize");
    if !std::path::Path::new(LOCK_DIR).exists() {
	std::fs::create_dir(LOCK_DIR)
            .unwrap_or_else(|e| {
                let help_msg = format!(
                    "Try creating {} yourself and giving yourself permission to \
                     make files within that directory",
                    LOCK_DIR
                );
                panic!("Fatal Error: Could not create {}: {}\n{}",
                       LOCK_DIR, e, help_msg);
            });
    }
    // TODO: delete previous Lock data in the dir
}

//================================== Locked ====================================
// one use lock
pub struct Locked {
    lock: File,
}
impl Locked {
    pub fn new(s: &str) -> Result<Self, Box<dyn Error>> {
        let path = String::from(LOCK_DIR) + LOCK_PREFIX + s;
        let file = OpenOptions::new().read(true).write(true).create(true).open(&path)
            .unwrap_or_else(|e| {
                panic!("Fatal Error: Failed to open {}: {}\n(Try changing \
                        permissions on {})", &path, e, LOCK_DIR);
            });
        file.lock_exclusive()?;
        Ok(Locked { lock: file })
    }

    pub fn unlock(self) -> Result<(), std::io::Error> {
        self.lock.unlock()
    }

}

//==================================<===|===>===================================
