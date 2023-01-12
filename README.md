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
