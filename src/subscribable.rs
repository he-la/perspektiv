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

use std::thread;

use threlm;
use ui;

pub struct Error {
    message: String,
    fatal: bool,
}
impl Error {
    pub fn new<S>(message: S, fatal: bool) -> Self
    where
        S: Into<String>,
    {
        Error {
            message: message.into(),
            fatal,
        }
    }
}

pub trait Subscribable {
    type Params: Send + 'static;

    fn subscribe(
        actor: threlm::Actor<ui::Window>,
        module_name: &'static str,
        params: Self::Params,
    ) {
        thread::Builder::new()
            .name(module_name.to_string())
            .spawn(move || {
                let mut f = match Self::poll_factory(params) {
                    Ok(f) => f,
                    Err(msg) => {
                        error!(
                            "Could not create polling function for module `{}`:\n  {}",
                            module_name, msg
                        );
                        return;
                    }
                };
                let mut err_count: usize = 0;
                loop {
                    match f() {
                        Ok(Some(msg)) => {
                            err_count = 0;
                            if actor.tell(msg).is_err() {
                                error!("Terminating `{}` because the subscribing ui widget has been dropped.",
                                         module_name);
                                return;
                            }
                        }
                        Err(e) => {
                            let mut terminate = false;
                            error!(
                                "Module `{}` encountered an error:\n  {}\n  {}",
                                module_name,
                                e.message,
                                if e.fatal {
                                    terminate = true;
                                    "This is a fatal error; terminating the module!"
                                } else {
                                    err_count += 1;
                                    if err_count >= 3 {
                                        terminate = true;
                                        "This is the third non-fatal error in a row; terminating the module!"
                                    } else {
                                        "Attempting to continue execution of the module."
                                    }
                                }
                            );
                            if terminate {
                                return;
                            }
                        }
                        Ok(None) => {
                            continue;
                        }
                    }
                }
            })
            .unwrap();
    }

    fn poll_factory(
        _params: Self::Params,
    ) -> Result<Box<FnMut() -> Result<Option<ui::Msg>, Error>>, String>;
}
