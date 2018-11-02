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

extern crate dirs;
extern crate toml;

use std::{
    default::Default, env, error::Error, fs, fs::File, io::prelude::*, path::PathBuf,
    string::String,
};

// Currently unused, but I'm keeping it in case it comes in handy in the future.
// I abandoned this solution because I believe that these semantics should be
// syntactically enforced with proper enum usage etc.
#[allow(unused_macros)]
macro_rules! config_requires {
    (@panic_msg $str:expr, $a:expr, $b:expr) => {
        let a: String = stringify!($a)
            .chars()
            .skip_while(|c| *c != '.')
            .skip(1)
            .collect();
        let b: String = stringify!($b)
            .chars()
            .skip_while(|c| *c != '.')
            .skip(1)
            .collect();
        panic!(format!($str, a = a, b = b))
    };

    (not_both $a:expr, $b:expr) => {
        if $a.is_some() && $b.is_some() {
            config_requires!(@panic_msg
                                     "Configuration option `{a}` conflicts with option `{b}`.
  help: Remove either `{a}` or `{b}` from your configuration.",
                                     $a, $b
                    );
        }
    };

    (one_of $a:expr, $b:expr) => {
        if $a.is_none() && $b.is_none() {
            config_requires!(@panic_msg
                                     "Configuration requires one of `{a}` or `{b}` to be set.",
                                     $a, $b
                    );
        }
    };

    (either $a:expr, $b:expr) => {
        config_requires!(one_of $a, $b);
        config_requires!(not_both $a, $b);
    };
}

const DEFAULT_CONF: &'static str = include_str!("../default.toml");

#[derive(Deserialize)]
pub struct Config {
    pub window: Window,
    pub boolean: Boolean,
    pub percentage: Percentage,
}

#[derive(Deserialize)]
pub struct Window {
    pub width: i32,
    pub height: i32,

    pub margin_horiz: MarginHoriz,
    pub margin_vert: MarginVert,

    pub duration: u32,

    pub padding: u32,
    pub spacing: u32,
    pub css: Option<PathBuf>,
}

#[derive(Deserialize)]
pub struct Boolean {
    pub show_label: bool,
}

#[derive(Deserialize)]
pub struct Percentage {
    pub show_numeric: bool,
}

#[derive(Deserialize)]
#[serde(tag = "anchor", content = "margin")]
pub enum MarginHoriz {
    Left(i32),
    Right(i32),
}

#[derive(Deserialize)]
#[serde(tag = "anchor", content = "margin")]
pub enum MarginVert {
    Top(i32),
    Bottom(i32),
}

pub fn read() -> Config {
    let config_dir: PathBuf = match env::var("PERSPEKTIV_CONFIG") {
        Ok(path) => PathBuf::from(path),
        Err(_) => match dirs::config_dir() {
            Some(ref path) => {
                let config_dir = path.as_path().join("perspektiv");
                fs::create_dir_all(&config_dir) // create if not exists
                    .expect(&format!("Failed to create configuration directory {:?}", config_dir));

                config_dir
            },
            _ => panic!("Cannot obtain config file: No PERSPEKTIV_CONFIG set, and no user home directory found!"),
        }
    };

    let config_path = config_dir.join("config.toml");
    // Read config as TOML string
    let config: String = if !config_path.exists() {
        info!(
            "Config file {:?} does not exist, creating it with default values.",
            config_path
        );
        match File::create(&config_path) {
            Err(e) => error!("Error: {}", e.description()),
            Ok(mut file) => match file.write_all(DEFAULT_CONF.as_bytes()) {
                Err(e) => error!("Error: {}", e.description()),
                Ok(_) => Default::default(), // wrote defaults to file
            },
        }

        DEFAULT_CONF.to_string()
    } else {
        let mut file = File::open(&config_path).expect(&format!(
            "Failed to open configuration file {:?}",
            config_path
        ));
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).expect(&format!(
            "Failed to read configuration file (maybe the file is corrupted):\n  {:?}",
            config_path
        ));

        buffer
    };

    // Parse config
    let mut config: Config = toml::from_str(&config).expect("Failed to parse TOML");

    // Modify config
    // expand css path
    config = match config {
        Config {
            window: Window {
                css: Some(path), ..
            },
            ..
        } => Config {
            window: Window {
                css: Some(expand_path(path, &config_dir)),
                ..config.window
            },
            ..config
        },
        _ => config,
    };

    return config;
}

// TODO: Write tests, check standards compliance
fn expand_path(path: PathBuf, working_dir: &PathBuf) -> PathBuf {
    if path.as_path().starts_with("~/") {
        let path = path.strip_prefix("~/").unwrap();
        dirs::home_dir().unwrap().join(path)
    } else if !path.as_path().starts_with("/") {
        let path = path.strip_prefix("./").unwrap_or(&path);
        working_dir.join(path)
    } else {
        path
    }
}
