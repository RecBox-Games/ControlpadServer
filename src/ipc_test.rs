/*
 * Copyright 2022-2024 RecBox, Inc.
 *
 * This file is part of the ControlpadServer program of the GameNite project.
 *
 * ControlpadServer is free software: you can redistribute it and/or modify it 
 * under the terms of the GNU General Public License as published by the Free 
 * Software Foundation, either version 3 of the License, or (at your option) 
 * any later version.
 * 
 * ControlpadServer is distributed in the hope that it will be useful, but 
 * WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY 
 * or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for 
 * more details.
 * 
 * You should have received a copy of the GNU General Public License along with 
 * ControlpadServer. If not, see <https://www.gnu.org/licenses/>.
 */

mod ipc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("dirty: {}", ipc::has_new("foo")?);
    println!("read: {}", ipc::read("foo")?);
    println!("dirty: {}", ipc::has_new("foo")?);
    let s1 = "Brundwich champions\n";
    println!("writing: {}", s1);
    ipc::write("foo", s1)?;
    println!("dirty: {}", ipc::has_new("foo")?);
    let s2 = "Champion Schmampion\n";
    println!("writing: {}", s2);
    ipc::write("foo", s2)?;
    println!("dirty: {}", ipc::has_new("foo")?);
    println!("consume: {}", ipc::consume("foo")?);
    println!("dirty: {}", ipc::has_new("foo")?);
    let s3 = "I'm written last with no newline.";
    println!("writing: {}", s3);
    ipc::write("foo", s3)?;
    Ok(())
}
