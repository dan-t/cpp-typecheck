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
