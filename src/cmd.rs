use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::hash::{Hash, SipHasher, Hasher};
use std::process::Command;
use serde_json::{Value, Map};
use ct_result::{CtResult, CtError, OrErr};
use dirs::cmd_cache_dir;

/// a compiler command from the clang compilation database
pub struct Cmd {
    /// the working directory for the compiler
    /// command execution
    directory: PathBuf,

    /// the compiler command itself
    command: String,

    /// the C++ source file for the compilation
    file: PathBuf
}

impl Cmd {
    pub fn from_cache(cpp_file: &Path) -> CtResult<Option<Cmd>> {
        let cache_dir = try!(cmd_cache_dir());
        let cache_file = cache_dir.join(compute_hash(cpp_file));
        if ! cache_file.is_file() {
            return Ok(None);
        }

        let mut file = try!(File::open(cache_file));
        let mut cmd_str = String::new();
        try!(file.read_to_string(&mut cmd_str));
        let mut lines = cmd_str.lines();

        let dir = PathBuf::from(try!(lines.next()
            .or_err(format!("Expected directory in first line of string:\n{}", cmd_str))));

        let cmd = String::from(try!(lines.next()
            .or_err(format!("Expected command in second line of string:\n{}", cmd_str))));

        let file = PathBuf::from(try!(lines.next()
            .or_err(format!("Expected file in third line of string:\n{}", cmd_str))));

        Ok(Some(Cmd { directory: dir, command: cmd, file: file }))
    }

    pub fn from_json_obj(obj: &Map<String, Value>) -> CtResult<Cmd> {
        let dir = PathBuf::from(try!(obj.get("directory").and_then(Value::as_str)
            .or_err(format!("Couldn't find string entry 'directory' in json object: '{:?}'", obj))));

        let file = {
            let f = PathBuf::from(try!(obj.get("file").and_then(Value::as_str)
                .or_err(format!("Couldn't find string entry 'file' in json object: '{:?}'", obj))));

            if f.is_relative() {
                dir.join(f)
            } else {
                f
            }
        };

        let cmd = String::from(try!(obj.get("command").and_then(Value::as_str)
            .or_err(format!("Couldn't find string entry 'command' in json object: '{:?}'", obj))))
            .replace("\\", "");

        Ok(Cmd { directory: dir, command: cmd, file: file })
    }

    pub fn write_to_cache(&self) -> CtResult<()> {
        let cache_dir = try!(cmd_cache_dir());
        let cache_file = cache_dir.join(compute_hash(&self.file));

        let mut file = try!(OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(cache_file));

        let _ = try!(file.write_fmt(format_args!("{}\n{}\n{}",
                                                 self.directory.to_string_lossy(),
                                                 self.command,
                                                 self.file.to_string_lossy())));
        Ok(())
    }

    pub fn has_cpp_file(&self, file: &Path) -> bool {
        file == self.file
    }

    pub fn exec(&self) -> CtResult<()> {
        try!((!self.command.is_empty()).or_err("Unexpected empty command string!"));

        let mut parts = self.command.split(" ");
        let compiler = try!(parts.next().or_err("Unexpected empty parts after command string split!"));

        let mut cmd = Command::new(&compiler);
        cmd.current_dir(&self.directory);

        for p in parts {
            if p.is_empty() {
                continue;
            }

            cmd.arg(p);
        }

        let is_gcc = compiler.contains("gcc") || compiler.contains("g++");
        let is_clang = compiler.contains("clang") || compiler.contains("clang++");
        if is_gcc || is_clang {
            cmd.arg("-fsyntax-only");
        }

        try!(cmd.spawn()
            .map_err(|e| CtError::from(format!("Command execution failed: {}, because: {}", self.command, e))));

        Ok(())
    }
}

fn compute_hash(cpp_file: &Path) -> String {
    let mut hasher = SipHasher::new();
    cpp_file.hash(&mut hasher);
    hasher.finish().to_string()
}
