
use std::{ env, fs, path::PathBuf, process::Command };



fn main()
{
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Copy linker script to output directory
    fs::copy("link.ld", out_dir.join("link.ld")).unwrap();

    // Tell cargo to re-run this build script if link.ld changes
    println!("cargo:rerun-if-changed=link.ld");

    // Tell rustc to link with our script
    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rustc-link-arg=-Tlink.ld");

    // Capture the build time of the kernel.
    let build_time_out = Command::new("date")
        .args(&["+%Y-%m-%d %H:%M:%S UTC"])
        .output()
        .expect("Failed to execute date command");
    let build_time = String::from_utf8_lossy(&build_time_out.stdout).trim().to_string();

    println!("cargo:rustc-env=BUILD_TIME={}", build_time);

    // Set PROFILE based on the build profile.
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=PROFILE={}", profile);
}
