mod ipc;
mod systemlock;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    //
    ipc::has_new("foo")?;
    ipc::read("foo")?;
    ipc::has_new("foo")?;
    let s1 = "Brundwich champions\n";
    ipc::write("foo", s1)?;
    ipc::has_new("foo")?;
    let s2 = "Champion Schmampion\n";
    ipc::write("foo", s2)?;
    ipc::write("baz", s2)?;
    ipc::has_new("foo")?;
    ipc::consume("foo")?;
    ipc::consume("baz")?;
    ipc::has_new("foo")?;
    ipc::has_new("baz")?;
    let s3 = "I'm written last with no newline.";
    ipc::write("foo", s3)?;
    //
    let elapsed = start.elapsed();
    println!("Elapsed time: {} microseconds", elapsed.as_micros());
    Ok(())
}
