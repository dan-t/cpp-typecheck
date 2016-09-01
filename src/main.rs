#![allow(dead_code)]

extern crate serde_json;
extern crate tempfile;

#[macro_use]
extern crate error_type;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate clap;

use std::io::{Write, stderr};
use std::process::exit;
use ct_result::{CtResult, CtError};
use config::{Config, SourceFile};
use cmd::{Cmd, has_only_type_checking_flag};

#[macro_use]
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
    let cmd = try!(get_cmd(&config));

    try!(if let Some(ref compiler) = config.compiler {
        cmd.exec_with(&compiler)
    } else {
        cmd.exec()
    });

    Ok(())
}

fn get_cmd(config: &Config) -> CtResult<Cmd> {
    let source_file = &config.source_file;
    match *source_file {
        SourceFile::FromArg { ref cpp_file, .. } | SourceFile::FromHeader { ref cpp_file, .. } => {
            if ! config.no_cache && ! config.force_recache {
                if let Some(cmd) = try!(Cmd::from_cache(&cpp_file)) {
                    return Ok(cmd);
                }
            }

            let cmd = try!(Cmd::from_databases(&cpp_file, &config.db_files));
            if ! config.no_cache {
                try!(cmd.write_to_cache());
            }

            Ok(cmd)
        },

        SourceFile::FromHeaderWithTmpSource { ref command, .. } => {
            let compiler = if let Some(ref compiler) = config.compiler {
                compiler.clone()
            } else {
                try!(command.get_compiler()).to_string()
            };

            if has_only_type_checking_flag(&compiler) {
                Ok(command.clone())
            } else {
                Err(CtError::from(format!("Unsupported compiler '{}' for type checking a header without a C++ source file!", compiler)))
            }
        }
    }
}
