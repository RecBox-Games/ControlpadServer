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
