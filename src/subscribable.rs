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
                        println!("Could not create polling function for module `{}`: {}", module_name, msg);
                        return;
                    }
                };
                let mut err_count: usize = 0;
                loop {
                    match f() {
                        Some(Ok(msg)) => {
                            err_count = 0;
                            if actor.tell(msg).is_err() {
                                println!("Terminating `{}` because the subscribing ui widget has been dropped.",
                                         module_name);
                                return;
                            }
                        }
                        Some(Err(e)) => {
                            println!("Module `{}` encountered an error:\n  {}", module_name, e.message);
                            if e.fatal {
                                println!("  This is a fatal error; terminating the module!");
                                return;
                            } else {
                                err_count += 1;
                                if err_count >= 3 {
                                    println!("  This is the third non-fatal error in a row; \
                                              terminating the module!");
                                }
                            }
                        }
                        None => {continue;}
                    }
                }
            })
            .unwrap();
    }

    fn poll_factory(
        _params: Self::Params,
    ) -> Result<Box<FnMut() -> Option<Result<ui::Msg, Error>>>, String>;
}
