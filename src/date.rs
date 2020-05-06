// from async-h1

use std::fmt::{self, Display, Formatter};
use std::str::from_utf8;
use std::time::{SystemTime, UNIX_EPOCH};

const YEAR_9999_SECONDS: u64 = 253402300800;
const SECONDS_IN_DAY: u64 = 86400;
const SECONDS_IN_HOUR: u64 = 3600;

#[derive(Copy, Clone, Debug)]
pub struct HttpDate {
    /// 0...59
    second: u8,
    /// 0...59
    minute: u8,
    /// 0...23
    hour: u8,
    /// 1...31
    day: u8,
    /// 1...12
    month: u8,
    /// 1970...9999
    year: u16,
    /// 1...7
    week_day: u8,
}

pub(crate) fn fmt_http_date(d: SystemTime) -> String {
    format!("{}", HttpDate::from(d))
}

impl From<SystemTime> for HttpDate {
    fn from(system_time: SystemTime) -> Self {
        let dur = system_time
            .duration_since(UNIX_EPOCH)
            .expect("all times should be after the epoch");
        let secs_since_epoch = dur.as_secs();

        if secs_since_epoch >= YEAR_9999_SECONDS {
            // year 9999
            panic!("date must be before year 9999");
        }

        /* 2000-03-01 (mod 400 year, immediately after feb29 */
        const LEAPOCH: i64 = 11017;
        const DAYS_PER_400Y: i64 = 365 * 400 + 97;
        const DAYS_PER_100Y: i64 = 365 * 100 + 24;
        const DAYS_PER_4Y: i64 = 365 * 4 + 1;

        let days = (secs_since_epoch / SECONDS_IN_DAY) as i64 - LEAPOCH;
        let secs_of_day = secs_since_epoch % SECONDS_IN_DAY;

        let mut qc_cycles = days / DAYS_PER_400Y;
        let mut remdays = days % DAYS_PER_400Y;

        if remdays < 0 {
            remdays += DAYS_PER_400Y;
            qc_cycles -= 1;
        }

        let mut c_cycles = remdays / DAYS_PER_100Y;
        if c_cycles == 4 {
            c_cycles -= 1;
        }
        remdays -= c_cycles * DAYS_PER_100Y;

        let mut q_cycles = remdays / DAYS_PER_4Y;
        if q_cycles == 25 {
            q_cycles -= 1;
        }
        remdays -= q_cycles * DAYS_PER_4Y;

        let mut remyears = remdays / 365;
        if remyears == 4 {
            remyears -= 1;
        }
        remdays -= remyears * 365;

        let mut year = 2000 + remyears + 4 * q_cycles + 100 * c_cycles + 400 * qc_cycles;

        let months = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];
        let mut month = 0;
        for month_len in months.iter() {
            month += 1;
            if remdays < *month_len {
                break;
            }
            remdays -= *month_len;
        }
        let mday = remdays + 1;
        let month = if month + 2 > 12 {
            year += 1;
            month - 10
        } else {
            month + 2
        };

        let mut week_day = (3 + days) % 7;
        if week_day <= 0 {
            week_day += 7
        };

        HttpDate {
            second: (secs_of_day % 60) as u8,
            minute: ((secs_of_day % SECONDS_IN_HOUR) / 60) as u8,
            hour: (secs_of_day / SECONDS_IN_HOUR) as u8,
            day: mday as u8,
            month: month as u8,
            year: year as u16,
            week_day: week_day as u8,
        }
    }
}

impl Display for HttpDate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let week_day = match self.week_day {
            1 => b"Mon",
            2 => b"Tue",
            3 => b"Wed",
            4 => b"Thu",
            5 => b"Fri",
            6 => b"Sat",
            7 => b"Sun",
            _ => unreachable!(),
        };
        let month = match self.month {
            1 => b"Jan",
            2 => b"Feb",
            3 => b"Mar",
            4 => b"Apr",
            5 => b"May",
            6 => b"Jun",
            7 => b"Jul",
            8 => b"Aug",
            9 => b"Sep",
            10 => b"Oct",
            11 => b"Nov",
            12 => b"Dec",
            _ => unreachable!(),
        };
        let mut buf: [u8; 29] = [
            // Too long to write as: b"Thu, 01 Jan 1970 00:00:00 GMT"
            b' ', b' ', b' ', b',', b' ', b'0', b'0', b' ', b' ', b' ', b' ', b' ', b'0', b'0',
            b'0', b'0', b' ', b'0', b'0', b':', b'0', b'0', b':', b'0', b'0', b' ', b'G', b'M',
            b'T',
        ];
        buf[0] = week_day[0];
        buf[1] = week_day[1];
        buf[2] = week_day[2];
        buf[5] = b'0' + (self.day / 10) as u8;
        buf[6] = b'0' + (self.day % 10) as u8;
        buf[8] = month[0];
        buf[9] = month[1];
        buf[10] = month[2];
        buf[12] = b'0' + (self.year / 1000) as u8;
        buf[13] = b'0' + (self.year / 100 % 10) as u8;
        buf[14] = b'0' + (self.year / 10 % 10) as u8;
        buf[15] = b'0' + (self.year % 10) as u8;
        buf[17] = b'0' + (self.hour / 10) as u8;
        buf[18] = b'0' + (self.hour % 10) as u8;
        buf[20] = b'0' + (self.minute / 10) as u8;
        buf[21] = b'0' + (self.minute % 10) as u8;
        buf[23] = b'0' + (self.second / 10) as u8;
        buf[24] = b'0' + (self.second % 10) as u8;
        f.write_str(from_utf8(&buf[..]).unwrap())
    }
}
