// This file is part of perspektiv, a userspace daemon for graphically reporting
// system events.
// Copyright © 2018  Henrik Laxhuber <henrik@laxhuber.com>
//
// perspektiv is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License, version 3, as published by the
// Free Software Foundation.
//
// This program is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
// PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;
extern crate stderrlog;

// For Config
#[macro_use]
extern crate serde_derive;

extern crate gdk;
extern crate glib;
extern crate gtk;

// UI library in ../threlm/
extern crate threlm;
use threlm::Threlm;

// Currently only used by x11_backlight, though through ui::Window::connect
extern crate gdk_sys;

// Currently only used by alsa
extern crate libc;

mod config;
mod subscribable;
mod ui;

use config::Config;

// MODULES
#[cfg(feature = "alsa_volume")]
mod alsa_volume;
#[cfg(feature = "x11_backlight")]
mod x11_backlight;

// error if no modules were selected (this is the default)
#[cfg(not(any(feature = "alsa_volume", feature = "x11_backlight")))]
compile_error!("You should select some modules that you want to use. See the README.md for more information on how to do that.");

lazy_static! {
    static ref CONFIG: Config = config::read();
}

fn main() {
    stderrlog::new().module(module_path!()).init().unwrap();
    gtk::init().expect("Failed to initialise GTK.");

    let _app = Threlm::new(ui::Window::new(&CONFIG));

    gtk::main();
}
