// Build script inspired by the one in gamozolabs x86_64 kernel:
// https://github.com/gamozolabs/chocolate_milk/blob/69640cc31e4cd96cbd162ab92fe5cf701c454f74/src/main.rs

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

const RELEASE_TARGETS: [&str; 3] = [
    "x86_64-unknown-linux-musl",
    "x86_64-pc-windows-gnu",
    "aarch64-unknown-linux-gnu",
];

/// Runs the given `cmdline` to determine whether the tool called `name` is installed and usable
///
/// Returns `Some(())` on success
fn run_quiet(cmd: &str, args: &[&str]) -> Option<()> {
    if Command::new(cmd).args(args).status().ok()?.success() {
        return Some(());
    } else {
        return None;
    }
}

fn build() -> Result<(), Box<dyn std::error::Error>> {
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
        .stdout(Stdio::null())
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

fn release() -> Result<(), Box<dyn std::error::Error>> {
    let bouffalo_cli_dir = Path::new("bouffalo-cli");

    run_quiet("git", &["version"]).expect("git not available");
    run_quiet("cross", &["version"]).expect("cross not available");
    run_quiet("gzip", &["-V"]).expect("gzip not available");

    // Get the current git tag
    let git_tag = String::from_utf8(
        Command::new("git")
            .args(&["describe", "--tags"])
            .output()
            .expect("Could not get current git tag")
            .stdout,
    )
    .expect("Could not parse git output");

    // Create a temporary staging directory
    let temp_dir = std::env::temp_dir().join("bouffalo-release");
    std::fs::create_dir_all(&temp_dir)?;

    for target in RELEASE_TARGETS.iter() {
        println!("Building bouffalo-cli for target {}", target);

        if !Command::new("cross")
            .current_dir(bouffalo_cli_dir)
            .arg("build")
            .arg("--verbose")
            .arg("--release")
            .arg("--target")
            .arg(target)
            .status()?
            .success()
        {
            return Err(format!("Failed to build bouffalo-cli for target {}", target).into());
        }

        // Copy the executable to the temporary working directory
        let filename = if target == &"x86_64-pc-windows-gnu" {
            "bouffalo-cli.exe"
        } else {
            "bouffalo-cli"
        };

        let exe_path = bouffalo_cli_dir
            .join("target")
            .join(target)
            .join("release")
            .join(filename);

        // Format the filename as `bouffalo-cli-<git tag>-<target>[.exe]`
        let target_filename = if target == &"x86_64-pc-windows-gnu" {
            format!(
                "{}-{}-{}.exe",
                exe_path.file_stem().unwrap().to_string_lossy(),
                &git_tag.trim(),
                target
            )
        } else {
            format!(
                "{}-{}-{}",
                exe_path.file_stem().unwrap().to_string_lossy(),
                &git_tag.trim(),
                target
            )
        };

        let mut temp_exe_path = temp_dir.join(target_filename);

        if let Some(ext) = exe_path.extension() {
            temp_exe_path.set_extension(ext);
        }

        println!(
            "Copying {} to {}",
            &exe_path.display(),
            &temp_exe_path.display()
        );

        std::fs::copy(&exe_path, &temp_exe_path)?;
        println!("Compressing {} with `gzip`", &temp_exe_path.display());

        // Compress the file using the `gzip` command line tool
        if !Command::new("gzip")
            .arg("--force")
            .arg(&temp_exe_path)
            .status()?
            .success()
        {
            return Err(format!("Failed to run gzip on file {}", &temp_exe_path.display()).into());
        }

        if let Ok(github_token) = std::env::var("GITHUB_TOKEN") {
            // Run `ghr` to publish the executable in a release
            run_quiet("ghr", &["-v"]).expect("ghr is not installed");

            if !Command::new("ghr")
                .env("GITHUB_TOKEN", github_token)
                .arg(&git_tag.trim())
                .arg(format!("{}.gz", temp_exe_path.display()))
                .status()?
                .success()
            {
                eprintln!(
                    "Failed to run ghr to submit file {}",
                    &temp_exe_path.display()
                );
            }
        } else {
            eprintln!("GITHUB_TOKEN not set, so not submitting releases to github");
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        build()?;
    } else {
        let action = args.get(1).unwrap();

        match action.as_str() {
            "build" => build()?,
            "release" => release()?,
            _ => {
                eprintln!("Unknown action: {}", action);
            }
        }
    }

    Ok(())
}
