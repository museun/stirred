pub trait FormatTime {
    fn as_readable_time(&self) -> String;
}

impl FormatTime for time::Duration {
    fn as_readable_time(&self) -> String {
        format_seconds(self.as_seconds_f64() as _)
    }
}

impl FormatTime for std::time::Duration {
    fn as_readable_time(&self) -> String {
        format_seconds(self.as_secs())
    }
}

fn format_seconds(mut secs: u64) -> String {
    const TABLE: [(&str, u64); 4] = [
        ("days", 86400),
        ("hours", 3600),
        ("minutes", 60),
        ("seconds", 1),
    ];

    fn pluralize(s: &str, n: u64) -> String {
        format!("{} {}", n, if n > 1 { s } else { &s[..s.len() - 1] })
    }

    let mut time = vec![];
    for (name, d) in &TABLE {
        let div = secs / d;
        if div > 0 {
            time.push(pluralize(name, div));
            secs -= d * div;
        }
    }

    let len = time.len();
    if len > 1 {
        if len > 2 {
            for segment in time.iter_mut().take(len - 2) {
                segment.push(',')
            }
        }
        time.insert(len - 1, "and".into())
    }
    time.join(" ")
}
