use alloc::{format, string::String, vec, vec::Vec};
use core::{
    clone::Clone,
    cmp::{Ordering, PartialEq},
    default::Default,
    fmt::Debug,
    marker::Copy,
    option::Option::{self, None, Some},
    primitive::{bool, i64, str, u8, u16, u64, usize},
};

use arbitrary_int::{traits::Integer, u1, u4, u5, u6, u7, u10, u13};

use crate::{Input, Language, bit, take};

// stamp (64 bits)
// note:
// - Date (year, month, day) all 0: no date, time only.
//   Time only 00:00 collides with Stamp::ZERO, so the day boundary is written as 24:00.
// - Zeros from the lowest field upward are unspecified;
//   the lowest non-zero field is the precision.
// - is_utc:
//   = 1: The value is UTC time (zone iana id may store original zone info).
//   = 0: The value is local time of zone

bit::field! {
    pub Centisecond(u7),
    pub Second(u6),
    pub Minute(u6),
    pub Hour(u5),
    pub Day(u5),
    pub Year(u13),
    // 1 = true (year~second is utc value) and 0 = false (iana local value)
    pub IsUtc(u1),
}

bit::composite! {
    pub struct Stamp {
        Centisecond,
        Second,
        Minute,
        Hour,
        Day,
        Month,
        Year,
        IsUtc,
        Zone,
    }
}

impl Input<'_> for Stamp {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        let stamp = Self::from_le_bytes(take(input, Self::BYTES)?.try_into().ok()?);
        let (centisecond, second, minute, hour, day, month, year, _, zone) = unpack(stamp);
        let clock = centisecond < 100
            && second < 60
            && minute < 60
            && (hour < 24 || (hour, minute, second, centisecond) == (24, 0, 0, 0));
        let date = month <= 12 && (month == 0 || day <= days_in_month(year, month));
        (clock && date && (zone as usize) < Zone::LIST.len()).then_some(stamp)
    }
}

bit::field! {
    // id of IANA Time Zone Database
    pub Zone(u10),
}

impl Zone {
    pub const LIST: [&str; 446] = [
        "Etc/UTC",
        "Etc/GMT",
        "Etc/GMT+12",
        "Etc/GMT+11",
        "Etc/GMT+10",
        "Etc/GMT+9",
        "Etc/GMT+8",
        "Etc/GMT+7",
        "Etc/GMT+6",
        "Etc/GMT+5",
        "Etc/GMT+4",
        "Etc/GMT+3",
        "Etc/GMT+2",
        "Etc/GMT+1",
        "Etc/GMT-1",
        "Etc/GMT-2",
        "Etc/GMT-3",
        "Etc/GMT-4",
        "Etc/GMT-5",
        "Etc/GMT-6",
        "Etc/GMT-7",
        "Etc/GMT-8",
        "Etc/GMT-9",
        "Etc/GMT-10",
        "Etc/GMT-11",
        "Etc/GMT-12",
        "Etc/GMT-13",
        "Etc/GMT-14",
        "Africa/Abidjan",
        "Africa/Accra",
        "Africa/Addis_Ababa",
        "Africa/Algiers",
        "Africa/Asmara",
        "Africa/Bamako",
        "Africa/Bangui",
        "Africa/Banjul",
        "Africa/Bissau",
        "Africa/Blantyre",
        "Africa/Brazzaville",
        "Africa/Bujumbura",
        "Africa/Cairo",
        "Africa/Casablanca",
        "Africa/Ceuta",
        "Africa/Conakry",
        "Africa/Dakar",
        "Africa/Dar_es_Salaam",
        "Africa/Djibouti",
        "Africa/Douala",
        "Africa/El_Aaiun",
        "Africa/Freetown",
        "Africa/Gaborone",
        "Africa/Harare",
        "Africa/Johannesburg",
        "Africa/Juba",
        "Africa/Kampala",
        "Africa/Khartoum",
        "Africa/Kigali",
        "Africa/Kinshasa",
        "Africa/Lagos",
        "Africa/Libreville",
        "Africa/Lome",
        "Africa/Luanda",
        "Africa/Lubumbashi",
        "Africa/Lusaka",
        "Africa/Malabo",
        "Africa/Maputo",
        "Africa/Maseru",
        "Africa/Mbabane",
        "Africa/Mogadishu",
        "Africa/Monrovia",
        "Africa/Nairobi",
        "Africa/Ndjamena",
        "Africa/Niamey",
        "Africa/Nouakchott",
        "Africa/Ouagadougou",
        "Africa/Porto-Novo",
        "Africa/Sao_Tome",
        "Africa/Tripoli",
        "Africa/Tunis",
        "Africa/Windhoek",
        "America/Adak",
        "America/Anchorage",
        "America/Anguilla",
        "America/Antigua",
        "America/Araguaina",
        "America/Argentina/Buenos_Aires",
        "America/Argentina/Catamarca",
        "America/Argentina/Cordoba",
        "America/Argentina/Jujuy",
        "America/Argentina/La_Rioja",
        "America/Argentina/Mendoza",
        "America/Argentina/Rio_Gallegos",
        "America/Argentina/Salta",
        "America/Argentina/San_Juan",
        "America/Argentina/San_Luis",
        "America/Argentina/Tucuman",
        "America/Argentina/Ushuaia",
        "America/Aruba",
        "America/Asuncion",
        "America/Atikokan",
        "America/Bahia",
        "America/Bahia_Banderas",
        "America/Barbados",
        "America/Belem",
        "America/Belize",
        "America/Blanc-Sablon",
        "America/Boa_Vista",
        "America/Bogota",
        "America/Boise",
        "America/Cambridge_Bay",
        "America/Campo_Grande",
        "America/Cancun",
        "America/Caracas",
        "America/Cayenne",
        "America/Cayman",
        "America/Chicago",
        "America/Chihuahua",
        "America/Ciudad_Juarez",
        "America/Costa_Rica",
        "America/Coyhaique",
        "America/Creston",
        "America/Cuiaba",
        "America/Curacao",
        "America/Danmarkshavn",
        "America/Dawson",
        "America/Dawson_Creek",
        "America/Denver",
        "America/Detroit",
        "America/Dominica",
        "America/Edmonton",
        "America/Eirunepe",
        "America/El_Salvador",
        "America/Fort_Nelson",
        "America/Fortaleza",
        "America/Glace_Bay",
        "America/Goose_Bay",
        "America/Grand_Turk",
        "America/Grenada",
        "America/Guadeloupe",
        "America/Guatemala",
        "America/Guayaquil",
        "America/Guyana",
        "America/Halifax",
        "America/Havana",
        "America/Hermosillo",
        "America/Indiana/Indianapolis",
        "America/Indiana/Knox",
        "America/Indiana/Marengo",
        "America/Indiana/Petersburg",
        "America/Indiana/Tell_City",
        "America/Indiana/Vevay",
        "America/Indiana/Vincennes",
        "America/Indiana/Winamac",
        "America/Inuvik",
        "America/Iqaluit",
        "America/Jamaica",
        "America/Juneau",
        "America/Kentucky/Louisville",
        "America/Kentucky/Monticello",
        "America/Kralendijk",
        "America/La_Paz",
        "America/Lima",
        "America/Los_Angeles",
        "America/Lower_Princes",
        "America/Maceio",
        "America/Managua",
        "America/Manaus",
        "America/Marigot",
        "America/Martinique",
        "America/Matamoros",
        "America/Mazatlan",
        "America/Menominee",
        "America/Merida",
        "America/Metlakatla",
        "America/Mexico_City",
        "America/Miquelon",
        "America/Moncton",
        "America/Monterrey",
        "America/Montevideo",
        "America/Montserrat",
        "America/Nassau",
        "America/New_York",
        "America/Nome",
        "America/Noronha",
        "America/North_Dakota/Beulah",
        "America/North_Dakota/Center",
        "America/North_Dakota/New_Salem",
        "America/Nuuk",
        "America/Ojinaga",
        "America/Panama",
        "America/Paramaribo",
        "America/Phoenix",
        "America/Port-au-Prince",
        "America/Port_of_Spain",
        "America/Porto_Velho",
        "America/Puerto_Rico",
        "America/Punta_Arenas",
        "America/Rankin_Inlet",
        "America/Recife",
        "America/Regina",
        "America/Resolute",
        "America/Rio_Branco",
        "America/Santarem",
        "America/Santiago",
        "America/Santo_Domingo",
        "America/Sao_Paulo",
        "America/Scoresbysund",
        "America/Sitka",
        "America/St_Barthelemy",
        "America/St_Johns",
        "America/St_Kitts",
        "America/St_Lucia",
        "America/St_Thomas",
        "America/St_Vincent",
        "America/Swift_Current",
        "America/Tegucigalpa",
        "America/Thule",
        "America/Tijuana",
        "America/Toronto",
        "America/Tortola",
        "America/Vancouver",
        "America/Whitehorse",
        "America/Winnipeg",
        "America/Yakutat",
        "Antarctica/Casey",
        "Antarctica/Davis",
        "Antarctica/DumontDUrville",
        "Antarctica/Macquarie",
        "Antarctica/Mawson",
        "Antarctica/McMurdo",
        "Antarctica/Palmer",
        "Antarctica/Rothera",
        "Antarctica/Syowa",
        "Antarctica/Troll",
        "Antarctica/Vostok",
        "Arctic/Longyearbyen",
        "Asia/Aden",
        "Asia/Almaty",
        "Asia/Amman",
        "Asia/Anadyr",
        "Asia/Aqtau",
        "Asia/Aqtobe",
        "Asia/Ashgabat",
        "Asia/Atyrau",
        "Asia/Baghdad",
        "Asia/Bahrain",
        "Asia/Baku",
        "Asia/Bangkok",
        "Asia/Barnaul",
        "Asia/Beirut",
        "Asia/Bishkek",
        "Asia/Brunei",
        "Asia/Chita",
        "Asia/Colombo",
        "Asia/Damascus",
        "Asia/Dhaka",
        "Asia/Dili",
        "Asia/Dubai",
        "Asia/Dushanbe",
        "Asia/Famagusta",
        "Asia/Gaza",
        "Asia/Hebron",
        "Asia/Ho_Chi_Minh",
        "Asia/Hong_Kong",
        "Asia/Hovd",
        "Asia/Irkutsk",
        "Asia/Jakarta",
        "Asia/Jayapura",
        "Asia/Jerusalem",
        "Asia/Kabul",
        "Asia/Kamchatka",
        "Asia/Karachi",
        "Asia/Kathmandu",
        "Asia/Khandyga",
        "Asia/Kolkata",
        "Asia/Krasnoyarsk",
        "Asia/Kuala_Lumpur",
        "Asia/Kuching",
        "Asia/Kuwait",
        "Asia/Macau",
        "Asia/Magadan",
        "Asia/Makassar",
        "Asia/Manila",
        "Asia/Muscat",
        "Asia/Nicosia",
        "Asia/Novokuznetsk",
        "Asia/Novosibirsk",
        "Asia/Omsk",
        "Asia/Oral",
        "Asia/Phnom_Penh",
        "Asia/Pontianak",
        "Asia/Pyongyang",
        "Asia/Qatar",
        "Asia/Qostanay",
        "Asia/Qyzylorda",
        "Asia/Riyadh",
        "Asia/Sakhalin",
        "Asia/Samarkand",
        "Asia/Seoul",
        "Asia/Shanghai",
        "Asia/Singapore",
        "Asia/Srednekolymsk",
        "Asia/Taipei",
        "Asia/Tashkent",
        "Asia/Tbilisi",
        "Asia/Tehran",
        "Asia/Thimphu",
        "Asia/Tokyo",
        "Asia/Tomsk",
        "Asia/Ulaanbaatar",
        "Asia/Urumqi",
        "Asia/Ust-Nera",
        "Asia/Vientiane",
        "Asia/Vladivostok",
        "Asia/Yakutsk",
        "Asia/Yangon",
        "Asia/Yekaterinburg",
        "Asia/Yerevan",
        "Atlantic/Azores",
        "Atlantic/Bermuda",
        "Atlantic/Canary",
        "Atlantic/Cape_Verde",
        "Atlantic/Faroe",
        "Atlantic/Madeira",
        "Atlantic/Reykjavik",
        "Atlantic/South_Georgia",
        "Atlantic/St_Helena",
        "Atlantic/Stanley",
        "Australia/Adelaide",
        "Australia/Brisbane",
        "Australia/Broken_Hill",
        "Australia/Darwin",
        "Australia/Eucla",
        "Australia/Hobart",
        "Australia/Lindeman",
        "Australia/Lord_Howe",
        "Australia/Melbourne",
        "Australia/Perth",
        "Australia/Sydney",
        "Europe/Amsterdam",
        "Europe/Andorra",
        "Europe/Astrakhan",
        "Europe/Athens",
        "Europe/Belgrade",
        "Europe/Berlin",
        "Europe/Bratislava",
        "Europe/Brussels",
        "Europe/Bucharest",
        "Europe/Budapest",
        "Europe/Busingen",
        "Europe/Chisinau",
        "Europe/Copenhagen",
        "Europe/Dublin",
        "Europe/Gibraltar",
        "Europe/Guernsey",
        "Europe/Helsinki",
        "Europe/Isle_of_Man",
        "Europe/Istanbul",
        "Europe/Jersey",
        "Europe/Kaliningrad",
        "Europe/Kirov",
        "Europe/Kyiv",
        "Europe/Lisbon",
        "Europe/Ljubljana",
        "Europe/London",
        "Europe/Luxembourg",
        "Europe/Madrid",
        "Europe/Malta",
        "Europe/Mariehamn",
        "Europe/Minsk",
        "Europe/Monaco",
        "Europe/Moscow",
        "Europe/Oslo",
        "Europe/Paris",
        "Europe/Podgorica",
        "Europe/Prague",
        "Europe/Riga",
        "Europe/Rome",
        "Europe/Samara",
        "Europe/San_Marino",
        "Europe/Sarajevo",
        "Europe/Saratov",
        "Europe/Simferopol",
        "Europe/Skopje",
        "Europe/Sofia",
        "Europe/Stockholm",
        "Europe/Tallinn",
        "Europe/Tirane",
        "Europe/Ulyanovsk",
        "Europe/Vaduz",
        "Europe/Vatican",
        "Europe/Vienna",
        "Europe/Vilnius",
        "Europe/Volgograd",
        "Europe/Warsaw",
        "Europe/Zagreb",
        "Europe/Zurich",
        "Indian/Antananarivo",
        "Indian/Chagos",
        "Indian/Christmas",
        "Indian/Cocos",
        "Indian/Comoro",
        "Indian/Kerguelen",
        "Indian/Mahe",
        "Indian/Maldives",
        "Indian/Mauritius",
        "Indian/Mayotte",
        "Indian/Reunion",
        "Pacific/Apia",
        "Pacific/Auckland",
        "Pacific/Bougainville",
        "Pacific/Chatham",
        "Pacific/Chuuk",
        "Pacific/Easter",
        "Pacific/Efate",
        "Pacific/Fakaofo",
        "Pacific/Fiji",
        "Pacific/Funafuti",
        "Pacific/Galapagos",
        "Pacific/Gambier",
        "Pacific/Guadalcanal",
        "Pacific/Guam",
        "Pacific/Honolulu",
        "Pacific/Kanton",
        "Pacific/Kiritimati",
        "Pacific/Kosrae",
        "Pacific/Kwajalein",
        "Pacific/Majuro",
        "Pacific/Marquesas",
        "Pacific/Midway",
        "Pacific/Nauru",
        "Pacific/Niue",
        "Pacific/Norfolk",
        "Pacific/Noumea",
        "Pacific/Pago_Pago",
        "Pacific/Palau",
        "Pacific/Pitcairn",
        "Pacific/Pohnpei",
        "Pacific/Port_Moresby",
        "Pacific/Rarotonga",
        "Pacific/Saipan",
        "Pacific/Tahiti",
        "Pacific/Tarawa",
        "Pacific/Tongatapu",
        "Pacific/Wake",
        "Pacific/Wallis",
    ];

    /// ```
    /// use app::time::Zone;
    ///
    /// assert_eq!(Zone::index("Etc/UTC"), Some(Zone::default()));
    /// assert_eq!(Zone::index("Asia/Tokyo").map(|zone| Zone::LIST[zone.0.value() as usize]), Some("Asia/Tokyo"));
    /// assert_eq!(Zone::index("Asia/Calcutta"), None);
    /// ```
    pub fn index(identifier: &str) -> Option<Self> {
        Self::LIST
            .iter()
            .position(|name| *name == identifier)
            .map(|index| Self(u10::new(index as u16)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Youbi {
    Monday = 1,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}
impl Youbi {
    pub const fn display(self, language: Language) -> &'static str {
        let names = match language {
            Language::En(_) => ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
            Language::Ja => ["月", "火", "水", "木", "金", "土", "日"],
        };
        names[self as usize - 1]
    }
}

bit::field! {
    pub Month(u4),
}

impl Month {
    pub const JANUARY: Self = Self(u4::new(1));
    pub const FEBRUARY: Self = Self(u4::new(2));
    pub const MARCH: Self = Self(u4::new(3));
    pub const APRIL: Self = Self(u4::new(4));
    pub const MAY: Self = Self(u4::new(5));
    pub const JUNE: Self = Self(u4::new(6));
    pub const JULY: Self = Self(u4::new(7));
    pub const AUGUST: Self = Self(u4::new(8));
    pub const SEPTEMBER: Self = Self(u4::new(9));
    pub const OCTOBER: Self = Self(u4::new(10));
    pub const NOVEMBER: Self = Self(u4::new(11));
    pub const DECEMBER: Self = Self(u4::new(12));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Iso,
    WithYoubi,
    WithoutYoubi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Unit {
    Centisecond = 1,
    Second,
    Minute,
    Hour,
    Day,
    Month,
    Year,
}

/// ```
/// use app::time::*;
///
/// let length = Length::new(Unit::Day, 2).with(Unit::Hour, 3);
/// assert_eq!(length.get(Unit::Day), 2);
/// assert_eq!(length.get(Unit::Hour), 3);
/// assert_eq!(length.get(Unit::Minute), 0);
/// assert_eq!(Length::default(), Length::new(Unit::Day, 0));
///
/// let hours = Length::new(Unit::Hour, 9).with(Unit::Minute, 30);
/// assert_eq!(hours.total(Unit::Minute), 570);
/// assert_eq!(hours.total(Unit::Hour), 9);
/// assert_eq!(hours, Length::new(Unit::Minute, 570));
/// assert!(hours < Length::new(Unit::Minute, 571));
/// assert!(Length::new(Unit::Month, 1) > Length::new(Unit::Day, 31));
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Length([u64; 7]);

impl Length {
    pub const fn new(unit: Unit, value: u64) -> Self {
        Self([0; 7]).with(unit, value)
    }

    pub const fn with(self, unit: Unit, value: u64) -> Self {
        let mut values = self.0;
        values[unit as usize - 1] = value;
        Self(values)
    }

    pub const fn get(&self, unit: Unit) -> u64 {
        self.0[unit as usize - 1]
    }

    pub fn total(&self, unit: Unit) -> u64 {
        use Unit::*;
        match unit {
            Year => self.get(Year),
            Month => self.get(Year) * 12 + self.get(Month),
            unit => [(Day, 1), (Hour, 24), (Minute, 60), (Second, 60), (Centisecond, 100)]
                .into_iter()
                .take_while(|(slot, _)| *slot >= unit)
                .fold(0, |total, (slot, scale)| total * scale + self.get(slot)),
        }
    }

    fn key(&self) -> (u64, u64) {
        (self.total(Unit::Centisecond), self.total(Unit::Month))
    }
}

impl PartialEq for Length {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for Length {}

impl PartialOrd for Length {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Length {
    fn cmp(&self, other: &Self) -> Ordering {
        let (key, other) = (self.key(), other.key());
        key.1.cmp(&other.1).then(key.0.cmp(&other.0))
    }
}

/// ```
/// use app::time::*;
///
/// let tokyo = Zone::index("Asia/Tokyo").unwrap();
/// let stamp = from_unix_timestamp_in_ms(946684800000, tokyo);
/// assert_eq!(unpack(stamp), (0, 0, 0, 0, 1, 1, 2000, 1, u64::from(tokyo.0.value())));
/// ```
pub fn from_unix_timestamp_in_ms(unix_timestamp_in_ms: i64, zone: Zone) -> Stamp {
    let centisecond = unix_timestamp_in_ms.rem_euclid(1000) / 10;
    let seconds = unix_timestamp_in_ms.div_euclid(1000);

    let seconds_of_day = seconds.rem_euclid(86400);
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;

    let (year, month, day) = date_of(seconds.div_euclid(86400) + 719468);

    pack(centisecond, second, minute, hour, day, month, year, 1, u64::from(zone.0.value()))
}

/// ```
/// use app::{En, Language, time::{Format, Unit::*, display, pack}};
///
/// let stamp = pack(7, 5, 30, 9, 2, 10, 2026, 0, 0);
/// assert_eq!(display(stamp, Language::Ja, (Year, Day), Format::Iso), "2026-10-02");
/// assert_eq!(display(stamp, Language::Ja, (Year, Minute), Format::Iso), "2026-10-02 09:30");
/// assert_eq!(display(stamp, Language::Ja, (Hour, Minute), Format::Iso), "09:30");
/// assert_eq!(display(stamp, Language::Ja, (Hour, Centisecond), Format::Iso), "09:30:05.07");
/// assert_eq!(display(stamp, Language::Ja, (Month, Day), Format::WithYoubi), "10/2(金)");
/// assert_eq!(display(stamp, Language::Ja, (Month, Day), Format::WithoutYoubi), "10/2");
/// assert_eq!(display(stamp, Language::Ja, (Day, Day), Format::WithoutYoubi), "2");
/// assert_eq!(display(stamp, Language::Ja, (Year, Day), Format::WithoutYoubi), "2026年10月2日");
/// assert_eq!(display(stamp, Language::Ja, (Year, Minute), Format::WithoutYoubi), "2026年10月2日 9時30分");
/// assert_eq!(display(stamp, Language::Ja, (Year, Day), Format::WithYoubi), "2026年10月2日(金)");
/// assert_eq!(display(stamp, Language::Ja, (Year, Minute), Format::WithYoubi), "2026年10月2日(金) 9時30分");
/// assert_eq!(display(stamp, Language::Ja, (Month, Minute), Format::WithYoubi), "10/2(金) 9:30");
/// assert_eq!(display(stamp, Language::Ja, (Hour, Minute), Format::WithYoubi), "9:30");
/// assert_eq!(display(stamp, Language::En(En::Gb), (Month, Day), Format::WithYoubi), "Fri 2 Oct");
/// assert_eq!(display(stamp, Language::En(En::Gb), (Month, Day), Format::WithoutYoubi), "2 Oct");
/// assert_eq!(display(stamp, Language::En(En::Gb), (Year, Day), Format::WithoutYoubi), "2 Oct 2026");
/// assert_eq!(display(stamp, Language::En(En::Gb), (Year, Day), Format::WithYoubi), "Fri 2 Oct 2026");
/// assert_eq!(display(stamp, Language::En(En::Gb), (Year, Minute), Format::WithYoubi), "Fri 2 Oct 2026 09:30");
/// assert_eq!(display(stamp, Language::En(En::Gb), (Month, Minute), Format::WithYoubi), "Fri 2 Oct 09:30");
/// assert_eq!(display(stamp, Language::En(En::Gb), (Year, Month), Format::WithYoubi), "Oct 2026");
/// assert_eq!(display(stamp, Language::En(En::Gb), (Hour, Centisecond), Format::WithYoubi), "09:30:05.07");
/// ```
pub fn display(stamp: Stamp, language: Language, scope: (Unit, Unit), format: Format) -> String {
    use Unit::*;

    let (centisecond, second, minute, hour, day, month, year, ..) = unpack(stamp);
    let within = |unit: Unit| (scope.1..=scope.0).contains(&unit);
    let dates = [(Year, year), (Month, month), (Day, day)];
    let clocks = [(Hour, hour), (Minute, minute), (Second, second), (Centisecond, centisecond)];
    let clock = |padded: bool| {
        clocks.iter().filter(|(unit, _)| within(*unit)).fold(
            String::new(),
            |text, (unit, value)| match (text.is_empty(), unit) {
                (true, _) if padded => format!("{value:02}"),
                (true, _) => format!("{value}"),
                (false, Centisecond) => format!("{text}.{value:02}"),
                (false, _) => format!("{text}:{value:02}"),
            },
        )
    };
    let join = |date: String, clock: String| match (date.is_empty(), clock.is_empty()) {
        (false, false) => format!("{date} {clock}"),
        _ => format!("{date}{clock}"),
    };
    let verbal = within(Year);
    let name = if within(Day) && format == Format::WithYoubi {
        youbi(stamp).display(language)
    } else {
        ""
    };
    match (format, language) {
        (Format::Iso, _) => {
            let date = dates
                .iter()
                .filter(|(unit, _)| within(*unit))
                .map(|(unit, value)| match unit {
                    Year => format!("{value:04}"),
                    _ => format!("{value:02}"),
                })
                .collect::<Vec<_>>()
                .join("-");
            join(date, clock(true))
        }
        (Format::WithYoubi | Format::WithoutYoubi, Language::Ja) if verbal => {
            let date = dates
                .iter()
                .filter(|(unit, _)| within(*unit))
                .map(|(unit, value)| {
                    format!("{value}{}", ["日", "月", "年"][*unit as usize - Day as usize])
                })
                .collect::<String>();
            let suffix = if name.is_empty() { String::new() } else { format!("({name})") };
            let clock = clocks
                .iter()
                .filter(|(unit, _)| within(*unit) && *unit != Centisecond)
                .map(|(unit, value)| match unit {
                    Hour => format!("{value}時"),
                    Minute => format!("{value}分"),
                    _ if within(Centisecond) => format!("{value}.{centisecond:02}秒"),
                    _ => format!("{value}秒"),
                })
                .collect::<String>();
            join(format!("{date}{suffix}"), clock)
        }
        (Format::WithYoubi | Format::WithoutYoubi, Language::Ja) => {
            let date = dates
                .iter()
                .filter(|(unit, _)| within(*unit))
                .map(|(_, value)| format!("{value}"))
                .collect::<Vec<_>>()
                .join("/");
            let suffix = if name.is_empty() { String::new() } else { format!("({name})") };
            join(format!("{date}{suffix}"), clock(false))
        }
        (Format::WithYoubi | Format::WithoutYoubi, Language::En(_)) => {
            let month_name = if within(Month) {
                ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
                    [month as usize - 1]
            } else {
                ""
            };
            let date = [
                (Day, String::from(name)),
                (Day, format!("{day}")),
                (Month, String::from(month_name)),
                (Year, format!("{year}")),
            ]
            .into_iter()
            .filter(|(unit, text)| within(*unit) && !text.is_empty())
            .map(|(_, text)| text)
            .collect::<Vec<_>>()
            .join(" ");
            join(date, clock(true))
        }
    }
}

/// ```
/// use app::time::*;
///
/// assert_eq!(youbi(pack(0, 0, 0, 0, 1, 1, 1970, 0, 0)), Youbi::Thursday);
/// assert_eq!(youbi(pack(0, 0, 0, 0, 4, 10, 2026, 0, 0)), Youbi::Sunday);
/// assert_eq!(youbi(pack(0, 0, 0, 0, 5, 1, 2026, 0, 0)), Youbi::Monday);
/// ```
pub fn youbi(stamp: Stamp) -> Youbi {
    let (.., day, month, year, _, _) = unpack(stamp);
    let month_offsets: [i64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let adjusted_year = if month < 3 { year - 1 } else { year };
    let youbi_index = (adjusted_year + adjusted_year / 4 - adjusted_year / 100
        + adjusted_year / 400
        + month_offsets[(month - 1) as usize]
        + day)
        % 7;
    [
        Youbi::Sunday,
        Youbi::Monday,
        Youbi::Tuesday,
        Youbi::Wednesday,
        Youbi::Thursday,
        Youbi::Friday,
        Youbi::Saturday,
    ][youbi_index as usize]
}

/// ```
/// use app::time::*;
///
/// let from = pack(0, 0, 0, 23, 2, 10, 2026, 0, 0);
/// let to = pack(50, 0, 0, 1, 4, 10, 2026, 0, 0);
/// let centiseconds = (24 + 2) * 3600 * 100 + 50;
/// assert_eq!(diff(from, to, Unit::Centisecond), centiseconds);
/// assert_eq!(diff(to, from, Unit::Centisecond), -centiseconds);
/// assert_eq!(diff(from, to, Unit::Hour), 26);
/// assert_eq!(diff(from, to, Unit::Day), 2);
/// assert_eq!(diff(from, to, Unit::Month), 0);
///
/// let day = 24 * 3600 * 100;
/// let leap = pack(0, 0, 0, 0, 29, 2, 2000, 0, 0);
/// let next = pack(0, 0, 0, 0, 1, 3, 2001, 0, 0);
/// assert_eq!(diff(leap, next, Unit::Centisecond), 366 * day);
/// assert_eq!(diff(leap, next, Unit::Day), 366);
/// assert_eq!(diff(leap, next, Unit::Month), 13);
/// assert_eq!(diff(leap, next, Unit::Year), 1);
/// ```
pub fn diff(stamp: Stamp, other: Stamp, unit: Unit) -> i64 {
    let count = |stamp: Stamp| {
        let (centisecond, second, minute, hour, day, month, year, ..) = unpack(stamp);
        match unit {
            Unit::Year => year,
            Unit::Month => year * 12 + month,
            unit => [(24, hour), (60, minute), (60, second), (100, centisecond)]
                .iter()
                .take(Unit::Day as usize - unit as usize)
                .fold(day_number(year, month, day), |count, (scale, part)| count * scale + part),
        }
    };
    count(other) - count(stamp)
}

/// ```
/// use app::time::*;
///
/// let date = |stamp: Stamp| {
///     let (_, _, minute, hour, day, month, year, ..) = unpack(stamp);
///     (year, month, day, hour, minute)
/// };
/// let at = |year, month, day, hour, minute| pack(0, 0, minute, hour, day, month, year, 0, 0);
/// let length = |unit, value| Length::new(unit, value);
///
/// assert_eq!(date(add(at(2000, 2, 29, 0, 0), length(Unit::Year, 1))), (2001, 2, 28, 0, 0));
/// assert_eq!(date(add(at(2001, 1, 31, 0, 0), length(Unit::Month, 1))), (2001, 2, 28, 0, 0));
/// assert_eq!(date(add(at(2001, 12, 31, 0, 0), length(Unit::Day, 1))), (2002, 1, 1, 0, 0));
/// assert_eq!(date(add(at(2001, 1, 31, 23, 0), length(Unit::Hour, 2))), (2001, 2, 1, 1, 0));
/// assert_eq!(date(add(at(2001, 1, 1, 23, 59), length(Unit::Minute, 2))), (2001, 1, 2, 0, 1));
///
/// let mixed = length(Unit::Month, 1).with(Unit::Day, 1);
/// assert_eq!(date(add(at(2001, 1, 30, 0, 0), mixed)), (2001, 3, 1, 0, 0));
///
/// assert_eq!(date(add(at(0, 0, 0, 9, 30), length(Unit::Minute, 45))), (0, 0, 0, 10, 15));
/// assert_eq!(date(add(at(0, 0, 0, 9, 30), length(Unit::Day, 0))), (0, 0, 0, 9, 30));
/// ```
pub fn add(stamp: Stamp, length: Length) -> Stamp {
    shift_length(stamp, length, 1)
}

/// ```
/// use app::time::*;
///
/// let date = |stamp: Stamp| {
///     let (_, _, minute, hour, day, month, year, ..) = unpack(stamp);
///     (year, month, day, hour, minute)
/// };
/// let at = |year, month, day, hour, minute| pack(0, 0, minute, hour, day, month, year, 0, 0);
/// let length = |unit, value| Length::new(unit, value);
///
/// assert_eq!(date(sub(at(2000, 2, 29, 0, 0), length(Unit::Year, 1))), (1999, 2, 28, 0, 0));
/// assert_eq!(date(sub(at(2002, 3, 1, 0, 0), length(Unit::Month, 14))), (2001, 1, 1, 0, 0));
/// assert_eq!(date(sub(at(2001, 3, 1, 0, 0), length(Unit::Day, 1))), (2001, 2, 28, 0, 0));
/// assert_eq!(date(sub(at(2001, 3, 1, 0, 0), length(Unit::Hour, 2))), (2001, 2, 28, 22, 0));
/// assert_eq!(date(sub(at(2001, 1, 2, 0, 0), length(Unit::Minute, 1))), (2001, 1, 1, 23, 59));
/// assert_eq!(date(sub(at(0, 0, 0, 10, 15), length(Unit::Minute, 45))), (0, 0, 0, 9, 30));
/// ```
pub fn sub(stamp: Stamp, length: Length) -> Stamp {
    shift_length(stamp, length, -1)
}

/// ```
/// use app::time::*;
///
/// let day = pack(0, 0, 30, 9, 2, 10, 2026, 0, 0);
/// assert_eq!(shift(day, Unit::Day, 30), pack(0, 0, 30, 9, 1, 11, 2026, 0, 0));
/// assert_eq!(shift(day, Unit::Day, -2), pack(0, 0, 30, 9, 30, 9, 2026, 0, 0));
/// assert_eq!(shift(day, Unit::Minute, -45), pack(0, 0, 45, 8, 2, 10, 2026, 0, 0));
/// assert_eq!(shift(day, Unit::Month, 0), day);
/// ```
pub fn shift(stamp: Stamp, unit: Unit, value: i64) -> Stamp {
    shift_length(stamp, Length::new(unit, value.unsigned_abs()), value.signum())
}

fn shift_length(stamp: Stamp, length: Length, sign: i64) -> Stamp {
    let value = |unit: Unit| length.get(unit) as i64 * sign;
    let (centisecond, second, minute, hour, day, month, year, is_utc, zone) = unpack(stamp);
    let (year, month, day) = match value(Unit::Year) * 12 + value(Unit::Month) {
        0 => (year, month, day),
        months => {
            let total = year * 12 + (month - 1) + months;
            let (year, month) = (total.div_euclid(12), total.rem_euclid(12) + 1);
            (year, month, day.min(days_in_month(year, month)))
        }
    };
    let total = ((hour * 60 + minute) * 60 + second) * 100
        + centisecond
        + ((value(Unit::Hour) * 60 + value(Unit::Minute)) * 60 + value(Unit::Second)) * 100
        + value(Unit::Centisecond);
    let (year, month, day) = match value(Unit::Day) + total.div_euclid(8_640_000) {
        0 => (year, month, day),
        days => date_of(day_number(year, month, day) + days),
    };
    let time = total.rem_euclid(8_640_000);
    let seconds = time / 100;
    pack(
        time % 100,
        seconds % 60,
        seconds % 3600 / 60,
        seconds / 3600,
        day,
        month,
        year,
        is_utc,
        zone,
    )
}

fn day_number(year: i64, month: i64, day: i64) -> i64 {
    let adjusted_year = if month <= 2 { year - 1 } else { year };
    let year_of_era = adjusted_year.rem_euclid(400);
    let shifted_month = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    adjusted_year.div_euclid(400) * 146097 + year_of_era * 365 + year_of_era / 4 - year_of_era / 100
        + day_of_year
}

fn date_of(day_number: i64) -> (i64, i64, i64) {
    let era = day_number.div_euclid(146097);
    let day_of_era = day_number.rem_euclid(146097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

pub fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

pub fn unpack(stamp: Stamp) -> (i64, i64, i64, i64, i64, i64, i64, u64, u64) {
    (
        i64::from(stamp.get::<Centisecond>().0.value()),
        i64::from(stamp.get::<Second>().0.value()),
        i64::from(stamp.get::<Minute>().0.value()),
        i64::from(stamp.get::<Hour>().0.value()),
        i64::from(stamp.get::<Day>().0.value()),
        i64::from(stamp.get::<Month>().0.value()),
        i64::from(stamp.get::<Year>().0.value()),
        u64::from(stamp.get::<IsUtc>().0.value()),
        u64::from(stamp.get::<Zone>().0.value()),
    )
}

pub fn pack(
    centisecond: i64,
    second: i64,
    minute: i64,
    hour: i64,
    day: i64,
    month: i64,
    year: i64,
    is_utc: u64,
    zone: u64,
) -> Stamp {
    let mut stamp = Stamp::ZERO;
    stamp.set(Centisecond(u7::masked_new(centisecond as u64)));
    stamp.set(Second(u6::masked_new(second as u64)));
    stamp.set(Minute(u6::masked_new(minute as u64)));
    stamp.set(Hour(u5::masked_new(hour as u64)));
    stamp.set(Day(u5::masked_new(day as u64)));
    stamp.set(Month(u4::masked_new(month as u64)));
    stamp.set(Year(u13::masked_new(year as u64)));
    stamp.set(IsUtc(u1::masked_new(is_utc)));
    stamp.set(Zone(u10::masked_new(zone)));
    stamp
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub start:  Stamp,
    pub length: Length,
}

pub struct Recurrence {
    pub range:            Range, // activate 〜 until
    pub months:           Option<Vec<Month>>,
    pub weeks:            Option<Vec<u8>>, // 1th, 2th, 3rd, 4th, 5th, 6th
    pub youbi:            Option<Vec<Youbi>>,
    pub periods_in_a_day: Option<Vec<Range>>,
}

impl Recurrence {
    /// ```
    /// use app::time::*;
    ///
    /// let year_start = pack(0, 0, 0, 0, 1, 1, 2026, 1, 0);
    /// let year_end   = pack(0, 0, 59, 23, 31, 12, 2026, 1, 0);
    /// let year = Length::new(Unit::Day, 364)
    ///     .with(Unit::Hour, 23)
    ///     .with(Unit::Minute, 59);
    /// assert_eq!(add(year_start, year), year_end);
    ///
    /// let morning   = pack(0, 0, 0, 9, 1, 1, 2000, 1, 0);
    /// let afternoon = pack(0, 0, 0, 14, 1, 1, 2000, 1, 0);
    /// let hours = |n| Length::new(Unit::Hour, n);
    ///
    /// let recurrence = Recurrence {
    ///     range: Range { start: year_start, length: year },
    ///     months: Some(vec![Month::JANUARY, Month::MARCH]),
    ///     weeks:  Some(vec![1, 3]),
    ///     youbi:  Some(vec![Youbi::Monday, Youbi::Wednesday]),
    ///     periods_in_a_day: Some(vec![
    ///         Range { start: morning,   length: hours(3) },
    ///         Range { start: afternoon, length: hours(4) },
    ///     ]),
    /// };
    ///
    /// let result = recurrence.occurrences(year_start, year_end);
    ///
    /// assert_eq!(result.len(), 16);
    ///
    /// let (_, _, minute, hour, day, month, year, ..) = unpack(result[0].start);
    /// assert_eq!((year, month, day, hour, minute), (2026, 1, 5, 9, 0));
    /// let (_, _, end_minute, end_hour, ..) = unpack(add(result[0].start, result[0].length));
    /// assert_eq!((end_hour, end_minute), (12, 0));
    /// ```
    pub fn occurrences(&self, start: Stamp, end: Stamp) -> Vec<Range> {
        let start = self.range.start.max(start);
        let until = if self.range.start == Stamp::ZERO || self.range.length == Length::default() {
            Stamp::ZERO
        } else {
            add(self.range.start, self.range.length)
        };
        let end = match (until, end) {
            (Stamp::ZERO, end) => end,
            (until, Stamp::ZERO) => until,
            (until, end) => until.min(end),
        };
        if start == Stamp::ZERO || end == Stamp::ZERO || start > end {
            return vec![];
        }

        let date = |mut stamp: Stamp| {
            stamp.set(Hour::default());
            stamp.set(Minute::default());
            stamp.set(Second::default());
            stamp.set(Centisecond::default());
            stamp
        };
        let day = Length::new(Unit::Day, 1);
        let mut result = Vec::new();
        let mut current = date(start);

        while current <= date(end) {
            let nth = (current.get::<Day>().0.value() - 1) / 7 + 1;
            let selected =
                self.months.as_ref().is_none_or(|months| months.contains(&current.get::<Month>()))
                    && self.weeks.as_ref().is_none_or(|weeks| weeks.contains(&nth))
                    && self.youbi.as_ref().is_none_or(|youbis| youbis.contains(&youbi(current)));

            if selected {
                match &self.periods_in_a_day {
                    Some(periods) => {
                        for period in periods {
                            let mut start = period.start;
                            if start != Stamp::ZERO {
                                start.set(current.get::<Year>());
                                start.set(current.get::<Month>());
                                start.set(current.get::<Day>());
                            }
                            result.push(Range { start, length: period.length });
                        }
                    }
                    None => result.push(Range { start: current, length: day }),
                }
            }

            current = add(current, day);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;
    use crate::{En, Rng};

    fn integer(stamp: Stamp) -> u64 {
        u64::from_le_bytes(stamp.to_le_bytes())
    }

    #[test]
    fn minute_arithmetic_keeps_a_zero_date_within_the_day() {
        let at = |hour, minute| pack(0, 0, minute, hour, 0, 0, 0, 0, 0);
        let minutes = |n| Length::new(Unit::Minute, n);
        assert_eq!(add(at(9, 30), minutes(60)), at(10, 30));
        assert_eq!(add(at(9, 30), minutes(45)), at(10, 15));
        assert_eq!(sub(at(10, 15), minutes(45)), at(9, 30));
        assert_eq!(add(at(9, 30), Length::default()), at(9, 30));
        let hour = 60 * 60 * 100;
        assert_eq!(diff(at(9, 30), at(10, 30), Unit::Centisecond), hour);
        assert_eq!(diff(at(10, 30), at(9, 30), Unit::Centisecond), -hour);
        assert!(at(9, 30) < at(10, 0) && at(10, 0) < at(10, 1));
    }

    #[test]
    fn layout_positions() {
        assert_eq!(integer(pack(0, 0, 0, 0, 0, 0, 1, 0, 0)), 1 << 33);
        assert_eq!(integer(pack(0, 0, 0, 0, 0, 1, 0, 0, 0)), 1 << 29);
        assert_eq!(integer(pack(0, 0, 0, 0, 1, 0, 0, 0, 0)), 1 << 24);
        assert_eq!(integer(pack(0, 0, 0, 1, 0, 0, 0, 0, 0)), 1 << 19);
        assert_eq!(integer(pack(0, 0, 1, 0, 0, 0, 0, 0, 0)), 1 << 13);
        assert_eq!(integer(pack(0, 1, 0, 0, 0, 0, 0, 0, 0)), 1 << 7);
        assert_eq!(integer(pack(1, 0, 0, 0, 0, 0, 0, 0, 0)), 1);
        assert_eq!(integer(pack(0, 0, 0, 0, 0, 0, 0, 1, 0)), 1 << 46);
        assert_eq!(integer(pack(0, 0, 0, 0, 0, 0, 0, 0, 1)), 1 << 47);
    }

    #[test]
    fn layout_bytes_are_little_endian_in_declaration_order() {
        let stamp = pack(0, 0, 0, 0, 1, 1, 2000, 1, 0);
        assert_eq!(integer(stamp), 0x0000_4FA0_2100_0000);
        assert_eq!(stamp.to_le_bytes(), [0x00, 0x00, 0x00, 0x21, 0xA0, 0x4F, 0x00, 0x00]);
        assert_eq!(unpack(stamp), (0, 0, 0, 0, 1, 1, 2000, 1, 0));
    }

    #[test]
    fn parts_carry_the_field_widths_and_round_trip() {
        let mut rng = Rng::new(0);
        for _ in 0..3000 {
            let year = rng.below(8192) as i64;
            let month = 1 + rng.below(12) as i64;
            let day = 1 + rng.below(days_in_month(year, month) as usize) as i64;
            let (hour, minute, second) =
                (rng.below(24) as i64, rng.below(60) as i64, rng.below(60) as i64);
            let (centisecond, is_utc) = (rng.below(100) as i64, rng.below(2) as u64);
            let zone = rng.below(Zone::LIST.len()) as u64;
            let stamp = pack(centisecond, second, minute, hour, day, month, year, is_utc, zone);

            assert_eq!(i64::from(stamp.get::<Year>().0.value()), year);
            assert_eq!(i64::from(stamp.get::<Month>().0.value()), month);
            assert_eq!(i64::from(stamp.get::<Day>().0.value()), day);
            assert_eq!(i64::from(stamp.get::<Hour>().0.value()), hour);
            assert_eq!(i64::from(stamp.get::<Minute>().0.value()), minute);
            assert_eq!(i64::from(stamp.get::<Second>().0.value()), second);
            assert_eq!(i64::from(stamp.get::<Centisecond>().0.value()), centisecond);
            assert_eq!(u64::from(stamp.get::<IsUtc>().0.value()), is_utc);
            assert_eq!(u64::from(stamp.get::<Zone>().0.value()), zone);

            let mut rebuilt = Stamp::ZERO;
            rebuilt.set(stamp.get::<Centisecond>());
            rebuilt.set(stamp.get::<Second>());
            rebuilt.set(stamp.get::<Minute>());
            rebuilt.set(stamp.get::<Hour>());
            rebuilt.set(stamp.get::<Day>());
            rebuilt.set(stamp.get::<Month>());
            rebuilt.set(stamp.get::<Year>());
            rebuilt.set(stamp.get::<IsUtc>());
            rebuilt.set(stamp.get::<Zone>());
            assert_eq!(rebuilt, stamp);
            assert_eq!(Stamp::from_le_bytes(stamp.to_le_bytes()), stamp);
        }
    }

    #[test]
    fn zones_are_unique_and_fit_the_field() {
        let mut names = Zone::LIST.to_vec();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), Zone::LIST.len());
        assert!(Zone::LIST.len() < 1 << 10);
    }

    #[test]
    fn diff_spans_the_whole_range_in_every_unit() {
        let maxes: [(Unit, i64); 7] = [
            (Unit::Year, 8_191),
            (Unit::Month, 98_303),
            (Unit::Day, 2_992_066),
            (Unit::Hour, 71_809_607),
            (Unit::Minute, 4_308_576_479),
            (Unit::Second, 258_514_588_799),
            (Unit::Centisecond, 25_851_458_879_999),
        ];
        let first = pack(0, 0, 0, 0, 1, 1, 0, 0, 0);
        let last = pack(99, 59, 59, 23, 31, 12, 8191, 0, 0);
        for (unit, max) in maxes {
            assert_eq!(diff(first, last, unit), max, "{unit:?}");
        }
    }

    #[test]
    fn decode_rejects_only_values_outside_each_field() {
        let decode = |stamp: Stamp| Stamp::decode(&mut &stamp.to_le_bytes()[..]);
        let last_zone = Zone::LIST.len() as u64 - 1;
        for stamp in [
            Stamp::ZERO,
            pack(99, 59, 59, 23, 31, 12, 8191, 1, last_zone),
            pack(0, 0, 0, 24, 1, 1, 2026, 0, 0),
            pack(0, 0, 0, 0, 29, 2, 2028, 0, 0),
            pack(0, 0, 0, 0, 0, 0, 2026, 0, 0),
        ] {
            assert_eq!(decode(stamp), Some(stamp), "{stamp:?}");
        }
        for stamp in [
            pack(100, 0, 0, 0, 1, 1, 2026, 0, 0),
            pack(0, 60, 0, 0, 1, 1, 2026, 0, 0),
            pack(0, 0, 60, 0, 1, 1, 2026, 0, 0),
            pack(0, 0, 0, 25, 1, 1, 2026, 0, 0),
            pack(0, 0, 1, 24, 1, 1, 2026, 0, 0),
            pack(0, 0, 0, 0, 1, 13, 2026, 0, 0),
            pack(0, 0, 0, 0, 29, 2, 2026, 0, 0),
            pack(0, 0, 0, 0, 1, 1, 2026, 0, last_zone + 1),
        ] {
            assert_eq!(decode(stamp), None, "{stamp:?}");
        }
    }

    #[test]
    fn layout_widths_do_not_overlap_and_leave_padding_zero() {
        let all = pack(127, 63, 63, 31, 31, 15, 8191, 1, 1023);
        assert_eq!(integer(all), !0u64 >> 7);
        assert_eq!(Stamp::BITS, 57);
    }

    #[test]
    fn display_format() {
        use Unit::*;

        let stamp = pack(10, 9, 8, 7, 6, 5, 2026, 1, 0);
        assert_eq!(display(stamp, Language::Ja, (Year, Minute), Format::Iso), "2026-05-06 07:08");
        assert_eq!(display(stamp, Language::Ja, (Year, Day), Format::Iso), "2026-05-06");
        assert_eq!(display(stamp, Language::Ja, (Hour, Minute), Format::Iso), "07:08");
        assert_eq!(display(stamp, Language::Ja, (Month, Day), Format::WithYoubi), "5/6(水)");
        assert_eq!(
            display(stamp, Language::Ja, (Year, Day), Format::WithYoubi),
            "2026年5月6日(水)"
        );
        assert_eq!(
            display(stamp, Language::Ja, (Year, Centisecond), Format::WithYoubi),
            "2026年5月6日(水) 7時8分9.10秒"
        );
        assert_eq!(display(stamp, Language::Ja, (Year, Year), Format::WithYoubi), "2026年");
        assert_eq!(display(stamp, Language::Ja, (Day, Day), Format::WithYoubi), "6(水)");
        assert_eq!(display(stamp, Language::Ja, (Minute, Second), Format::Iso), "08:09");
        assert_eq!(display(stamp, Language::En(En::Gb), (Year, Year), Format::WithYoubi), "2026");
        assert_eq!(display(stamp, Language::En(En::Gb), (Day, Day), Format::WithYoubi), "Wed 6");
    }

    fn leap(year: i64) -> bool {
        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }

    fn month_days(year: i64, month: i64) -> i64 {
        let february = if leap(year) { 29 } else { 28 };
        [31, february, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][month as usize - 1]
    }

    fn year_days(year: i64) -> i64 {
        if leap(year) { 366 } else { 365 }
    }

    fn days_since_epoch(year: i64, month: i64, day: i64) -> i64 {
        let mut days = 0;
        if year >= 1970 {
            (1970..year).for_each(|y| days += year_days(y));
        } else {
            (year..1970).for_each(|y| days -= year_days(y));
        }
        (1..month).for_each(|m| days += month_days(year, m));
        days + day - 1
    }

    fn civil(days: i64) -> (i64, i64, i64) {
        let (mut year, mut left) = (1970, days);
        while left < 0 {
            year -= 1;
            left += year_days(year);
        }
        while left >= year_days(year) {
            left -= year_days(year);
            year += 1;
        }
        let mut month = 1;
        while left >= month_days(year, month) {
            left -= month_days(year, month);
            month += 1;
        }
        (year, month, left + 1)
    }

    fn centiseconds(stamp: Stamp) -> i64 {
        let (centisecond, second, minute, hour, day, month, year, ..) = unpack(stamp);
        days_since_epoch(year, month, day) * 8_640_000
            + ((hour * 60 + minute) * 60 + second) * 100
            + centisecond
    }

    fn at_centiseconds(flags_from: Stamp, total: i64) -> Stamp {
        let (.., is_utc, zone) = unpack(flags_from);
        let (year, month, day) = civil(total.div_euclid(8_640_000));
        let time = total.rem_euclid(8_640_000);
        let (seconds, centisecond) = (time / 100, time % 100);
        pack(
            centisecond,
            seconds % 60,
            seconds % 3600 / 60,
            seconds / 3600,
            day,
            month,
            year,
            is_utc,
            zone,
        )
    }

    fn step(stamp: Stamp, unit: Unit, value: i64) -> Stamp {
        let length = Length::new(unit, value.unsigned_abs());
        if value < 0 { sub(stamp, length) } else { add(stamp, length) }
    }

    fn random_stamp(rng: &mut Rng) -> Stamp {
        let year = if rng.chance(30) {
            [1900, 2000, 2100, 2400][rng.below(4)]
        } else {
            1700 + rng.below(800) as i64
        };
        let month = 1 + rng.below(12) as i64;
        let day = if month == 2 && rng.chance(50) {
            month_days(year, 2)
        } else {
            1 + rng.below(month_days(year, month) as usize) as i64
        };
        pack(
            rng.below(100) as i64,
            rng.below(60) as i64,
            rng.below(60) as i64,
            rng.below(24) as i64,
            day,
            month,
            year,
            rng.below(2) as u64,
            rng.below(10) as u64,
        )
    }

    #[test]
    fn calendar_arithmetic_agrees_with_a_naive_day_by_day_reference() {
        for seed in 0..3000 {
            let mut rng = Rng::new(seed);
            let first = random_stamp(&mut rng);
            let second = random_stamp(&mut rng);
            let context = format!("seed {seed}");

            let date = |stamp: Stamp| {
                let (_, second, minute, hour, day, month, year, ..) = unpack(stamp);
                (year, month, day, hour, minute, second)
            };
            let (from, to) = (date(first), date(second));
            let since = |d: (i64, i64, i64, i64, i64, i64)| days_since_epoch(d.0, d.1, d.2);
            for (unit, expected) in [
                (Unit::Centisecond, centiseconds(second) - centiseconds(first)),
                (
                    Unit::Second,
                    (since(to) * 86_400 + (to.3 * 60 + to.4) * 60 + to.5)
                        - (since(from) * 86_400 + (from.3 * 60 + from.4) * 60 + from.5),
                ),
                (
                    Unit::Minute,
                    (since(to) * 1_440 + to.3 * 60 + to.4)
                        - (since(from) * 1_440 + from.3 * 60 + from.4),
                ),
                (Unit::Hour, (since(to) * 24 + to.3) - (since(from) * 24 + from.3)),
                (Unit::Day, since(to) - since(from)),
                (Unit::Month, (to.0 * 12 + to.1) - (from.0 * 12 + from.1)),
                (Unit::Year, to.0 - from.0),
            ] {
                assert_eq!(diff(first, second, unit), expected, "{context} {unit:?}");
                assert_eq!(diff(second, first, unit), -expected, "{context} {unit:?}");
            }
            let delta = rng.below(2_000_000_001) as i64 - 1_000_000_000;
            let third = at_centiseconds(first, centiseconds(first) + delta);
            assert_eq!(first.cmp(&third), 0.cmp(&delta), "{context} order");

            let days = rng.below(80001) as i64 - 40000;
            let hours = rng.below(200001) as i64 - 100000;
            let minutes = rng.below(2_000_001) as i64 - 1_000_000;
            for (moved, unit, name) in [
                (step(first, Unit::Day, days), 8_640_000, days),
                (step(first, Unit::Hour, hours), 360_000, hours),
                (step(first, Unit::Minute, minutes), 6_000, minutes),
            ]
            .map(|(moved, unit, value)| (moved, unit * value, value))
            {
                assert_eq!(
                    moved,
                    at_centiseconds(first, centiseconds(first) + unit),
                    "{context} {name}"
                );
            }
            assert_eq!(step(step(first, Unit::Day, days), Unit::Day, -days), first, "{context}");

            let (.., day, month, year, _, _) = unpack(first);
            let weekday = (days_since_epoch(year, month, day) + 4).rem_euclid(7);
            let expected = [
                Youbi::Sunday,
                Youbi::Monday,
                Youbi::Tuesday,
                Youbi::Wednesday,
                Youbi::Thursday,
                Youbi::Friday,
                Youbi::Saturday,
            ][weekday as usize];
            assert_eq!(youbi(first), expected, "{context}");

            let century = [1900, 2000, 2100, 2400][rng.below(4)];
            let months = if rng.chance(30) {
                (century * 12 + rng.below(12) as i64) - (year * 12 + month - 1)
            } else {
                rng.below(2401) as i64 - 1200
            };
            let total = year * 12 + month - 1 + months;
            let (target_year, target_month) = (total.div_euclid(12), total.rem_euclid(12) + 1);
            let moved = unpack(step(first, Unit::Month, months));
            assert_eq!(
                (moved.6, moved.5, moved.4),
                (target_year, target_month, day.min(month_days(target_year, target_month))),
                "{context} months {months}"
            );
            let years = if rng.chance(30) { century - year } else { rng.below(201) as i64 - 100 };
            let moved = unpack(step(first, Unit::Year, years));
            assert_eq!(
                (moved.6, moved.5, moved.4),
                (year + years, month, day.min(month_days(year + years, month))),
                "{context} years {years}"
            );
            for shifted in [
                step(first, Unit::Year, years),
                step(first, Unit::Month, months),
                step(first, Unit::Day, days),
                step(first, Unit::Hour, hours),
                step(first, Unit::Minute, minutes),
            ] {
                let (centisecond, second_of_minute, .., is_utc, zone) = unpack(shifted);
                let (original_centisecond, original_second, .., original_utc, original_zone) =
                    unpack(first);
                assert_eq!(
                    (second_of_minute, centisecond, is_utc, zone),
                    (original_second, original_centisecond, original_utc, original_zone),
                    "{context}"
                );
            }
        }
    }

    #[test]
    fn from_unix_timestamp_in_ms_matches_the_reference_and_keeps_the_zone() {
        for seed in 0..3000 {
            let mut rng = Rng::new(seed);
            let milliseconds =
                (rng.below(4_000_000) as i64 - 1_000_000) * 86_400 + rng.below(86_400_000) as i64;
            let zone = Zone(u10::new(rng.below(Zone::LIST.len()) as u16));
            let (year, month, day) = civil(milliseconds.div_euclid(86_400_000));
            let time = milliseconds.rem_euclid(86_400_000);
            let expected = (
                time % 1000 / 10,
                time % 60_000 / 1000,
                time % 3_600_000 / 60_000,
                time / 3_600_000,
                day,
                month,
                year,
                1,
                u64::from(zone.0.value()),
            );
            assert_eq!(
                unpack(from_unix_timestamp_in_ms(milliseconds, zone)),
                expected,
                "seed {seed} ms {milliseconds}"
            );
        }
    }
}
