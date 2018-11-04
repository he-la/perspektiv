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

extern crate libc;

use libc::*;
use std::{ffi::CString, mem::size_of, mem::uninitialized, borrow::Cow};

use subscribable;
use subscribable::Subscribable;
use ui;

const RFKILL_DEV_PATH: &'static str = "/dev/rfkill";

// Defined in <linux/rfkill.h>:

const RFKILL_EVENT_SIZE_V1: isize = 8;
/**
 * struct rfkill_event - events for userspace on /dev/rfkill
 * @idx: index of dev rfkill
 * @type: type of the rfkill struct
 * @op: operation code
 * @hard: hard state (0/1)
 * @soft: soft state (0/1)
 *
 * Structure used for userspace communication on /dev/rfkill,
 * used for events from the kernel and control to the kernel.
 */
#[repr(C)]
#[repr(packed)]
struct rfkill_event {
    idx: u32,
    type_: rfkill_type,
    op: rfkill_operation,
    soft_blocked: bool,
    hard_blocked: bool,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
#[allow(non_camel_case_types)]
#[allow(unused)]
enum rfkill_type {
    ALL = 0,
    WLAN,
    BLUETOOTH,
    UWB,
    WIMAX,
    WWAN,
    GPS,
    FM,
    NFC,
    NUM_TYPES,
}
/**
 * enum rfkill_operation - operation types
 * @RFKILL_OP_ADD: a device was added
 * @RFKILL_OP_DEL: a device was removed
 * @RFKILL_OP_CHANGE: a device's state changed -- userspace changes one device
 * @RFKILL_OP_CHANGE_ALL: userspace changes all devices (of a type, or all)
 *	into a state, also updating the default state used for devices that
 *	are hot-plugged later.
 */
#[repr(u8)]
#[allow(non_camel_case_types)]
#[allow(unused)]
enum rfkill_operation {
    ADD = 0,
    DEL,
    CHANGE,
    CHANGE_ALL,
}

macro_rules! errno {
    () => {
        *__errno_location()
    };
}

struct RFkill {
    pollfd: pollfd,
}

impl RFkill {
    fn open() -> Result<Self, String> {
        let fd = unsafe { open(CString::new(RFKILL_DEV_PATH).unwrap().as_ptr(), O_RDONLY) };
        err_if!(
            fd < 0,
            format!(
                "Failed to open rfkil device `{}`: rc = {}",
                RFKILL_DEV_PATH, fd
            )
        );

        let pollfd = pollfd {
            fd: fd,
            events: POLLIN | POLLHUP,
            revents: 0,
        };

        Ok(RFkill { pollfd })
    }

    fn poll(&mut self) -> Result<rfkill_event, String> {
        loop {
            let n_events = unsafe { poll(&mut self.pollfd as *mut pollfd, 1, -1) };
            err_if!(
                n_events < 0,
                format!(
                    "Received error `{err}` while polling `{dev}` (poll returned {ret})",
                    err = self.pollfd.revents,
                    dev = RFKILL_DEV_PATH,
                    ret = n_events
                )
            );
            if n_events == 0 {
                continue;
            }

            // Read Event
            let mut event: rfkill_event = unsafe { uninitialized() };
            let len: ssize_t = unsafe {
                read(
                    self.pollfd.fd,
                    &mut event as *mut _ as *mut c_void,
                    size_of::<rfkill_event>(),
                )
            };
            if len < 0 {
                let errno = unsafe { errno!() };
                err_if!(
                    len < 0 && errno != EAGAIN,
                    format!("Error reading rfkill_event: -errno = {}", errno)
                );
                // errno == EAGAIN, which is silently ignored
                continue;
            }
            err_if!(len > RFKILL_EVENT_SIZE_V1, "Wrong size of rfkill event.");
            err_if!(event.type_ as u8 >= rfkill_type::NUM_TYPES as u8,
                    format!("Event type `{:#?}` unkown (maybe this was added in a future version of the linux kernel)",
                            event.type_));

            return Ok(event);
        }
    }
}

impl Drop for RFkill {
    fn drop(&mut self) {
        assert!(unsafe { close(self.pollfd.fd) } == 0);
    }
}

pub struct Subscription();
impl Subscribable for Subscription {
    type Params = ();

    fn poll_factory(_: Self::Params) -> Result<Box<subscribable::PollFn>, String> {
        let mut rfkill = RFkill::open()?;

        Ok(Box::new(move || loop {
            let event: rfkill_event = match rfkill.poll() {
                Ok(event) => event,
                Err(msg) => return Err(subscribable::Error::from(msg)),
            };

            let (icon, label) = match event.type_ {
                rfkill_type::ALL => ("", "rfkill: All"),
                rfkill_type::WLAN => ("", "WiFi"),
                rfkill_type::BLUETOOTH => ("", "Bluetooth"),
                rfkill_type::UWB => ("", "Ultrawideband"), // FIXME: Change icon to something more fitting
                rfkill_type::WIMAX => ("", "WiMAX"),       // FIXME: Change icon to WiMAX logo
                rfkill_type::WWAN => ("", "WWAN"),
                rfkill_type::GPS => ("", "GPS"),
                rfkill_type::FM => ("", "FM"), // FIXME: A radio icon would be better
                rfkill_type::NFC => ("", "NFC"), // FIXME: NFC has a logo
                _ => unreachable!(),
            };
            let mut label = label.to_owned();
            label.push_str(" ");
            label.push_str(if event.hard_blocked || event.soft_blocked {"disabled"} else {"enabled"});

            return Ok(ui::ShowBool(icon, Cow::from(label)));
        }))
    }
}
