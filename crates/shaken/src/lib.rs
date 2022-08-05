// #![cfg_attr(debug_assertions, allow(dead_code, unused_variables,))]

#[macro_export]
macro_rules! check {
    ($cond:expr) => {
        if !$cond {
            return Response::nothing();
        }
    };
}

#[macro_export]
macro_rules! ensure {
    ($cond:expr, $f:expr, $($tt:tt),* $(,)?) => {
        if !$cond {
            return Response::empty().problem(format!($f, $($tt),*)).ok()
        }
    };

    ($cond:expr, $f:expr) => {
        if !$cond {
            return Response::empty().problem(format!($f)).ok()
        }
    };
}

mod persist;

pub mod config;
pub use config::Config;

pub mod error;
pub mod irc;
pub mod modules;
pub mod queue;
pub mod serde;
pub mod twitch;

mod binding;
pub use binding::{create, Binding};

mod callable;
pub use callable::{
    BoxedCallable, BoxedFuture, BoxedResponse, Callable, Request, Response, ResponseKind,
};

mod state;
pub use state::{SharedState, State};

mod format;
pub use format::FormatTime;

mod arguments;
pub use arguments::Arguments;

mod util;

mod http;

pub mod help;

mod system_time;
pub use system_time::SystemTime;

#[cfg(test)]
mod testing;

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> std::ops::Deref for Either<L, R>
where
    L: std::ops::Deref<Target = R>,
{
    type Target = R;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Left(val) => val,
            Self::Right(val) => val,
        }
    }
}
