use std::{borrow::Cow, str::FromStr};

use serde::{de::Error, Deserialize, Deserializer};
use time::OffsetDateTime;

pub fn assume_utc_date_time<'de, D>(deser: D) -> Result<OffsetDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    const FORMAT: &str = "[year]-[month]-[day]T\
        [hour]:[minute]:[second]Z\
        [offset_hour sign:mandatory][offset_minute]";

    let s = <Cow<'_, str>>::deserialize(deser)? + "+0000";
    let f = time::format_description::parse(FORMAT).map_err(Error::custom)?;
    OffsetDateTime::parse(&s, &f).map_err(Error::custom)
}

pub fn from_str<'de, D, T>(deser: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: std::fmt::Display,
    D: Deserializer<'de>,
{
    <Cow<'_, str>>::deserialize(deser)?
        .parse()
        .map_err(Error::custom)
}
