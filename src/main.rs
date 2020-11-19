// Build script inspired by the one in gamozolabs x86_64 kernel:
// https://github.com/gamozolabs/chocolate_milk/blob/69640cc31e4cd96cbd162ab92fe5cf701c454f74/src/main.rs

use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the build folders if they don't exist
    fs::create_dir_all("build/bootloader")?;

    // Build the bouffalo cli tool
    println!("Building bouffali-cli");

    let bouffalo_cli_dir = Path::new("bouffalo-cli");

    if !Command::new("cargo")
        .current_dir(bouffalo_cli_dir)
        .arg("build")
        .arg("--target-dir")
        .arg("../build/bouffali-cli")
        .arg("--release")
        .status()?
        .success()
    {
        return Err("Could not build bouffalo-cli".into());
    }

    // Build the bootloader
    let bootloader_dir = Path::new("bootloader");

    println!("Building bootloader");

    if !Command::new("cargo")
        .current_dir(bootloader_dir)
        .arg("build")
        .arg("--target-dir")
        .arg("../build/bootloader")
        .arg("--release")
        .status()?
        .success()
    {
        return Err("Could not build bootloader".into());
    }

    // Convert the elf to a firmware image
    if !Command::new("build/bouffali-cli/release/bouffalo-cli")
        .arg("elf2image")
        .arg("build/bootloader/riscv32imac-unknown-none-elf/release/bootloader")
        .status()?
        .success()
    {
        return Err("Could not convert elf to firmware image".into());
    }

    Ok(())
}
