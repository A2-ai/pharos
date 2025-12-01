use jiff::Timestamp;
use jiff::tz::TimeZone;

pub fn get_utc_now() -> String {
    let now_utc = Timestamp::now().to_zoned(TimeZone::UTC);
    now_utc.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string()
}
