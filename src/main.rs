//#![feature(custom_derive, plugin)]
//#![plugin(serde_macros)]

extern crate serde_json;

#[macro_use]
extern crate error_type;

use std::fs::File;
use std::io::Read;
use std::io::{Write, stderr};
use std::process::{Command, exit};
use std::env::args;
use std::path::PathBuf;
use serde_json::Value;
use ct_result::{CtResult, CtError};

mod ct_result;

fn main() {
    execute().unwrap_or_else(|err| {
        let _ = writeln!(&mut stderr(), "{}", err);
        exit(1);
    });
}

fn execute() -> CtResult<()> {
    let mut args = std::env::args();
    args.next();

    let cpp_file = match args.next() {
        Some(arg) => PathBuf::from(arg),
        None      => return Err(CtError::from("Missing C++ source file argument!"))
    };

    if cpp_file.is_relative() {
        return Err(CtError::from("C++ source file has to have an absolute path!"));
    }

    let mut cmd_str: Option<String> = None;
    {
        let mut contents = String::new();

        'loops: for db_file in args {
            let mut file = try!(File::open(db_file));
            contents.clear();
            try!(file.read_to_string(&mut contents));

            let json_value: Value = try!(serde_json::from_str(&contents));

            let objs = try!(json_value.as_array()
                .ok_or(CtError::from(format!("Expected a json array but got: '{}'", json_value))));

            for obj in objs {
                let obj = try!(obj.as_object()
                    .ok_or(CtError::from(format!("Expected a json object but got: '{}'", obj))));

                let file = {
                    let f = PathBuf::from(try!(try!(obj.get("file")
                        .ok_or(CtError::from(format!("Couldn't find entry 'file' in json object: '{:?}'", obj))))
                        .as_str()
                        .ok_or(CtError::from(format!("Couldn't get entry 'file' as str from json object: '{:?}'", obj)))));

                    if f.is_relative() {
                        let dir = PathBuf::from(try!(try!(obj.get("directory")
                            .ok_or(CtError::from(format!("Couldn't find entry 'directory' in json object: '{:?}'", obj))))
                            .as_str()
                            .ok_or(CtError::from(format!("Couldn't get entry 'directory' as str from json object: '{:?}'", obj)))));

                        dir.join(f)
                    } else {
                        f
                    }
                };

                if cpp_file == file {
                    cmd_str = Some(String::from(try!(try!(obj.get("command")
                        .ok_or(CtError::from(format!("Couldn't find entry 'command' in json object: '{:?}'", obj))))
                        .as_str()
                        .ok_or(CtError::from(format!("Couldn't get entry 'command' as str from json object: '{:?}'", obj))))));

                    break 'loops;
                }
            }
        }
    }

    if cmd_str.is_none() {
        return Err(CtError::from(format!("Couldn't find C++ source file '{:?}' in compilation database!", cpp_file)));
    }

    let cmd_str = cmd_str.unwrap();

    let mut parts = cmd_str.split(" ");
    let mut cmd = Command::new(parts.next().unwrap());
    for p in parts {
        if p.is_empty() {
            continue;
        }

        cmd.arg(p.replace("\\", ""));
    }

    cmd.arg("-fsyntax-only");

    try!(cmd.spawn()
       .map_err(|e| CtError::from(format!("Command execution failed: {}, because: {}", cmd_str, e))));

    Ok(())
}
