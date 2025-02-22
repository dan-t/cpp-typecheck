#![allow(dead_code)]

extern crate serde_json;
extern crate tempfile;
extern crate atomicwrites;
extern crate dirs as extern_dirs;

#[macro_use]
extern crate error_type;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate clap;

use std::io::{Write, stderr};
use std::process::exit;
use std::path::PathBuf;
use ct_result::CtResult;
use config::{Config, SourceFile, CmdCaching};
use cmd::Cmd;

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
    let config = Config::from_command_args()?;
    let cmd = get_cmd(&config)?;
    if config.preprocess {
        cmd.preprocess(&config.compiler)
    } else {
        cmd.typecheck(&config.compiler)
    }
}

fn get_cmd(config: &Config) -> CtResult<Cmd> {
    let source_file = &config.source_file;
    match *source_file {
        SourceFile::FromArg { ref cpp_file, .. } | SourceFile::FromHeader { ref cpp_file, .. } => {
            get_cmd_from_files(cpp_file, &config.db_files, &config.cmd_caching)
        }

        SourceFile::FromHeaderWithTmpSource { ref temp_cpp_file, ref cmd_cpp_file, .. } => {
            let cmd = get_cmd_from_files(cmd_cpp_file, &config.db_files, &config.cmd_caching)?;
            Ok(cmd.replace_cpp_file(temp_cpp_file.path()))
        }
    }
}

fn get_cmd_from_files(cpp_file: &PathBuf, db_files: &[PathBuf], cmd_caching: &CmdCaching) -> CtResult<Cmd> {
    match cmd_caching {
        CmdCaching::None => {
            Cmd::from_databases(cpp_file, db_files)
        }

        CmdCaching::Normal => {
            if let Some(cmd) = Cmd::from_cache(&cpp_file)? {
                return Ok(cmd);
            }
            let cmd = Cmd::from_databases(&cpp_file, db_files)?;
            cmd.write_to_cache()?;
            Ok(cmd)
        }

        CmdCaching::Recache => {
            let cmd = Cmd::from_databases(&cpp_file, db_files)?;
            cmd.write_to_cache()?;
            Ok(cmd)
        }
    }
}

