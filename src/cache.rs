use std::fs::{File, OpenOptions};
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::hash::{Hash, SipHasher, Hasher};
use ct_result::CtResult;
use dirs::cmd_cache_dir;

pub fn find_command_str(cpp_file: &Path) -> CtResult<Option<String>> {
    let cache_dir = try!(cmd_cache_dir());
    let cache_file = cache_dir.join(compute_hash(cpp_file));
    if ! cache_file.is_file() {
        return Ok(None);
    }

    let mut file = try!(File::open(cache_file));
    let mut cmd_str = String::new();
    try!(file.read_to_string(&mut cmd_str));
    return Ok(Some(cmd_str));
}

pub fn write_command_str(cpp_file: &Path, cmd_str: &str) -> CtResult<()> {
    let cache_dir = try!(cmd_cache_dir());
    let cache_file = cache_dir.join(compute_hash(cpp_file));
    if cache_file.is_file() {
        return Ok(());
    }

    let mut file = try!(OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(cache_file));

    let _ = try!(file.write_fmt(format_args!("{}", cmd_str)));
    Ok(())
}

fn compute_hash(cpp_file: &Path) -> String {
    let mut hasher = SipHasher::new();
    cpp_file.hash(&mut hasher);
    hasher.finish().to_string()
}
