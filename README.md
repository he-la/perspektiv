# perspektiv

perspektiv (engl. _spotting scope_) is designed to be a lightweight userland
daemon for graphically reporting system events such as monitor brightness
changes, audio volume changes etc.

![screenshot](screenshot.png)

I created perspektiv to go hand in hand with
the hotkey setup on my laptop: I wanted something that fits well with the
graphical theme of my system, was lightweight to comfortably have running as a
daemon in the background, and was hackable enough to easily add functionality as
I desired.

However, this means that perspektiv only has stuff that I myself cared about
enough to implement. Currently, that's just the following:
- Monitor brightness with X11/RandR
- Audio volume/mute with ALSA

I have a few more things on my roadmap:
- Bluetoth toggle
- Wifi toggle

If there are any additional things that you would like to have, you can easily
implement them yourself! See [the contributing guide](CONTRIBUTING.md) for more
information and hopefully enough documentation to get you started.

perspektiv has been tested on Arch Linux. Your milage might vary on other
distrubutions, although I believe that the core of perspektiv should work
regardless of what platform you're on (as long as you're using linux).

## Installation

perspektiv is written in rust, and to build it, you need a rust compiler. I
realise that this is quite a large requirement and hope to provide some prebuilt
binaries soon^TM.

You can easily set up the rust toolchain with [rustup](https://rustup.rs). Once
you have that, clone this repository and run:

```shell
cargo build --release --features "feature_list"
```

where `feature_list` is a space-separated list of the modules that you would
like to include. You can pick from the following modules:

- `x11_backlight`: Show a popup with the monitor brightness when it is changed
- `alsa_volume`: Show a popup with the current volume or mute status when they
  are changed
  
The binary will be created as `./target/release/perspektiv`. Drop this file into
your $PATH somewhere (for a system-wide installation, something like `/bin` will
probaly work well) and set up your system to run the binary on boot.

If you use systemd, you can use the provided [systemd unit](perspektiv.service).
You will probably want to edit the `ExecStart` line in that file to point to the
location where you installed perspektiv. Drop the file into e.g.
`~/.config/systemd/user/perspektiv.service` and run `systemctl --user enable
perspektiv.service && systemctl --user start perspektiv.service`.

Note that runnning perspektiv for the first time will create a default
configuration file under `~/.config/perspektiv`.

## Configuration

perspektiv aims to be heavily customizable so that it fits on any system with
any design. This is partially achieved by using GTK to integrate well with the
system theme. But not everything can be done with CSS, and not every GTK theme
works well for perspektiv. This is why you have the [toml
configuration file][0] at your disposal, where you can:
- Set custom CSS files
- Change dimensions such as width, padding, margins, and more
- Change how information is presented

See the [default configuration][0] file for more information on how
to do that.

[0]: default.toml
