macro_rules! err_if {
    ($test:expr, $message:expr) => {
        if $test {
            return Err(format!(
                "{msg}.\n  L{line} in {module}: Did not expect `{test}`",
                msg = &$message,
                line = line!(),
                module = module_path!(),
                test = &stringify!($test),
            ));
        }
    };
}

macro_rules! err_expect {
    ($test:expr, $message:expr) => {
        if !$test {
            return Err(format!(
                "{msg}.\n  L{line} in {module}: Expected `{assertion}`",
                msg = &$message,
                line = line!(),
                module = module_path!(),
                assertion = &stringify!($test),
            ));
        }
    };
}

#[allow(unused_macros)]
macro_rules! printdbg {
    ($e:expr) => {
        println!("{}: {:#?}", stringify!($e), $e)
    };
}
