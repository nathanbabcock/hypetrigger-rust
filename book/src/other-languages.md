# Using with other languages

Use Rust if you're looking to get the full power of Hypetrigger out of the box.
There's a few reasons for the choice of language:

- Memory safety guarantees at compile time
- High performance
- Good ecosystem of supporting libraries

FFMPEG is really the V12 engine underneath Hypetrigger that does all the heavy
lifting. In its essence, Hypetrigger is a framework that uses FFMPEG to let you
run arbitrary Rust code on a per-frame basis -- taking care of all the
idiosyncrasies of FFMPEG, including its arcane command-line syntax.

The WebAssembly bindings are mainly useful as an interop/GUI frontend for
configuring specific Triggers, which will then run natively in Rust, and to a
certain extent replicating those results natively. That's how it's used in the
official [Hypetrigger app](https://hypetrigger.io/) (with
[Tauri](https://tauri.app/) to communicate between Rust backend and embedded
browser frontend). However, it doesn't include any FFMPEG functionality so it
can't achieve the same performance as the Rust version. Also, it has no direct
access to run on the GPU, another major bottleneck.

## What you should do if you don't know Rust

### 1. Learn Rust

I didn't know any Rust when I started this project. I'm a web developer, and I
was writing Typescript and React extensively before this project. My
introduction to Rust was through [Tauri](https://tauri.app/) as a lightweight
alternative to Electron. I was able to feel my way through the basics at first
by adding Tauri command handlers (written in the Rust backend of the framework)
and made a lot of progress that way -- I definitely recommend it as a starting
point for web devs looking for an entry point into Rust-land. Although it's a
challenging language (especially understanding lifetimes and borrow checking),
learning it is within the capabilities of any interested and motivated
programmer. Also, it unlocks the door to a whole new level of concurrency and
speed, if you don't already have a language in your toolkit which gives you
this. Personally I always found C++ inaccessible, whereas Rust felt very
familiar with a package manager, language server, and a lot of the accoutrements
of a modern programming language.

Having said that, the API of Hypetrigger is designed in a way that is intended
to be friendly to Rust beginners (since it was written by a Rust beginner),
abstracting away most of the confusion of how data is being passed around
between threads, and, in its simplest usage, just provides you with a per-frame
callback where you can do exactly what you need. That leads into the second
approach to using Hypetrigger if you're not a Rust expert:

### 2. Learn the bare minimum Rust

You do have the option of bailing out of Rust as soon as you possibly can. The
first thing your custom Trigger does could be invoking another binary or sending
a message to a language you're more at home in, passing along the relevant
`Context` and `RawImageData`. Just a few ideas:

- [Spawn a child
  process](https://doc.rust-lang.org/std/process/struct.Child.html) that runs
  Python or NodeJS, taking data through `stdin`
- [Send a websocket message](https://docs.rs/websocket/latest/websocket/) to
  another process in a language you're more comfortable with
- [Emit a Tauri event](https://tauri.app/v1/api/js/event#emit) and do all of the
  processing in browser Javascript

If you can hack your way through the 5-10 lines of code needed to start a child
process (likely by copy-pasting straight from the Rust official docs or
Stackoverflow), then you can be up and running with a relatively minimal
understanding of Rust.

Don't be too pessimistic about performance in these kinds of setups either --
keep in mind that **video decoding** is almost always the performance bottleneck
in these systems. Running that on the GPU (thanks to FFMPEG) is the #1 most
important factor, and the key to unlocking 10x+ realtime speeds. The actual ML
parts, whether it's Tensorflow image classification or Tesseract OCR, will
typically run faster than decoding 1080p/60fps video (mainly due to the fact
that you can afford to sample at a much lower framerate like 1-4 FPS), so your
transport can afford to be a little slower/lower throughput after you have the
raw frames you need.

Not to mention that the support for these kinds of ML libraries in your language
of choice might just be bindings to C libraries anyways (as is the case in the
`tesseract` and `tensorflow` dependencies used in Hypetrigger itself). If
handling the plumbing and business logic in between is easier in &lt;language
X&gt;, go for it. Do whatever makes your life easiest.

### 3. Replicate the Hypetrigger pipeline natively in another language

There's another option if you're feeling ambitious or inspired and you'd like to
get your hands dirty. Since FFMPEG is the powerhouse here, nothing is tying you
down to Rust. As mentioned at the top, the memory safety guarantees and
"fearless concurrency" mantra are major selling points, but re-writing the core
pipeline in a language that you know is 100% possible with comparable
performance. Native NodeJS or Python could spawn FFMPEG, read or pipe raw data
from its stdout to appropriate handlers, and go from there. Dig into the
Hypetrigger docs and source code to understand how data is being passed around
and the small handful of threads involved. If you know what you're doing you
might be able to replicate this in no more than a day or two. If it's your first
time wrestling with FFMPEG it might take a few more, but you'll pick up some
useful media-processing skills for the road.

If you do undertake such an endeavor, please [let the community
know](https://hypetrigger.io/discord) and publish it on Github for the benefit
of the open source community at large. Also, you may open a pull request on this
repo to add a link to your project right here. `hypetrigger-node` and
`hypetrigger-python` would surely be valuable and welcome additions.

### 4. Ask for help

Whether you undertake option 1, 2, or 3 above, or if you're comfortable in Rust
but need to understand the intricacies of the system better, you can ask
questions in the [dev channel on Discord](https://hypetrigger.io/discord) or
[open an issue on Github](https://github.com/nathanbabcock/hypetrigger/issues).

Best of luck!
