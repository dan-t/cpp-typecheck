use std::borrow::Cow;
use std::error::Error as StdError;
use std::io::Error as IoError;
use serde_json::Error as SerdeError;

/// the result type used for the whole application
pub type CtResult<T> = Result<T, CtError>;

/// the error type used for the whole application
error_type! {
    #[derive(Debug)]
    pub enum CtError {
        Io(IoError) {
            cause;
        },

        Serde(SerdeError) {
            cause;
        },

        Msg(Cow<'static, str>) {
            desc (e) &**e;
            from (s: &'static str) s.into();
            from (s: String) s.into();
        },

        Other(Box<StdError>) {
            desc (e) e.description();
            cause (e) Some(&**e);
        }
    }
}
