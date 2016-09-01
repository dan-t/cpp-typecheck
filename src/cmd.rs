use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::hash::{Hash, SipHasher, Hasher};
use std::process::Command;
use serde_json::{self, Value, Map};
use atomicwrites::{AtomicFile, AllowOverwrite};
use ct_result::{CtResult, CtError, OkOr};
use dirs::cmd_cache_dir;

/// a compiler command from the clang compilation database
#[derive(Clone, Debug)]
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
            .ok_or(format!("Expected directory in first line of string:\n{}", cmd_str))));

        let cmd = String::from(try!(lines.next()
            .ok_or(format!("Expected command in second line of string:\n{}", cmd_str))));

        try!((! cmd.is_empty()).ok_or(format!("Unexpected empty command in: {}!", cmd_str)));

        let file = PathBuf::from(try!(lines.next()
            .ok_or(format!("Expected file in third line of string:\n{}", cmd_str))));

        Ok(Some(Cmd { directory: dir, command: cmd, file: file }))
    }

    pub fn from_databases(cpp_file: &Path, db_files: &[PathBuf]) -> CtResult<Cmd> {
        let mut file_buffer = String::new();
        for db_file in db_files {
            let mut file = try!(File::open(db_file));
            file_buffer.clear();
            try!(file.read_to_string(&mut file_buffer));

            let json_value: Value = try!(serde_json::from_str(&file_buffer));
            let objs = try!(json_value.as_array().ok_or(format!("Expected a json array but got: '{}'", json_value)));

            for obj in objs {
                let obj = try!(obj.as_object().ok_or(format!("Expected a json object but got: '{}'", obj)));
                let cmd = try!(Cmd::from_json_obj(obj));
                if cmd.has_cpp_file(cpp_file) {
                    return Ok(cmd);
                }
            }
        }

        Err(format!("Couldn't find C++ source file '{}' in compilation databases {:?}!",
                    cpp_file.display(), db_files).into())
    }

    fn from_json_obj(obj: &Map<String, Value>) -> CtResult<Cmd> {
        let dir = PathBuf::from(try!(obj.get("directory").and_then(Value::as_str)
            .ok_or(format!("Couldn't find string entry 'directory' in json object: '{:?}'", obj))));

        let file = {
            let f = PathBuf::from(try!(obj.get("file").and_then(Value::as_str)
                .ok_or(format!("Couldn't find string entry 'file' in json object: '{:?}'", obj))));

            if f.is_relative() {
                dir.join(f)
            } else {
                f
            }
        };

        let cmd = String::from(try!(obj.get("command").and_then(Value::as_str)
            .ok_or(format!("Couldn't find string entry 'command' in json object: '{:?}'", obj))))
            .replace("\\", "");

        Ok(Cmd { directory: dir, command: cmd, file: file })
    }

    pub fn write_to_cache(&self) -> CtResult<()> {
        let cache_dir = try!(cmd_cache_dir());
        let cache_file = cache_dir.join(compute_hash(&self.file));

        let file = AtomicFile::new(cache_file, AllowOverwrite);
        try!(file.write(|f| {
            f.write_fmt(format_args!("{}\n{}\n{}",
                                     self.directory.to_string_lossy(),
                                     self.command,
                                     self.file.to_string_lossy()))
        }));

        Ok(())
    }

    pub fn has_cpp_file(&self, file: &Path) -> bool {
        file == self.file
    }

    pub fn replace_cpp_file(&self, cpp_file: &Path) -> Cmd {
        Cmd {
            directory: self.directory.clone(),
            command: self.command.clone().replace(&format!("{}", self.file.display()),
                                                  &format!("{}", cpp_file.display())),
            file: cpp_file.to_path_buf()
        }
    }

    pub fn exec(&self) -> CtResult<()> {
        self.exec_internal(None)
    }

    pub fn exec_with(&self, compiler: &str) -> CtResult<()> {
        self.exec_internal(Some(compiler))
    }

    pub fn get_compiler(&self) -> CtResult<&str> {
        let mut parts = self.command.split(" ");
        parts.next().ok_or(CtError::from(format!("Unexpected empty parts after command split of: {}!", self.command)))
    }

    fn exec_internal(&self, compiler: Option<&str>) -> CtResult<()> {
        let mut parts = self.command.split(" ");

        let db_compiler = try!(parts.next().ok_or("Unexpected empty parts after command string split!"));
        let used_compiler = compiler.unwrap_or(db_compiler);

        let mut cmd = Command::new(&used_compiler);
        cmd.current_dir(&self.directory);

        for p in parts {
            if p.is_empty() {
                continue;
            }

            cmd.arg(p);
        }

        if is_gcc_or_clang_compiler(used_compiler) {
            cmd.arg("-fsyntax-only");
        }

        try!(cmd.status()
            .map_err(|e| CtError::from(format!("Command execution failed: {}, because: {}", self.command, e))));

        Ok(())
    }
}

pub fn has_only_type_checking_flag(compiler: &str) -> bool {
    is_gcc_or_clang_compiler(compiler)
}

fn is_gcc_or_clang_compiler(compiler: &str) -> bool {
    compiler.contains("gcc")
        || compiler.contains("g++")
        || compiler.contains("clang")
        || compiler.contains("clang++")
}

fn compute_hash(cpp_file: &Path) -> String {
    let mut hasher = SipHasher::new();
    cpp_file.hash(&mut hasher);
    hasher.finish().to_string()
}
