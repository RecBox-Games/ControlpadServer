#![allow(dead_code)]

extern crate file_lock;
use file_lock::{FileLock, FileOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;

use crate::systemlock::*;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const IPC_PATH: &str = "/home/requin/ipc/";


/* Atomically append to the IPC object with *name*.
 */
pub fn write(name: &str, data: &str) -> Result<()> {
    let lock = Locked::new("ipc")?;
    let path = format!("{}{}", IPC_PATH, name);
    let mut fl: FileLock;
    if Path::new(&path).exists() {
	let options: FileOptions = FileOptions::new().write(true);
	fl = FileLock::lock(&path, true, options)?;
	fl.file.seek(SeekFrom::Start(0))?;
	fl.file.write_all(&[1 as u8])?;
	fl.file.seek(SeekFrom::End(0))?;
	fl.file.write_all(data.as_bytes())?;
    } else {
	let options: FileOptions = FileOptions::new().create(true).write(true);
	fl = FileLock::lock(&path, true, options)?;
	fl.file.write_all(&[1 as u8])?;
	fl.file.write_all(data.as_bytes())?;
    }
    fl.unlock()?;
    lock.unlock()?;
    Ok(())
}


/* Atomically read the contents of the IPC object with *name*.
 */
pub fn read(name: &str) -> Result<String> {
    let lock = Locked::new("ipc")?;
    let path = format!("{}{}",IPC_PATH, name);
    if ! Path::new(&path).exists() {
	return Ok(String::new());
    }
    let options: FileOptions = FileOptions::new().read(true).write(true);
    let mut fl = FileLock::lock(&path, true, options)?;
    let mut s = String::new();
    fl.file.write_all(&[0 as u8])?;
    fl.file.read_to_string(&mut s)?;
    fl.unlock()?;
    lock.unlock();
    Ok(s)
}


/* Atomically read and erase the contents of the ipc object with *name*. Counts 
 * as a read.
 */
pub fn consume(name: &str) -> Result<String> {
    let lock = Locked::new("ipc")?;
    let path = format!("{}{}",IPC_PATH, name);
    if ! Path::new(&path).exists() {
	return Ok(String::new());
    }
    let options: FileOptions = FileOptions::new().read(true).write(true);
    let mut fl = FileLock::lock(&path, true, options)?;
    let mut s = String::new();
    fl.file.seek(SeekFrom::Start(1))?;
    fl.file.read_to_string(&mut s)?;
    //fl.file.set_len(0)?;
    std::fs::remove_file(&path)?;
    fl.unlock()?;
    lock.unlock()?;
    Ok(s)
}


/* Return true if the ipc object with *name* has been written to since the last 
 * read (or consume).
 */
pub fn has_new(name: &str) -> Result<bool> {
    let lock = Locked::new("ipc")?;
    let path = format!("{}{}",IPC_PATH, name);
    if ! Path::new(&path).exists() {
	return Ok(false);
    }
    let options: FileOptions = FileOptions::new().read(true).write(false).create(false);
    let mut fl = FileLock::lock(&path, true, options)?;
    if fl.file.metadata().unwrap().len() == 0 {
	return Ok(false);
    }
    let mut buf = [0 as u8];
    fl.file.read_exact(&mut buf)?;
    fl.unlock()?;
    lock.unlock()?;
    Ok(buf[0] != 0)
}


