#![allow(dead_code)]

use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;
use std::fs::File;

use crate::systemlock::Locked;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[cfg(target_os = "linux")]
const IPC_PATH: &str = "/home/requin/ipc/";
#[cfg(target_os = "windows")]
const IPC_PATH: &str = "C:\\Windows\\ipc\\";


pub fn initialize() -> Result<()> {
    //#[cfg(debug_assertions)] println!("ipc initialize");
    if !std::path::Path::new(IPC_PATH).exists() {
	std::fs::create_dir(IPC_PATH)?;
    }
    // TODO: delete previous IPC data in the dir
    Ok(())
}

/* Atomically append to the IPC object with *name*.
 */
pub fn write(name: &str, data: &str) -> Result<()> {
    //#[cfg(debug_assertions)] println!("ipc write: name: {}, data: {}", name, data);
    let lock = Locked::new(name)?;
    let path = format!("{}{}", IPC_PATH, name);
    if Path::new(&path).exists() {
        //#[cfg(debug_assertions)] println!("existing file");
	let mut f = File::options().write(true).open(&path)?;
	f.seek(SeekFrom::Start(0))?;
	f.write_all(&[1 as u8])?;
	f.seek(SeekFrom::End(0))?;
	f.write_all(data.as_bytes())?;
    } else {
        //#[cfg(debug_assertions)] println!("new file");
	let mut f = File::options().create(true).write(true).open(&path)?;
	f.write_all(&[1 as u8])?;
	f.write_all(data.as_bytes())?;
    }
    lock.unlock()?;
    Ok(())
}


/* Atomically read the contents of the IPC object with *name*.
 */
pub fn read(name: &str) -> Result<String> {
    //#[cfg(debug_assertions)] println!("ipc read: name: {}", name);
    let lock = Locked::new(name)?;
    let path = format!("{}{}",IPC_PATH, name);
    if ! Path::new(&path).exists() {
        //#[cfg(debug_assertions)] println!("no file to read");
	return Ok(String::new());
    }
    let mut f = File::options().read(true).write(true).open(&path)?;    
    let mut s = String::new();
    f.write_all(&[0 as u8])?;
    f.read_to_string(&mut s)?;
    lock.unlock()?;
    Ok(s)
}


/* Atomically read and erase the contents of the ipc object with *name*. Counts 
 * as a read.
 */
pub fn consume(name: &str) -> Result<String> {
    //#[cfg(debug_assertions)] println!("ipc consume: name: {}", name);
    let lock = Locked::new(name)?;
    let path = format!("{}{}",IPC_PATH, name);
    if ! Path::new(&path).exists() {
        return Ok(String::new());
    }
    let mut f = File::options().read(true).write(true).open(&path)?;    
    let mut s = String::new();
    f.seek(SeekFrom::Start(1))?;
    f.read_to_string(&mut s)?;
    std::fs::remove_file(&path)?;
    let mut f_new = File::options().create(true).write(true).open(&path)?;
    f_new.write_all(&[1 as u8])?;
    lock.unlock()?;
    Ok(s)
}


/* Return true if the ipc object with *name* has been written to since the last 
 * read (or consume).
 */
pub fn has_new(name: &str) -> Result<bool> {
    //#[cfg(debug_assertions)] println!("has_new: {}", name);
    let lock = Locked::new(name)?;
    let path = format!("{}{}",IPC_PATH, name);
    if ! Path::new(&path).exists() {
        //#[cfg(debug_assertions)] println!("no file to check");
	return Ok(false);
    }
    let mut f = File::options().read(true).write(false).create(false).open(&path)?;    
    if f.metadata()?.len() == 0 {
	return Ok(false);
    }
    let mut buf = [0 as u8];
    f.read_exact(&mut buf)?;
    lock.unlock()?;
    Ok(buf[0] != 0)
}


