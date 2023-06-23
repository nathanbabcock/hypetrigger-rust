use std::process::{Command, Stdio};

fn cargo_vpkg_is_installed() -> bool {
    Command::new("cargo")
        .arg("vcpkg")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if !cargo_vpkg_is_installed() {
        eprintln!("Native dependencies must be built with cargo-vcpkg");
        eprintln!("Run the following commands:");
        eprintln!("");
        eprintln!("cargo install cargo-vcpkg");
        eprintln!("cargo vcpkg build");
        eprintln!("");
        eprintln!("Then try cargo build again.");
        panic!();
    }
}
