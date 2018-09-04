#![feature(trace_macros)]

#[macro_use]
extern crate lazy_static;

extern crate dirs;
#[macro_use]
extern crate serde_derive;
extern crate toml;

extern crate gdk;
extern crate glib;
extern crate gtk;

// UI library in ../threlm/
extern crate threlm;

// Currently only used by x11_backlight
extern crate gdk_sys;

// Currently only used by alsa
extern crate libc;

use threlm::Threlm;

mod config;
mod subscribable;
mod ui;

use config::Config;

// MODULES
#[cfg(feature = "alsa_volume")]
mod alsa_volume;
#[cfg(feature = "x11_backlight")]
mod x11_backlight;

lazy_static! {
    static ref CONFIG: Config = config::read();
}

fn main() {
    gtk::init().expect("Failed to initialise GTK. Perspektiv is a GUI app and needs GTK to work!");

    let _app = Threlm::new(ui::Window::new(&CONFIG));

    gtk::main();
}
