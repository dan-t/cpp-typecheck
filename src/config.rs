use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use clap::{App, Arg};
use tempfile::{NamedTempFile, NamedTempFileOptions};
use ct_result::{CtResult, CtError, OkOr};
use cmd::Cmd;

#[derive(Debug)]
pub enum SourceFile {
    /// the C++ source file given as argument
    /// to `cpp-typecheck`
    FromArg {
        cpp_file: PathBuf
    },

    /// a header file was given as argument and a
    /// source file in the same directory was found
    FromHeader {
        header_file: PathBuf,
        cpp_file: PathBuf
    },

    /// a header file was given as argument and no
    /// source file could be found, so a temporary
    /// one was created that just includes the header,
    /// a compiler command was created by taking the
    /// arguments of an other source file in the same
    /// directory
    FromHeaderWithTmpSource {
        header_file: PathBuf,
        cpp_file: NamedTempFile,
        command: Cmd
    }
}

impl SourceFile {
    pub fn origin_path(&self) -> &Path {
        match *self {
            SourceFile::FromArg { ref cpp_file } => cpp_file.as_path(),
            SourceFile::FromHeader { ref header_file, .. } => header_file.as_path(),
            SourceFile::FromHeaderWithTmpSource { ref header_file, .. } => header_file.as_path()
        }
    }
}

/// the configuration used to run `cpp-typecheck`
#[derive(Debug)]
pub struct Config {
    /// use this compiler for the type checking instead
    /// of the one specified in the database
    pub compiler: Option<String>,

    /// the C++ source file to type check
    pub source_file: SourceFile,

    /// the clang compilation databases to look up
    /// the C++ source file
    pub db_files: Vec<PathBuf>,

    /// forces the lookup in the database without considering
    /// the command cache
    pub no_cache: bool,

    /// forces the recaching of the command by doing the lookup
    /// in the database
    pub force_recache: bool
}

impl Config {
   pub fn from_command_args() -> CtResult<Config> {
       let matches = try!(App::new("cpp-typecheck")
           .about("Type check a C++ source file with a clang compilation database")
           .version(crate_version!())
           .author("Daniel Trstenjak <daniel.trstenjak@gmail.com>")
           .arg(Arg::with_name("compiler")
                .short("c")
                .long("compiler")
                .value_names(&["PATH"])
                .help("Use this compiler for the type checking instead of the one specified in the database")
                .takes_value(true))
           .arg(Arg::with_name("no-cache")
                .short("n")
                .long("no-cache")
                .help("Forces the lookup in the database without considering the command cache"))
           .arg(Arg::with_name("force-recache")
                .short("f")
                .long("force-recache")
                .help("Forces the recaching of the command by doing the lookup in the database"))
           .arg(Arg::with_name("SOURCE-FILE")
               .help("The C++ source file to type check")
               .required(true)
               .index(1))
           .arg(Arg::with_name("CLANG-DB")
               .help("The clang compilation database")
               .index(2)
               .multiple(true))
           .get_matches_safe());

       let src_file = PathBuf::from(try!(matches.value_of("SOURCE-FILE").ok_or("Missing C++ source file!")));
       try!(src_file.is_absolute().ok_or(format!("C++ source file '{}' has to have an absolute path!",
                                         src_file.display())));

       let db_files: Vec<PathBuf> = {
           if let Some(values) = matches.values_of("CLANG-DB") {
               values.map(PathBuf::from).collect()
           } else {
               let dir = try!(src_file.parent()
                  .ok_or(format!("Couldn't get directory of source file '{}'!", src_file.display())));

               vec![try!(find_db(&dir))]
           }
       };

       try!((! db_files.is_empty()).ok_or("Missing clang compilation database!"));

       let source_file = try!(get_source_file(&src_file, &db_files));

       Ok(Config {
           compiler: matches.value_of("compiler").map(String::from),
           source_file: source_file,
           db_files: db_files,
           no_cache: matches.is_present("no-cache"),
           force_recache: matches.is_present("force-recache")
       })
   }
}

/// Searches for a `compile_commands.json` file starting at `start_dir` and continuing the search
/// upwards the directory tree until the file is found.
fn find_db(start_dir: &Path) -> CtResult<PathBuf> {
    let mut dir = start_dir.to_path_buf();
    loop {
        if let Ok(files) = fs::read_dir(&dir) {
            for file in files {
                if let Ok(file) = file {
                    let path = file.path();
                    if path.is_file() {
                        if let Some("compile_commands.json") = path.file_name().and_then(|s| s.to_str()) {
                            return Ok(path);
                        }
                    }
                }
            }
        }

        try!(dir.pop().ok_or(format!("Couldn't find 'compile_commands.json' starting at directory '{}'!",
                                     start_dir.display())));
    }
}

fn get_source_file(src_file: &Path, db_files: &[PathBuf]) -> CtResult<SourceFile> {
    let is_header_file = if let Some(ext) = src_file.extension() {
        ext == "h"
            || ext == "hpp"
            || ext == "hh"
            || ext == "H"
            || ext == "HPP"
            || ext == "HH"
    } else {
        // no extension, assume a header file
        true
    };

    if ! is_header_file {
        return Ok(SourceFile::FromArg { cpp_file: src_file.to_path_buf() });
    }

    let cpp_exts = ["cpp", "cxx", "cc", "c", "CPP", "CXX", "CC", "C"];
    // search for C++ source file in the same directory with same
    // name as the header file
    {
        let mut cpp_file: Option<PathBuf> = None;
        for cpp_ext in &cpp_exts {
            let file = src_file.with_extension(cpp_ext);
            if file.is_file() {
                cpp_file = Some(file);
                break;
            }
        }

        if let Some(cpp_file) = cpp_file {
            return Ok(SourceFile::FromHeader {
                header_file: src_file.to_path_buf(),
                cpp_file: cpp_file
            });
        }
    }

    // search for any other C++ source file in the same directory
    // as the header file, take its compiler command and replace the
    // source file with a temporary created source file that only
    // includes the header file
    {
        let src_dir = try!(src_file.parent()
            .ok_or(format!("Couldn't get directory of source file '{}'!", src_file.display())));

        for file in try!(src_dir.read_dir()) {
            if let Ok(file) = file {
                let file = file.path();
                if ! file.is_file() {
                    continue;
                }

                if ! is_cpp_source_file(&file) {
                    continue;
                }

                if let Ok(cmd) = Cmd::from_databases(&file, db_files) {
                    let mut cpp_file = try!(NamedTempFileOptions::new()
                        .prefix("cpp-typecheck-")
                        .suffix(".cpp")
                        .create());

                    try!(cpp_file.write_fmt(format_args!("#include \"{}\"\n", src_file.display())));
                    let cmd = cmd.replace_cpp_file(&cpp_file.path());

                    return Ok(SourceFile::FromHeaderWithTmpSource {
                        header_file: src_file.to_path_buf(),
                        cpp_file: cpp_file,
                        command: cmd
                    });
                }
            }
        }
    }

    Err(CtError::from(format!("Couldn't find C++ source file for header '{}'!", src_file.display())))
}

fn is_cpp_source_file(file: &Path) -> bool {
    if let Some(ext) = file.extension() {
        let ext = ext.to_string_lossy();
        let cpp_exts = ["cpp", "cxx", "c"];
        for cpp_ext in &cpp_exts {
            if ext == *cpp_ext {
                return true;
            }
        }
    }

    return false;
}
