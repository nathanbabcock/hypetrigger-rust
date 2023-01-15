# Hypetrigger ⚡

GPU-accelerated computer vision pipeline for native and web.

**Links**
| [hypetrigger.io](https://hypetrigger.io)
| [crates.io](https://crates.io/crates/hypetrigger)

## Getting started (Cargo)

```console
cargo install hypetrigger
```

```rs
use hypetrigger::{Hypetrigger, SimpleTrigger};

fn main() {
    Hypetrigger::new()
        .test_input()
        .add_trigger(SimpleTrigger::new(|frame| {
            println!("received frame {}: {}x{}",
                frame.frame_num,
                frame.image.width(),
                frame.image.height()
            );
            // Now do whatever you want with it...
        }))
        .run();
}
```

## Getting started (NPM)

> Coming soon 🚧

## Native Dependencies

### Visual Studio Build Tools

- Must install "Visual Studio Build Tools 2017" -- current version 15.9.50
- Must ALSO install "Visual Studio Community 2019" with the following components
  of "Desktop development with C++" workload:
  - MSVC v142 - VS 2019 C++ x65/x86 build tools
  - C++ CMake tools for Windows
  - C++ ATL for latest v142 build tools

> Build tools are required by Cargo, VS 2019 is used to compile & link native dependencies

### Tensorflow

Should be installed automatically by Cargo.

### Tesseract

Install manually with `vcpkg`: ([Github](https://github.com/microsoft/vcpkg#quick-start-windows))

```sh
git clone https://github.com/microsoft/vcpkg
cd vcpkg
./bootstrap-vcpkg.bat
./vcpkg integrate install
./vcpkg install leptonica:x64-windows-static-md
./vcpkg install tesseract:x64-windows-static-md
```

Also install **`libclang`** included in the [latest LLVM release](https://github.com/llvm/llvm-project/releases).

Current version: <https://github.com/llvm/llvm-project/releases/download/llvmorg-14.0.6/LLVM-14.0.6-win64.exe>

Useful links:

- <https://github.com/charlesw/tesseract/wiki/Compiling-Tesseract-and-Libleptonica-(using-vcpkg)>
- <https://sunnysab-cn.translate.goog/2020/10/06/Use-Tesseract-To-Identify-Captchas-In-Rust/?_x_tr_sl=zh-CN&_x_tr_tl=en&_x_tr_hl=en&_x_tr_pto=sc>
