#![allow(dead_code)]

extern crate serde_json;

#[macro_use]
extern crate error_type;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate clap;

use std::fs::File;
use std::io::Read;
use std::io::{Write, stderr};
use std::process::exit;
use std::path::{Path, PathBuf};
use serde_json::Value;
use ct_result::{CtResult, CtError};
use config::Config;
use cmd::Cmd;

mod ct_result;
mod dirs;
mod config;
mod cmd;

fn main() {
    execute().unwrap_or_else(|err| {
        let _ = writeln!(&mut stderr(), "{}", err);
        exit(1);
    });
}

fn execute() -> CtResult<()> {
    let config = try!(Config::from_command_args());
    let cmd = try!(get_cmd(&config.cpp_file, &config.db_files));
    try!(cmd.exec());
    Ok(())
}

fn get_cmd(cpp_file: &Path, db_files: &[PathBuf]) -> CtResult<Cmd> {
    if let Some(cmd) = try!(Cmd::from_cache(cpp_file)) {
        return Ok(cmd);
    }

    let mut file_buffer = String::new();

    for db_file in db_files {
        let mut file = try!(File::open(db_file));
        file_buffer.clear();
        try!(file.read_to_string(&mut file_buffer));

        let json_value: Value = try!(serde_json::from_str(&file_buffer));

        let objs = try!(json_value.as_array()
            .ok_or(CtError::from(format!("Expected a json array but got: '{}'", json_value))));

        for obj in objs {
            let obj = try!(obj.as_object()
                .ok_or(CtError::from(format!("Expected a json object but got: '{}'", obj))));

            let cmd = try!(Cmd::from_json_obj(obj));
            if cmd.has_cpp_file(cpp_file) {
                try!(cmd.write_to_cache());
                return Ok(cmd);
            }
        }
    }

    Err(CtError::from(format!("Couldn't find C++ source file '{:?}' in compilation databases '{:?}'!", cpp_file, db_files)))
}
