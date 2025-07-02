
use std::env;
use std::fs;
use std::path::PathBuf;

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
}
