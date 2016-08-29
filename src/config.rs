use std::path::{Path, PathBuf};
use std::fs;
use clap::{App, Arg};
use ct_result::{CtResult, CtError, OkOr};

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

       let mut cpp_file = PathBuf::from(try!(matches.value_of("SOURCE-FILE").ok_or("Missing C++ source file!")));
       try!(cpp_file.is_absolute().ok_or(format!("C++ source file '{}' has to have an absolute path!", cpp_file.display())));

       // if 'cpp_file' is a header file search for a C++ source file in the same directory
       {
           let header_file = cpp_file.clone();
           let is_header_file = if let Some(ext) = header_file.extension() {
               ext == "h" || ext == "hpp"
           } else {
               // no extension, assume a header file
               true
           };

           if is_header_file {
               let mut found_cpp_file: Option<PathBuf> = None;
               let cpp_exts = ["cpp", "cxx", "c"];
               for cpp_ext in &cpp_exts {
                   let file = header_file.with_extension(cpp_ext);
                   if file.is_file() {
                       found_cpp_file = Some(file);
                       break;
                   }
               }

               if found_cpp_file.is_none() {
                   return Err(CtError::from(format!("Couldn't find C++ source file for header '{}'!", header_file.display())));
               } else {
                   cpp_file = found_cpp_file.unwrap();
               }
           }
       }

       let db_files: Vec<PathBuf> = {
           if let Some(values) = matches.values_of("CLANG-DB") {
               values.map(PathBuf::from).collect()
           } else {
               let dir = try!(cpp_file.parent()
                  .ok_or(format!("Couldn't get directory of source file '{}'!", cpp_file.display())));

               vec![try!(find_db(&dir))]
           }
       };

       try!((! db_files.is_empty()).ok_or("Missing clang compilation database!"));

       Ok(Config {
           compiler: matches.value_of("compiler").map(String::from),
           cpp_file: cpp_file,
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
