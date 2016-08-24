use std::path::{Path, PathBuf};
use std::fs;
use clap::{App, Arg};
use ct_result::{CtResult, OrErr};

/// the configuration used to run `cpp-typecheck`
#[derive(Debug)]
pub struct Config {
    /// use this compiler for the type checking instead
    /// of the one specified in the database
    pub compiler: Option<String>,

    /// the C++ source file to type check
    pub cpp_file: PathBuf,

    /// the clang compilation databases to look up
    /// the C++ source file
    pub db_files: Vec<PathBuf>
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
           .arg(Arg::with_name("SOURCE-FILE")
               .help("The C++ source file to type check")
               .required(true)
               .index(1))
           .arg(Arg::with_name("CLANG-DB")
               .help("The clang compilation database")
               .index(2)
               .multiple(true))
           .get_matches_safe());

       let cpp_file = PathBuf::from(try!(matches.value_of("SOURCE-FILE").or_err("Missing C++ source file!")));
       try!(cpp_file.is_absolute().or_err(format!("C++ source file '{}' has to have an absolute path!", cpp_file.display())));

       let db_files: Vec<PathBuf> = {
           if let Some(values) = matches.values_of("CLANG-DB") {
               values.map(PathBuf::from).collect()
           } else {
               let dir = try!(cpp_file.parent()
                  .or_err(format!("Couldn't get directory of source file '{}'!", cpp_file.display())));

               vec![try!(find_db(&dir))]
           }
       };

       try!((! db_files.is_empty()).or_err("Missing clang compilation database!"));

       Ok(Config {
           compiler: matches.value_of("compiler").map(String::from),
           cpp_file: cpp_file,
           db_files: db_files
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

        try!(dir.pop().or_err(format!("Couldn't find 'compile_commands.json' starting at directory '{}'!",
                                      start_dir.display())));
    }
}
