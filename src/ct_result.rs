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

pub trait OrErr {
    type R;

    fn or_err<E>(self, err: E) -> Result<Self::R, E>;
}

impl<T> OrErr for Option<T> {
    type R = T;

    fn or_err<E>(self, err: E) -> Result<Self::R, E> {
        match self {
            Some(v) => Ok(v),
            None    => Err(err)
        }
    }
}

impl OrErr for bool {
    type R = ();

    fn or_err<E>(self, err: E) -> Result<Self::R, E> {
        if self {
            Ok(())
        } else {
            Err(err)
        }
    }
}
