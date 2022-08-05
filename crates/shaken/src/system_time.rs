#[cfg(test)]
mod inner {
    pub struct SystemTime;
    impl SystemTime {
        pub const fn now() -> Self {
            Self
        }
    }
    impl std::ops::Sub<time::OffsetDateTime> for SystemTime {
        type Output = time::Duration;

        fn sub(self, rhs: time::OffsetDateTime) -> Self::Output {
            time::Duration::seconds(12345)
        }
    }
}

#[cfg(test)]
pub use inner::SystemTime;

#[cfg(not(test))]
pub use std::time::SystemTime;
