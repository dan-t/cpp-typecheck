use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use ct_result::{CtResult, CtError};

lazy_static! {
    static ref CMD_CACHE_DIR: CtResult<PathBuf> = {
        env::home_dir()
            .ok_or(CtError::from("Couldn't read home directory!"))
            .map(|d| d.join(".cpp_typecheck")
                      .join("cache")
                      .join("cmds"))
    };
}

pub fn cmd_cache_dir() -> CtResult<&'static Path> {
    match *CMD_CACHE_DIR {
        Ok(ref dir) => {
            if ! dir.is_dir() {
                try!(fs::create_dir_all(&dir));
            }

            Ok(dir)
        },

        Err(ref err) => { Err(err.clone()) }
    }
}
