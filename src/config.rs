use std::path::PathBuf;
use clap::{App, Arg};
use ct_result::{CtResult, CtError};

/// the configuration used to run `cpp-typecheck`
#[derive(Debug)]
pub struct Config {
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
           .arg(Arg::with_name("SOURCE-FILE")
               .help("The C++ source file to type check")
               .required(true)
               .index(1))
           .arg(Arg::with_name("CLANG-DB")
               .help("The clang compilation database")
               .required(true)
               .index(2)
               .multiple(true))
           .get_matches_safe());

       let cpp_file = PathBuf::from(try!(matches.value_of("SOURCE-FILE")
           .ok_or(CtError::from("Missing C++ source file!"))));

        if cpp_file.is_relative() {
            return Err(CtError::from("C++ source file has to have an absolute path!"));
        }

       let db_files: Vec<PathBuf> = try!(matches.values_of("CLANG-DB")
           .ok_or(CtError::from("Missing clang compilation database!")))
           .map(PathBuf::from)
           .collect();

       Ok(Config { cpp_file: cpp_file, db_files: db_files })
   }
}
