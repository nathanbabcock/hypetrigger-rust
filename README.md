# Hypetrigger ⚡

Perform efficient per-frame operations on streaming video.

**Links**
| [hypetrigger.io](https://hypetrigger.io)
| [Github](https://github.com/nathanbabcock/hypetrigger)
| [crates.io](https://crates.io/crates/hypetrigger)
| [docs.rs](https://docs.rs/hypetrigger)
| [npm](https://www.npmjs.com/package/hypetrigger)

## Architecture diagram

### Simple version:

```txt
Video → FFMPEG → Tensorflow/Tesseract/Custom → Callback
```

### Annotated version:

```txt
                                 metadata,
                                 progress,
                                  errors
                                    ▲
                                    │
                            ┌───────┴───┐   ┌────────────┐
                         ┌──► stderr    │ ┌─► tesseract  ├─────► callback
                         │  └───────────┘ │ └────────────┘
                         │                │
 ┌─────────────┐ ┌───────┴┐ ┌───────────┐ │ ┌────────────┐
 │ Input Video ├─► ffmpeg ├─► stdout    ├─┼─► tensorflow ├─────► callback
 └─────────────┘ └───────┬┘ └───────────┘ │ └────────────┘
  - Video files          │                │
  - Static images        │  ┌───────────┐ │ ┌────────────────┐
  - HTTP URLs            └──► stdin     │ └─► custom trigger ├─► callback
  - Live streams            └───────┬───┘   └────────────────┘
  - Desktop capture                 │
  - Webcam video                    ▼
                               pause/stop
                                commands

 └─────────────┘ └───────────────────────┘ └─────────────────┘   └──────┘
   MEDIA SOURCE        VIDEO DECODING        COMPUTER VISION     CALLBACK
```

## Getting started (Rust)

```console
cargo add hypetrigger
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

## Getting started (Typescript)

Browser and Node are supported through a WASM compilation of the image
preprocessing code with the excellent
[Photon.js](https://github.com/silvia-odwyer/photon) image processing library.
After that [Tesseract.js](https://github.com/naptha/tesseract.js/) is used for
the text recognition.

```console
npm add hypetrigger
```

```ts
const videoElem = document.getElementById('video')
const pipeline = new Hypetrigger(videoElem)
  .addTrigger(frame => {
    console.log({ frame })
    // do whatever you want with the frame
  })
  .autoRun()
```

### Limitations

The TS version is not a fully featured port of the Rust library; rather it is more of a
parallel toolkit with a subset of the full functionality.

There are no [Tensorflow.js](https://github.com/tensorflow/tfjs) bindings yet,
and frames are pulled directly from media sources, eliminating the useage of FFMPEG
completely.

For more information, see this page in the docs: [Using with other languages](/docs/src/other-languages).

## In-depth example (Rust)

> This is slightly simplified sample code. It won't immediately compile and work
> without the right input source and parameters, but it illustrates how to use
> the API to solve a real-world problem.

**Problem statement:** Detect when a goal is scored in live video of a World Cup
match. ⚽

### `Cargo.toml`

```toml
[dependencies]
hypetrigger = { version = "0.2.0", features = ["photon", "tesseract"] }
# enable the `tesseract` feature and its `photon` dependency for image processing
# see the "Native Dependencies" section in `README.md` if you have trouble building
```

### `main.rs`

```rs
use hypetrigger::{Hypetrigger, SimpleTrigger};
use hypetrigger::photon::{Crop, ThresholdFilter};
use hypetrigger::tesseract::{TesseractTrigger, init_tesseract}

fn main() {
    // First, init a Tesseract instance with default params
    let tesseract = init_tesseract(None, None)?;

    // Initialize some state (use an Rc or Arc<Mutex> if needed)
    let mut last_score: Option<u32> = None;

    // Create a trigger that will be used to detect the scoreboard
    let trigger = TesseractTrigger {
        tesseract, // pass in the Tesseract instance

        // Identify the rectangle of the video that contains
        // the scoreboard (probably the bottom-middle of the
        // screen)
        crop: Some(Crop {
            left_percent: 25.0,
            top_percent: 25.0,
            width_percent: 10.0,
            height_percent: 10.0,
        }),

        // Filter the image to black and white
        // based on text color. This preprocessing improves Tesseract's
        // ability to recognize text. You could replace it with
        // your own custom preprocessing, like edge-detection,
        // sharpening, or anything else.
        threshold_filter: Some(ThresholdFilter {
          r: 255,
          g: 255,
          b: 255,
          threshold: 42,
        }),

        // Attach the callback which will run on every frame with the
        // recognized text
        callback: |result| {
          let parsed_score: u32 = result.text.parse();
          if parsed_score.is_err() {
            return Ok(()) // no score detected; continue to next frame
          }

          // Check for different score than last frame
          if last_score.unwrap() == parsed_score.unwrap() {
            println!("A goal was scored!");

            // Do something:
            todo!("celebrate 🎉");
            todo!("tell your friends");
            todo!("record a clip");
            todo!("send a tweet");
            todo!("cut to commercial break");
          }

          // Update state
          last_score = parsed_score;
        },

        // Using this option will pause after every frame,
        // so you can see the effect of your crop + filter settings
        enable_debug_breakpoints: false,
    };

    // Create a pipeline using the input video and your customized trigger
    Hypetrigger::new()
        .input("https://example.com/world-cup-broadcast.m3u8")
        .add_trigger(trigger)
        .run();

    // `run()` will block the main thread until the job completes,
    // but the callback will be invoked in realtime as frames are processed!
}
```

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

#### Useful links:

- <https://github.com/charlesw/tesseract/wiki/Compiling-Tesseract-and-Libleptonica-(using-vcpkg)>
- <https://sunnysab-cn.translate.goog/2020/10/06/Use-Tesseract-To-Identify-Captchas-In-Rust/?_x_tr_sl=zh-CN&_x_tr_tl=en&_x_tr_hl=en&_x_tr_pto=sc>

### `wasm-pack`

```sh
cargo install wasm-pack
```

If you get OpenSSL/Perl errors like this:

> This perl implementation doesn't produce Windows like paths

Try running once from windows `cmd.exe` instead of VSCode integrated terminal
and/or git bash.
