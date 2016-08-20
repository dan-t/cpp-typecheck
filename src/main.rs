//#![feature(custom_derive, plugin)]
//#![plugin(serde_macros)]

extern crate serde_json;

use std::fs::File;
use std::io::Read;
use std::io::{Write, stderr};
use std::process::{Command, exit};
use std::env::args;
use std::path::PathBuf;
use serde_json::Value;

fn main() {
    let mut args = std::env::args();
    args.next();

    let cpp_file = match args.next() {
        Some(arg) => PathBuf::from(arg),
        None      => {
            stderr().write(b"Missing cpp source file argument!\n").unwrap();
            exit(1);
        }
    };

    if cpp_file.is_relative() {
        stderr().write(b"cpp source file has to have an absolute path!\n").unwrap();
        exit(1);
    }

    let mut cmd_str: Option<String> = None;
    {
        let mut contents = String::new();

        'loops: for db_file in args {
            let mut file = File::open(db_file).unwrap();
            contents.clear();
            file.read_to_string(&mut contents).unwrap();

            let json_value: Value = serde_json::from_str(&contents).unwrap();
            let objs = json_value.as_array().unwrap();
            for obj in objs {
                let obj = obj.as_object().unwrap();

                let file = {
                    let f = PathBuf::from(obj.get("file").unwrap().as_str().unwrap());
                    if f.is_relative() {
                        let dir = PathBuf::from(obj.get("directory").unwrap().as_str().unwrap());
                        dir.join(f)
                    } else {
                        f
                    }
                };

                if cpp_file == file {
                    cmd_str = Some(String::from(obj.get("command").unwrap().as_str().unwrap()));
                    break 'loops;
                }
            }
        }
    }

    if cmd_str.is_none() {
        stderr()
            .write_fmt(format_args!("Couldn't find source file '{:?}' in compilation database!\n", cpp_file))
            .unwrap();

        exit(1);
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

    cmd.spawn()
       .expect(&format!("Command execution failed:\n{}\n", cmd_str));
}
