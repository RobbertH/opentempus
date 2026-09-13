//! Time zone resolution. Calendars in the wild use IANA names, Windows names
//! (Outlook) and a handful of odd aliases; this maps them all onto chrono-tz.

use chrono_tz::Tz;

/// Resolve a TZID string to a chrono-tz zone.
pub fn resolve(tzid: &str) -> Option<Tz> {
    let cleaned = tzid.trim().trim_matches('"');
    if let Ok(tz) = cleaned.parse::<Tz>() {
        return Some(tz);
    }
    // Some producers prefix with a path such as "/mozilla.org/20050126_1/Europe/Amsterdam"
    if let Some(idx) = cleaned.rfind('/') {
        let tail_start = cleaned[..idx].rfind('/').map(|i| i + 1).unwrap_or(0);
        if let Ok(tz) = cleaned[tail_start..].parse::<Tz>() {
            return Some(tz);
        }
    }
    windows_to_iana(cleaned).and_then(|name| name.parse::<Tz>().ok())
}

/// A pragmatic subset of the CLDR windowsZones mapping.
fn windows_to_iana(name: &str) -> Option<&'static str> {
    Some(match name {
        "UTC" | "Coordinated Universal Time" | "GMT" => "UTC",
        "GMT Standard Time" | "Greenwich Standard Time" => "Europe/London",
        "W. Europe Standard Time" => "Europe/Berlin",
        "Romance Standard Time" => "Europe/Paris",
        "Central Europe Standard Time" => "Europe/Budapest",
        "Central European Standard Time" => "Europe/Warsaw",
        "E. Europe Standard Time" => "Europe/Chisinau",
        "FLE Standard Time" => "Europe/Kiev",
        "GTB Standard Time" => "Europe/Bucharest",
        "Russian Standard Time" => "Europe/Moscow",
        "Turkey Standard Time" => "Europe/Istanbul",
        "Israel Standard Time" => "Asia/Jerusalem",
        "Arabian Standard Time" => "Asia/Dubai",
        "India Standard Time" => "Asia/Kolkata",
        "SE Asia Standard Time" => "Asia/Bangkok",
        "Singapore Standard Time" => "Asia/Singapore",
        "China Standard Time" => "Asia/Shanghai",
        "Tokyo Standard Time" => "Asia/Tokyo",
        "Korea Standard Time" => "Asia/Seoul",
        "AUS Eastern Standard Time" => "Australia/Sydney",
        "E. Australia Standard Time" => "Australia/Brisbane",
        "AUS Central Standard Time" => "Australia/Darwin",
        "W. Australia Standard Time" => "Australia/Perth",
        "New Zealand Standard Time" => "Pacific/Auckland",
        "South Africa Standard Time" => "Africa/Johannesburg",
        "Egypt Standard Time" => "Africa/Cairo",
        "Eastern Standard Time" => "America/New_York",
        "Central Standard Time" => "America/Chicago",
        "Mountain Standard Time" => "America/Denver",
        "US Mountain Standard Time" => "America/Phoenix",
        "Pacific Standard Time" => "America/Los_Angeles",
        "Alaskan Standard Time" => "America/Anchorage",
        "Hawaiian Standard Time" => "Pacific/Honolulu",
        "Atlantic Standard Time" => "America/Halifax",
        "Newfoundland Standard Time" => "America/St_Johns",
        "SA Pacific Standard Time" => "America/Bogota",
        "Argentina Standard Time" => "America/Buenos_Aires",
        "E. South America Standard Time" => "America/Sao_Paulo",
        "Central America Standard Time" => "America/Guatemala",
        "Mexico Standard Time" | "Central Standard Time (Mexico)" => "America/Mexico_City",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_iana_windows_and_prefixed_names() {
        assert_eq!(resolve("Europe/Amsterdam"), Some(chrono_tz::Europe::Amsterdam));
        assert_eq!(resolve("W. Europe Standard Time"), Some(chrono_tz::Europe::Berlin));
        assert_eq!(resolve("/mozilla.org/20050126_1/Europe/Amsterdam"), Some(chrono_tz::Europe::Amsterdam));
        assert_eq!(resolve("Nowhere/Land"), None);
    }
}
