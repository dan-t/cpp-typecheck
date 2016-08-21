#![allow(dead_code)]

extern crate serde_json;

#[macro_use]
extern crate error_type;

#[macro_use]
extern crate lazy_static;

use std::fs::{File, OpenOptions};
use std::io::Read;
use std::io::{Write, stderr};
use std::process::{Command, exit};
use std::env::args;
use std::path::{Path, PathBuf};
use std::hash::{Hash, SipHasher, Hasher};
use serde_json::Value;
use ct_result::{CtResult, CtError};
use dirs::cmd_cache_dir;

mod ct_result;
mod dirs;

fn main() {
    execute().unwrap_or_else(|err| {
        let _ = writeln!(&mut stderr(), "{}", err);
        exit(1);
    });
}

fn execute() -> CtResult<()> {
    let mut args = std::env::args();
    args.next();

    let cpp_file = {
        let file = match args.next() {
            Some(arg) => PathBuf::from(arg),
            None      => return Err(CtError::from("Missing C++ source file argument!"))
        };

        if file.is_relative() {
            return Err(CtError::from("C++ source file has to have an absolute path!"));
        }

        file
    };

    let db_files = {
        let mut files = Vec::<PathBuf>::new();
        for arg in args {
            files.push(PathBuf::from(arg));
        }

        if files.is_empty() {
            return Err(CtError::from("Missing clang compilation database argument!"));
        }

        files
    };

    let cmd_str = try!(get_command_str(&cpp_file, &db_files));
    let mut cmd = try!(build_command(&cmd_str));

    try!(cmd.spawn()
       .map_err(|e| CtError::from(format!("Command execution failed: {}, because: {}", cmd_str, e))));

    Ok(())
}

fn get_command_str(cpp_file: &Path, db_files: &[PathBuf]) -> CtResult<String> {
    if let Some(cmd_str) = try!(find_command_str_cache(cpp_file)) {
        return Ok(cmd_str);
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
                let cmd_str = String::from(try!(try!(obj.get("command")
                    .ok_or(CtError::from(format!("Couldn't find entry 'command' in json object: '{:?}'", obj))))
                    .as_str()
                    .ok_or(CtError::from(format!("Couldn't get entry 'command' as str from json object: '{:?}'", obj)))));

                try!(write_command_str_cache(cpp_file, &cmd_str));
                return Ok(cmd_str);
            }
        }
    }

    Err(CtError::from(format!("Couldn't find C++ source file '{:?}' in compilation database!", cpp_file)))
}

fn build_command(cmd_str: &str) -> CtResult<Command> {
    if cmd_str.is_empty() {
        return Err(CtError::from("Unexpected empty command string!"));
    }

    let mut parts = cmd_str.split(" ");
    let compiler = try!(parts.next()
        .ok_or(CtError::from("Unexpected empty parts after command string split!")));

    if ! compiler.contains("gcc")
        && ! compiler.contains("g++")
        && ! compiler.contains("clang")
        && ! compiler.contains("clang++") {
        return Err(CtError::from(format!(
            "Unsupported compiler for typecheck: '{}'! Currently supported are 'gcc/g++' and 'clang/clang++'!", compiler)));
    }

    let mut cmd = Command::new(compiler);
    for p in parts {
        if p.is_empty() {
            continue;
        }

        cmd.arg(p.replace("\\", ""));
    }

    cmd.arg("-fsyntax-only");
    Ok(cmd)
}

fn find_command_str_cache(cpp_file: &Path) -> CtResult<Option<String>> {
    let cache_dir = try!(cmd_cache_dir());
    let cache_file = cache_dir.join(compute_hash(cpp_file));
    if ! cache_file.is_file() {
        return Ok(None);
    }

    let mut file = try!(File::open(cache_file));
    let mut cmd_str = String::new();
    try!(file.read_to_string(&mut cmd_str));
    return Ok(Some(cmd_str));
}

fn write_command_str_cache(cpp_file: &Path, cmd_str: &str) -> CtResult<()> {
    let cache_dir = try!(cmd_cache_dir());
    let cache_file = cache_dir.join(compute_hash(cpp_file));
    if cache_file.is_file() {
        return Ok(());
    }

    let mut file = try!(OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(cache_file));

    let _ = try!(file.write_fmt(format_args!("{}", cmd_str)));
    Ok(())
}

fn compute_hash(cpp_file: &Path) -> String {
    let mut hasher = SipHasher::new();
    cpp_file.hash(&mut hasher);
    hasher.finish().to_string()
}
