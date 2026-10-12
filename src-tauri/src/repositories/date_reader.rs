use chrono::{DateTime, Utc};

use crate::models::backup::RemoteObject;

/// Reads and writes the date formats of the backup servers. Dates are milliseconds since 1970.
pub struct DateReader;

impl DateReader {
    /// "2009-10-09T21:04:30.000Z"
    pub fn iso8601(text: &str) -> i64 {
        DateTime::parse_from_rfc3339(text).map_or(RemoteObject::UNKNOWN_DATE, |date| date.timestamp_millis())
    }

    /// "Fri, 09 Oct 2009 21:04:30 GMT"
    pub fn rfc1123(text: &str) -> i64 {
        DateTime::parse_from_rfc2822(text).map_or(RemoteObject::UNKNOWN_DATE, |date| date.timestamp_millis())
    }

    /// The time now, the way AWS wants it in a signed request: "20130524T000000Z".
    pub fn amz_now() -> String {
        Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
    }
}
