use std::io::Error as IoError;
use serde_json::Error as SerdeError;
use clap::Error as ClapError;

/// the result type used for the whole application
pub type CtResult<T> = Result<T, CtError>;

/// the error type used for the whole application
error_type! {
    #[derive(Debug, Clone)]
    pub enum CtError {
        Msg(String) {
            desc (e) &e;
            from (s: &'static str) s.into();
            from (ie: IoError) ie.to_string();
            from (se: SerdeError) se.to_string();
            from (ce: ClapError) ce.to_string();
        },
    }
}

macro_rules! unwrap_or_err {
    ($opt:expr, $err:expr) => (match $opt {
        Option::Some(val) => val,
        Option::None      => {
            return Result::Err(CtError::from($err))
        }
    })
}

macro_rules! true_or_err {
    ($bool:expr, $err:expr) => (if $bool {} else { return Result::Err(CtError::from($err)) })
}

macro_rules! false_or_err {
    ($bool:expr, $err:expr) => (if $bool { return Result::Err(CtError::from($err)) } else {})
}
