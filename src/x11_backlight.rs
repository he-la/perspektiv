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

// BIG TODO:
// Switch to xcb-rs
extern crate x11;

use std::{ffi::CString, mem::uninitialized, ops::Range, os::raw::*, ptr};

use gdk_sys;

use self::x11::xlib; // also allow scoped access for disambiguation
use self::x11::{xlib::*, xrandr::*};

use subscribable;
use subscribable::{PollFn, Subscribable};
use ui;

pub struct Backlight {
    display: *mut Display,
    backlight: Atom,
    output: RROutput,
    backlight_range: Range<c_long>,
}

impl Backlight {
    /// Initialise the module by connecting to the X11 server and getting handles
    /// for the display and backlight.
    fn new() -> Result<Backlight, String> {
        unsafe {
            let display = XOpenDisplay(ptr::null());
            err_if!(
                display.is_null(),
                "Cannot open default display (maybe no $DISPLAY environment variable set)"
            );

            let mut major: c_int = uninitialized();
            let mut minor: c_int = uninitialized();
            err_expect!(
                XRRQueryVersion(display, &mut major as *mut c_int, &mut minor as *mut c_int) != 0,
                "RandR extension missing"
            );
            err_expect!(
                major > 1 || (major == 1 && minor > 2),
                "RandR version too old"
            );

            // Get atom (numeric ID) for the Backlight property
            let mut backlight_name = CString::new("Backlight").unwrap().into_raw();
            let mut backlight = XInternAtom(display, backlight_name, true as i32);
            if backlight != 0 {
                backlight_name = CString::new("BACKLIGHT").unwrap().into_raw();
                backlight = XInternAtom(display, backlight_name, true as i32);
                err_if!(
                    backlight == 0,
                    "Given display has no property `Backlight` or `BACKLIGHT`"
                );
            }
            let _ = CString::from_raw(backlight_name); // back into Rust memory management to free properly

            let root = XDefaultRootWindow(display);
            err_if!(
                root == 0,
                "Cannot get default root window for given display"
            );

            let resources = XRRGetScreenResources(display, root);
            err_if!(
                resources.is_null(),
                "Cannot get xrandr resources for given display and root"
            );

            let output_ptr = (*resources).outputs;
            err_if!(
                output_ptr.is_null(),
                "Cannot get outputs for given xrandr resources"
            );
            let output = *output_ptr;

            let backlight_info = XRRQueryOutputProperty(display, output, backlight);
            err_if!(
                backlight_info.is_null(),
                "Cannot get property `Backlight` for given display and xrandr outputs"
            );

            let backlight_range = Range {
                start: *(*backlight_info).values,
                end: *(*backlight_info).values.offset(1),
            };

            XFree(backlight_info as *mut c_void);
            XRRFreeScreenResources(resources as *mut XRRScreenResources);

            Ok(Backlight {
                display,
                backlight,
                output,
                backlight_range,
            })
        }
    }

    fn get_brightness(&self) -> Result<f64, String> {
        unsafe {
            let mut actual_type: Atom = uninitialized();
            let mut actual_format: c_int = uninitialized();
            let mut n_items: c_ulong = uninitialized();
            let mut bytes_after: c_ulong = uninitialized();
            let mut prop: *mut c_uchar = uninitialized();

            XRRGetOutputProperty(
                self.display,                     // dpy: *mut Display,
                self.output,                      // output: RROutput,
                self.backlight,                   // property: Atom,
                0,                                // offset: c_long,
                4,                                // length: c_long,
                false as i32,                     // _delete: Bool,
                false as i32,                     // pending: Bool,
                0,                                // req_type: Atom,
                &mut actual_type as *mut Atom,    // actual_type: *mut Atom,
                &mut actual_format as *mut c_int, // actual_format: *mut c_int,
                &mut n_items as *mut c_ulong,     // nitems: *mut c_ulong,
                &mut bytes_after as *mut c_ulong, // bytes_after: *mut c_ulong,
                &mut prop as *mut *mut c_uchar,   // prop: *mut *mut c_uchar
            );

            err_expect!(
                actual_type == XA_INTEGER,
                "X11 did not return an integer for the backlight property"
            );
            err_expect!(
                n_items == 1,
                "Got zero or multiple values for backlight property; expected exactly one"
            );
            err_expect!(
                actual_format == 32,
                "Backlight was not a 32-bit value as expected"
            );

            let brightness = *(prop as *const c_long);
            XFree(prop as *mut c_void);

            let brightness: f64 = ((brightness - self.backlight_range.start) as f64)
                / ((self.backlight_range.end - self.backlight_range.start) as f64);
            return Ok(brightness);
        }
    }
}

pub struct Subscription();
impl Subscribable for Subscription {
    type Params = xlib::Window;

    fn poll_factory(window: Self::Params) -> Result<Box<PollFn>, String> {
        let mut backlight = Backlight::new()?;

        // Subscribe to X11 event for (any) RandR Output Property changes on the display.
        // Unfortunately this does not have a status return value, so who know's if it worked?
        unsafe {
            XRRSelectInput(backlight.display, window, RROutputPropertyNotifyMask);
        }

        unsafe extern "C" fn predicate(
            _display: *mut Display,
            event: *mut XEvent,
            arg: *mut c_char,
        ) -> i32 {
            let output = *(arg as *const RROutput); // backlight.output
            let event = *event;

            // No idea where 90 is defined, but that's what highly
            // sophisticated println! brute-force gives me.
            if event.type_ == 90 {
                // is RandR event
                // fine filtering
                let event: XRROutputPropertyNotifyEvent = event.xrr_output_property_notify;
                if event.subtype == RRNotify_OutputProperty && event.output == output {
                    return true as i32;
                }
            }

            return false as i32;
        }

        let mut event: XEvent = unsafe { uninitialized() };
        Ok(Box::new(move || {
            loop {
                unsafe {
                    XIfEvent(
                        backlight.display,
                        &mut event as *mut XEvent,
                        Some(predicate),
                        &mut backlight.output as *mut _ as *mut c_char,
                    );
                }
                // The event doesn't contain the new value, so we need to query it
                match backlight.get_brightness() {
                    Ok(brightness) => {
                        return Ok(ui::ShowPercent("", brightness));
                    }
                    Err(msg) => {
                        return Err(subscribable::Error::from(msg));
                    }
                }
            }
        }))
    }
}

// Provided by gdk, but not contained in the gdk_sys crate (to the best of my
// knowledge)
extern "C" {
    pub fn gdk_x11_window_get_xid(window: *const gdk_sys::GdkWindow) -> xlib::Window;
}
