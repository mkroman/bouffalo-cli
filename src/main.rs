// Build script inspired by the one in gamozolabs x86_64 kernel:
// https://github.com/gamozolabs/chocolate_milk/blob/69640cc31e4cd96cbd162ab92fe5cf701c454f74/src/main.rs

use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the build folders if they don't exist
    fs::create_dir_all("build/bootloader")?;

    // Build the bootloader
    let bootloader_dir = Path::new("bootloader");
    if !Command::new("cargo")
        .current_dir(bootloader_dir)
        .arg("build")
        .arg("--release")
        .arg("--target-dir")
        .arg("../build/bootloader")
        .status()?
        .success()
    {
        return Err("Could not build bootloader".into());
    }

    Ok(())
}
