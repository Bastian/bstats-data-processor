use chrono::prelude::*;
use once_cell::sync::Lazy;

// tms2000 were supposed to be the number of 30 minute intervals since
// 2000-01-01 00:00:00 (tms2000 = *t*hirty *m*minutes *s*ince the year *2000*).
// However, the original implementation was written in JavaScript using
// Date.UTC(2000, 1, 1, 0, 0, 0, 0) as the starting point. This is incorrect, as
// the month is 0-indexed in JavaScript, so it should have been
// Date.UTC(2000, 0, 1, 0, 0, 0, 0);. This means that the tms2000 values are off
// by 1 month. And since the tms2000 has been used in the database, we can't
// easily correct it. So we need to keep the incorrect starting point.
// So, maybe we should call it tmsFeb2000 instead... 🙈🙈🙈
static PAST: Lazy<DateTime<Utc>> = Lazy::new(|| Utc.with_ymd_and_hms(2000, 2, 1, 0, 0, 0).unwrap());

/// Converts a DateTime<Utc> to a tms2000 timestamp.
pub fn date_to_tms2000(date: DateTime<Utc>) -> i64 {
    let duration = date.signed_duration_since(*PAST);
    duration.num_minutes() / 30
}

/// Converts a tms2000 timestamp to a DateTime<Utc>.
pub fn tms2000_to_date(tms2000: i64) -> DateTime<Utc> {
    *PAST + chrono::Duration::minutes(tms2000 * 30)
}

/// Converts a tms2000 timestamp to a UNIX timestamp.
pub fn tms2000_to_timestamp(tms2000: i64) -> i64 {
    tms2000_to_date(tms2000).timestamp()
}

/// Converts a DateTime<Utc> to a tms2000 timestamp divided by 1000 (rounded down).
pub fn date_to_tms2000_div1000(date: DateTime<Utc>) -> i64 {
    let duration = date.signed_duration_since(*PAST);
    duration.num_seconds() / (1000 * 60 * 30)
}

/// Converts a tms2000 timestamp divided by 1000 (rounded down) to a DateTime<Utc>.
pub fn tms2000div1000_to_date(tms2000div1000: i64) -> DateTime<Utc> {
    *PAST + chrono::Duration::seconds(tms2000div1000 * 1000 * 60 * 30)
}

/// Returns the number of 30 minute intervals since the last tms2000div1000 timestamp at the given date.
pub fn thirty_minutes_since_last_tms2000div1000(date: DateTime<Utc>) -> i64 {
    let tms2000 = date_to_tms2000(date);
    let tms2000div1000 = date_to_tms2000_div1000(date);
    let date_last_tms2000div1000 = tms2000div1000_to_date(tms2000div1000);
    let tms2000_at_last_tms2000div1000 = date_to_tms2000(date_last_tms2000div1000);
    tms2000 - tms2000_at_last_tms2000div1000
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_date_to_tms2000() {
        assert_eq!(
            date_to_tms2000(Utc.with_ymd_and_hms(2000, 2, 1, 0, 0, 0).unwrap()),
            0
        );
        assert_eq!(
            date_to_tms2000(Utc.with_ymd_and_hms(2024, 7, 25, 19, 0, 3).unwrap()),
            429206
        );
    }

    #[test]
    fn test_tms2000_to_date() {
        assert_eq!(
            tms2000_to_date(0),
            Utc.with_ymd_and_hms(2000, 2, 1, 0, 0, 0).unwrap()
        );
        assert_eq!(
            tms2000_to_date(429206),
            Utc.with_ymd_and_hms(2024, 7, 25, 19, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_tms2000_to_timestamp() {
        assert_eq!(tms2000_to_timestamp(0), 949363200);
        assert_eq!(tms2000_to_timestamp(429206), 1721934000);
    }

    #[test]
    fn test_date_to_tms2000_div1000() {
        assert_eq!(
            date_to_tms2000_div1000(Utc.with_ymd_and_hms(2000, 2, 1, 0, 0, 0).unwrap()),
            0
        );
        assert_eq!(
            date_to_tms2000_div1000(Utc.with_ymd_and_hms(2024, 7, 25, 19, 0, 3).unwrap()),
            429
        );
    }

    #[test]
    fn test_tms2000div1000_to_date() {
        assert_eq!(
            tms2000div1000_to_date(0),
            Utc.with_ymd_and_hms(2000, 2, 1, 0, 0, 0).unwrap()
        );
        assert_eq!(
            tms2000div1000_to_date(429),
            Utc.with_ymd_and_hms(2024, 7, 21, 12, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_thirty_minutes_since_last_tms2000div1000() {
        assert_eq!(
            thirty_minutes_since_last_tms2000div1000(
                Utc.with_ymd_and_hms(2000, 2, 1, 0, 0, 0).unwrap()
            ),
            0
        );
        assert_eq!(
            thirty_minutes_since_last_tms2000div1000(
                Utc.with_ymd_and_hms(2000, 2, 1, 0, 30, 0).unwrap()
            ),
            1
        );
        assert_eq!(
            thirty_minutes_since_last_tms2000div1000(
                Utc.with_ymd_and_hms(2000, 2, 1, 1, 0, 0).unwrap()
            ),
            2
        );
        assert_eq!(
            thirty_minutes_since_last_tms2000div1000(
                Utc.with_ymd_and_hms(2000, 2, 1, 0, 0, 0).unwrap()
            ),
            0
        );
        assert_eq!(
            thirty_minutes_since_last_tms2000div1000(
                Utc.with_ymd_and_hms(2024, 7, 21, 12, 0, 0).unwrap()
            ),
            0
        );
        assert_eq!(
            thirty_minutes_since_last_tms2000div1000(
                Utc.with_ymd_and_hms(2024, 7, 22, 12, 0, 0).unwrap()
            ),
            48
        );
    }
}
