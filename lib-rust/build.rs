fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=../target/vcpkg/installed/vcpkg/status");

    let tesseract_found = vcpkg::find_package("tesseract");
    let leptonica_found = vcpkg::find_package("leptonica");

    if tesseract_found.is_err() {
        println!("cargo:warning=Missing vcpkg dependency: tesseract");
    }

    if leptonica_found.is_err() {
        println!("cargo:warning=Missing vcpkg dependency: leptonica");
    }

    if tesseract_found.is_err() || leptonica_found.is_err() {
        eprintln!("Please install the missing dependencies with cargo-vcpkg");
        eprintln!("Run the following commands:");
        eprintln!("");
        eprintln!("cargo install cargo-vcpkg");
        eprintln!("cargo vcpkg build");
        eprintln!("");
        eprintln!("Then try cargo build again.");
        panic!("Missing vcpkg dependencies");
    }
}
