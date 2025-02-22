use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::process::Command;
use serde_json::{self, Value, Map};
use atomicwrites::{AtomicFile, AllowOverwrite};
use ct_result::{CtResult, OkOr};
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
        let cache_dir = cmd_cache_dir()?;
        let cache_file = cache_dir.join(compute_hash(cpp_file));
        if !cache_file.is_file() {
            return Ok(None);
        }

        let mut file = File::open(cache_file)?;
        let mut cmd_str = String::new();
        file.read_to_string(&mut cmd_str)?;
        let mut lines = cmd_str.lines();

        let dir = PathBuf::from(lines.next()
            .ok_or(format!("Expected directory in first line of string:\n{}", cmd_str))?);

        let cmd = String::from(lines.next()
            .ok_or(format!("Expected command in second line of string:\n{}", cmd_str))?);

        (!cmd.is_empty()).ok_or(format!("Unexpected empty command in: {}!", cmd_str))?;

        let file = PathBuf::from(lines.next()
            .ok_or(format!("Expected file in third line of string:\n{}", cmd_str))?);

        Ok(Some(Cmd { directory: dir, command: cmd, file: file }))
    }

    pub fn from_databases(cpp_file: &Path, db_files: &[PathBuf]) -> CtResult<Cmd> {
        let mut file_buffer = String::new();
        for db_file in db_files {
            let mut file = File::open(db_file)?;
            file_buffer.clear();
            file.read_to_string(&mut file_buffer)?;

            let json_value: Value = serde_json::from_str(&file_buffer)?;
            let objs = json_value.as_array().ok_or(format!("Expected a json array but got: '{}'", json_value))?;

            for obj in objs {
                let obj = obj.as_object().ok_or(format!("Expected a json object but got: '{}'", obj))?;
                let cmd = Cmd::from_json_obj(obj)?;
                if cmd.has_cpp_file(cpp_file) {
                    return Ok(cmd);
                }
            }
        }

        Err(format!("Couldn't find C++ source file '{}' in compilation databases {:?}!",
                    cpp_file.display(), db_files).into())
    }

    fn from_json_obj(obj: &Map<String, Value>) -> CtResult<Cmd> {
        let dir = PathBuf::from(obj.get("directory").and_then(Value::as_str)
            .ok_or(format!("Couldn't find string entry 'directory' in json object: '{:?}'", obj))?);

        let file = {
            let f = PathBuf::from(obj.get("file").and_then(Value::as_str)
                .ok_or(format!("Couldn't find string entry 'file' in json object: '{:?}'", obj))?);

            if f.is_relative() {
                dir.join(f)
            } else {
                f
            }
        };

        let cmd = String::from(obj.get("command").and_then(Value::as_str)
            .ok_or(format!("Couldn't find string entry 'command' in json object: '{:?}'", obj))?)
            .replace("\\", "");

        Ok(Cmd { directory: dir, command: cmd, file: file })
    }

    pub fn write_to_cache(&self) -> CtResult<()> {
        let cache_dir = cmd_cache_dir()?;
        let cache_file = cache_dir.join(compute_hash(&self.file));

        AtomicFile::new(cache_file, AllowOverwrite).write(|f| {
            f.write_fmt(format_args!("{}\n{}\n{}",
                        self.directory.to_string_lossy(),
                        self.command,
                        self.file.to_string_lossy()))
        })?;

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

    pub fn typecheck(&self, compiler: &Option<String>) -> CtResult<()> {
        let (mut command, used_compiler) = self.get_command(compiler)?;
        if let Some(flag) = get_typecheck_flag(&used_compiler) {
            command.arg(flag);
        }
        command.status().map_err(|e| format!("Command execution failed: {}, because: {}", self.command, e))?;
        Ok(())
    }

    pub fn preprocess(&self, compiler: &Option<String>) -> CtResult<()> {
        let (mut command, used_compiler) = self.get_command(compiler)?;
        if let Some(flag) = get_preprocess_flag(&used_compiler) {
            command.arg(flag);
            command.status().map_err(|e| format!("Command execution failed: {}, because: {}", self.command, e))?;
            Ok(())
        } else {
            Err(format!("Unsupported compiler {} for preprocessing", used_compiler).into())
        }
    }

    fn get_command(&self, compiler: &Option<String>) -> CtResult<(Command, String)> {
        let mut args = self.command.split(" ");
        let db_compiler = args.next().ok_or("Unexpected empty arguments after command string split!")?;
        let used_compiler = if let Some(ref c) = compiler {
            c.clone()
        } else {
            db_compiler.to_string()
        };

        let mut command = Command::new(&used_compiler);
        command.current_dir(&self.directory);

        while let Some(arg) = args.next() {
            if arg == "-o" {
                // remove the file argument
                args.next();
            } else if !arg.is_empty() {
                command.arg(arg);
            }
        }

        Ok((command, used_compiler))
    }
}

fn get_typecheck_flag(compiler: &str) -> Option<&str> {
    if is_gcc_or_clang(compiler) {
        Some("-fsyntax-only")
    } else {
        None
    }
}

fn get_preprocess_flag(compiler: &str) -> Option<&str> {
    if is_gcc_or_clang(compiler) {
        Some("-E")
    } else {
        None
    }
}

fn is_gcc_or_clang(compiler: &str) -> bool {
    compiler.contains("gcc")
        || compiler.contains("g++")
        || compiler.contains("clang")
        || compiler.contains("clang++")
}

fn compute_hash(cpp_file: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    cpp_file.hash(&mut hasher);
    hasher.finish().to_string()
}
