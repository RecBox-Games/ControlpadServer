#![allow(dead_code)]

extern crate file_lock;
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;
use std::fs::File;

use crate::systemlock::Locked;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const IPC_PATH: &str = "/home/requin/ipc/";


/* Atomically append to the IPC object with *name*.
 */
pub fn write(name: &str, data: &str) -> Result<()> {
    let lock = Locked::new(name)?;
    let path = format!("{}{}", IPC_PATH, name);
    if Path::new(&path).exists() {
	let mut f = File::options().write(true).open(&path)?;
	f.seek(SeekFrom::Start(0))?;
	f.write_all(&[1 as u8])?;
	f.seek(SeekFrom::End(0))?;
	f.write_all(data.as_bytes())?;
    } else {
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
    let lock = Locked::new(name)?;
    let path = format!("{}{}",IPC_PATH, name);
    if ! Path::new(&path).exists() {
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
    lock.unlock()?;
    Ok(s)
}


/* Return true if the ipc object with *name* has been written to since the last 
 * read (or consume).
 */
pub fn has_new(name: &str) -> Result<bool> {
    let lock = Locked::new(name)?;
    let path = format!("{}{}",IPC_PATH, name);
    if ! Path::new(&path).exists() {
	return Ok(false);
    }
    let mut f = File::options().read(true).write(false).create(false).open(&path)?;    
    if f.metadata().unwrap().len() == 0 {
	return Ok(false);
    }
    let mut buf = [0 as u8];
    f.read_exact(&mut buf)?;
    lock.unlock()?;
    Ok(buf[0] != 0)
}


