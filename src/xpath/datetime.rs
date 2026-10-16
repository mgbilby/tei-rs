//! XML Schema dates, times and durations: `xs:dateTime`, `xs:date`,
//! `xs:time`, the five `xs:g…` types, and `xs:duration` with its subtypes
//! `xs:dayTimeDuration` and `xs:yearMonthDuration`. Their lexical forms,
//! casts, comparisons and arithmetic follow XPath 2.0 Functions and
//! Operators.
//!
//! Years follow XML Schema 1.1, as Saxon does: there is a year 0 (1 BCE), so
//! `-0001` is 2 BCE. The implicit timezone is always UTC, so values without
//! a timezone compare (and subtract) the same wherever validation runs.
//! Fractions of a second are kept to the nanosecond.

use std::cmp::Ordering;
use std::fmt;

use super::XError;
use super::ast::ArithmeticOp;
use super::value::Atomic;

const NANOS_PER_SECOND: i128 = 1_000_000_000;
const NANOS_PER_MINUTE: i128 = 60 * NANOS_PER_SECOND;
const NANOS_PER_HOUR: i128 = 60 * NANOS_PER_MINUTE;
const NANOS_PER_DAY: i128 = 24 * NANOS_PER_HOUR;
/// Years beyond this (nine digits) are rejected, so date arithmetic can
/// never overflow.
const MAX_YEAR: i64 = 999_999_999;
/// Timezones are at most 14 hours from UTC.
const MAX_TIMEZONE_MINUTES: i16 = 14 * 60;

/// Which date/time type a [`DateTime`] has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DateKind {
    DateTime,
    Date,
    Time,
    GYearMonth,
    GYear,
    GMonthDay,
    GDay,
    GMonth,
}

impl DateKind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::DateTime => "xs:dateTime",
            Self::Date => "xs:date",
            Self::Time => "xs:time",
            Self::GYearMonth => "xs:gYearMonth",
            Self::GYear => "xs:gYear",
            Self::GMonthDay => "xs:gMonthDay",
            Self::GDay => "xs:gDay",
            Self::GMonth => "xs:gMonth",
        }
    }

    /// Whether `lt`, `gt` and friends are defined (the `g…` types only
    /// have `eq` and `ne`).
    pub(crate) fn is_ordered(self) -> bool {
        matches!(self, Self::DateTime | Self::Date | Self::Time)
    }
}

/// A value of one of the date/time types. Components the type doesn't have
/// hold the reference values Functions and Operators compares with (the
/// date of an `xs:time` is 1972-12-31, the month of an `xs:gYear` is 1, …),
/// so every value is a point in time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DateTime {
    pub kind: DateKind,
    pub year: i64,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub nanos: u32,
    /// Minutes east of UTC, or `None` for a value without a timezone.
    pub timezone: Option<i16>,
}

/// Which duration type a [`Duration`] has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DurationKind {
    Duration,
    DayTime,
    YearMonth,
}

impl DurationKind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Duration => "xs:duration",
            Self::DayTime => "xs:dayTimeDuration",
            Self::YearMonth => "xs:yearMonthDuration",
        }
    }
}

/// A duration: months and nanoseconds, never of opposite signs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Duration {
    pub kind: DurationKind,
    pub months: i64,
    pub nanos: i128,
}

pub(crate) fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub(crate) fn days_in_month(year: i64, month: u8) -> u8 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 in the proleptic Gregorian calendar, with a year 0
/// (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: u8, day: u8) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month = i64::from(month);
    let day_of_year =
        (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The inverse of [`days_from_civil`].
fn civil_from_days(days: i64) -> (i64, u8, u8) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (
        year_of_era + era * 400 + i64::from(month <= 2),
        month as u8,
        day as u8,
    )
}

fn overflow() -> XError {
    XError::new("FODT0001", "date/time overflow")
}

fn duration_overflow() -> XError {
    XError::new("FODT0002", "duration overflow")
}

/// A cursor over the characters of a lexical form.
struct Lexer<'t> {
    bytes: &'t [u8],
    at: usize,
}

impl Lexer<'_> {
    fn eat(&mut self, byte: u8) -> bool {
        let found = self.bytes.get(self.at) == Some(&byte);
        if found {
            self.at += 1;
        }
        found
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    /// A run of at least one digit.
    fn digit_run(&mut self) -> Option<&[u8]> {
        let start = self.at;
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.at += 1;
        }
        (self.at > start).then(|| &self.bytes[start..self.at])
    }

    /// Exactly `count` digits.
    fn digits(&mut self, count: usize) -> Option<u32> {
        let run = self.bytes.get(self.at..self.at + count)?;
        if !run.iter().all(u8::is_ascii_digit)
            || self
                .bytes
                .get(self.at + count)
                .is_some_and(u8::is_ascii_digit)
        {
            return None;
        }
        self.at += count;
        Some(
            run.iter()
                .fold(0, |value, b| value * 10 + u32::from(b - b'0')),
        )
    }

    fn done(&self) -> bool {
        self.at == self.bytes.len()
    }

    /// `-?YYYY`: at least four digits, and no leading zero beyond four.
    fn year(&mut self) -> Option<i64> {
        let negative = self.eat(b'-');
        let run = self.digit_run()?;
        if run.len() < 4 || run.len() > 9 || (run.len() > 4 && run[0] == b'0') {
            return None;
        }
        let value = run
            .iter()
            .fold(0i64, |value, b| value * 10 + i64::from(b - b'0'));
        Some(if negative { -value } else { value })
    }

    /// `hh:mm:ss(.s+)?`, as (hour, minute, second, nanoseconds).
    fn time(&mut self) -> Option<(u8, u8, u8, u32)> {
        let hour = self.digits(2)?;
        if !self.eat(b':') {
            return None;
        }
        let minute = self.digits(2)?;
        if !self.eat(b':') {
            return None;
        }
        let second = self.digits(2)?;
        let mut nanos = 0;
        if self.eat(b'.') {
            let fraction = self.digit_run()?;
            // Digits beyond the ninth are dropped.
            for (index, b) in fraction.iter().take(9).enumerate() {
                nanos += u32::from(b - b'0') * 10u32.pow(8 - index as u32);
            }
        }
        let valid = (hour < 24 && minute < 60 && second < 60)
            || (hour == 24 && minute == 0 && second == 0 && nanos == 0);
        valid.then_some((hour as u8, minute as u8, second as u8, nanos))
    }

    /// An optional timezone: `Z` or `±hh:mm`.
    fn timezone(&mut self) -> Option<Option<i16>> {
        if self.eat(b'Z') {
            return Some(Some(0));
        }
        let sign = match self.peek() {
            Some(b'+') => 1,
            Some(b'-') => -1,
            _ => return Some(None),
        };
        self.at += 1;
        let hours = self.digits(2)?;
        if !self.eat(b':') {
            return None;
        }
        let minutes = self.digits(2)?;
        if hours > 14 || minutes > 59 {
            return None;
        }
        Some(Some(sign * (hours * 60 + minutes) as i16))
    }
}

impl DateTime {
    /// A value of `kind` from its components, with the reference values for
    /// the components the kind doesn't have.
    #[allow(clippy::too_many_arguments)]
    fn new(
        kind: DateKind,
        year: i64,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        nanos: u32,
        timezone: Option<i16>,
    ) -> Self {
        let (year, month, day) = match kind {
            DateKind::DateTime | DateKind::Date => (year, month, day),
            DateKind::Time => (1972, 12, 31),
            DateKind::GYearMonth => (year, month, 1),
            DateKind::GYear => (year, 1, 1),
            DateKind::GMonthDay => (1972, month, day),
            DateKind::GDay => (1972, 12, day),
            DateKind::GMonth => (1972, month, 1),
        };
        let (hour, minute, second, nanos) = match kind {
            DateKind::DateTime | DateKind::Time => (hour, minute, second, nanos),
            _ => (0, 0, 0, 0),
        };
        Self {
            kind,
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanos,
            timezone,
        }
    }

    /// Parses the lexical form of `kind` (surrounding whitespace allowed).
    pub(crate) fn parse(kind: DateKind, text: &str) -> Option<Self> {
        let text = text.trim_matches([' ', '\t', '\n', '\r']);
        let mut lexer = Lexer {
            bytes: text.as_bytes(),
            at: 0,
        };
        let (mut year, mut month, mut day) = (1972, 1, 1);
        let mut time = (0, 0, 0, 0);
        match kind {
            DateKind::DateTime | DateKind::Date | DateKind::GYearMonth | DateKind::GYear => {
                year = lexer.year()?;
                if kind != DateKind::GYear {
                    if !lexer.eat(b'-') {
                        return None;
                    }
                    month = lexer.digits(2)? as u8;
                }
                if matches!(kind, DateKind::DateTime | DateKind::Date) {
                    if !lexer.eat(b'-') {
                        return None;
                    }
                    day = lexer.digits(2)? as u8;
                }
                if kind == DateKind::DateTime {
                    if !lexer.eat(b'T') {
                        return None;
                    }
                    time = lexer.time()?;
                }
            }
            DateKind::Time => time = lexer.time()?,
            DateKind::GMonthDay | DateKind::GMonth => {
                if !(lexer.eat(b'-') && lexer.eat(b'-')) {
                    return None;
                }
                month = lexer.digits(2)? as u8;
                if kind == DateKind::GMonthDay {
                    if !lexer.eat(b'-') {
                        return None;
                    }
                    day = lexer.digits(2)? as u8;
                }
            }
            DateKind::GDay => {
                if !(lexer.eat(b'-') && lexer.eat(b'-') && lexer.eat(b'-')) {
                    return None;
                }
                day = lexer.digits(2)? as u8;
            }
        }
        let timezone = lexer.timezone()?;
        if !lexer.done() || !(1..=12).contains(&month) || day == 0 {
            return None;
        }
        let last_day = match kind {
            DateKind::DateTime | DateKind::Date => days_in_month(year, month),
            // February 29 exists in some year.
            DateKind::GMonthDay => days_in_month(2000, month),
            _ => 31,
        };
        if day > last_day {
            return None;
        }
        let (hour, minute, second, nanos) = time;
        let value = Self::new(
            kind, year, month, day, hour, minute, second, nanos, timezone,
        );
        if hour == 24 {
            // 24:00:00 is midnight at the end of the day.
            let midnight = Self { hour: 0, ..value };
            return match kind {
                DateKind::Time => Some(midnight),
                _ => midnight.add_nanos(NANOS_PER_DAY).ok(),
            };
        }
        Some(value)
    }

    /// Nanoseconds since 1970-01-01T00:00:00 in the value's own timezone,
    /// ignoring it.
    fn local_nanos(&self) -> i128 {
        i128::from(days_from_civil(self.year, self.month, self.day)) * NANOS_PER_DAY
            + i128::from(self.hour) * NANOS_PER_HOUR
            + i128::from(self.minute) * NANOS_PER_MINUTE
            + i128::from(self.second) * NANOS_PER_SECOND
            + i128::from(self.nanos)
    }

    /// The point in time, as nanoseconds since 1970-01-01T00:00:00Z (a
    /// value without a timezone is in the implicit timezone, UTC).
    pub(crate) fn instant(&self) -> i128 {
        self.local_nanos() - i128::from(self.timezone.unwrap_or(0)) * NANOS_PER_MINUTE
    }

    /// The value of the same kind and timezone at `local` nanoseconds since
    /// 1970-01-01T00:00:00 (dropping the time of day for types without one).
    fn at_local_nanos(&self, local: i128) -> Result<Self, XError> {
        let days = local.div_euclid(NANOS_PER_DAY);
        let time = local.rem_euclid(NANOS_PER_DAY);
        let days = i64::try_from(days).map_err(|_| overflow())?;
        let (year, month, day) = civil_from_days(days);
        if year.abs() > MAX_YEAR {
            return Err(overflow());
        }
        Ok(Self::new(
            self.kind,
            year,
            month,
            day,
            (time / NANOS_PER_HOUR) as u8,
            (time % NANOS_PER_HOUR / NANOS_PER_MINUTE) as u8,
            (time % NANOS_PER_MINUTE / NANOS_PER_SECOND) as u8,
            (time % NANOS_PER_SECOND) as u32,
            self.timezone,
        ))
    }

    /// Adds a day-time duration: an `xs:date` moves as midnight that day
    /// would, and an `xs:time` wraps around midnight.
    fn add_nanos(&self, nanos: i128) -> Result<Self, XError> {
        if self.kind == DateKind::Time {
            let time = (self.local_nanos() + nanos).rem_euclid(NANOS_PER_DAY);
            return self
                .at_local_nanos(i128::from(days_from_civil(1972, 12, 31)) * NANOS_PER_DAY + time);
        }
        self.at_local_nanos(self.local_nanos() + nanos)
    }

    /// Adds a year-month duration, keeping the day within the new month.
    fn add_months(&self, months: i64) -> Result<Self, XError> {
        let total = (self.year * 12 + i64::from(self.month) - 1)
            .checked_add(months)
            .ok_or_else(overflow)?;
        let year = total.div_euclid(12);
        if year.abs() > MAX_YEAR {
            return Err(overflow());
        }
        let month = total.rem_euclid(12) as u8 + 1;
        Ok(Self {
            year,
            month,
            day: self.day.min(days_in_month(year, month)),
            ..*self
        })
    }

    /// Casts to another date/time type; `None` where XPath doesn't allow
    /// the cast.
    pub(crate) fn convert(&self, kind: DateKind) -> Option<Self> {
        let allowed = match self.kind {
            DateKind::DateTime => true,
            DateKind::Date => kind != DateKind::Time,
            other => other == kind,
        };
        allowed.then(|| {
            Self::new(
                kind,
                self.year,
                self.month,
                self.day,
                self.hour,
                self.minute,
                self.second,
                self.nanos,
                self.timezone,
            )
        })
    }

    /// The same value with another timezone (or none), as the
    /// `adjust-*-to-timezone` functions define it.
    pub(crate) fn adjust_timezone(&self, timezone: Option<i16>) -> Result<Self, XError> {
        let (Some(from), Some(to)) = (self.timezone, timezone) else {
            return Ok(Self { timezone, ..*self });
        };
        let shifted = Self { timezone, ..*self };
        shifted.add_nanos(i128::from(to - from) * NANOS_PER_MINUTE)
    }

    /// `fn:dateTime()`: a date and a time combined.
    pub(crate) fn combine(date: &Self, time: &Self) -> Result<Self, XError> {
        let timezone = match (date.timezone, time.timezone) {
            (Some(a), Some(b)) if a != b => {
                return Err(XError::new(
                    "FORG0008",
                    "the date and time have different timezones",
                ));
            }
            (a, b) => a.or(b),
        };
        Ok(Self::new(
            DateKind::DateTime,
            date.year,
            date.month,
            date.day,
            time.hour,
            time.minute,
            time.second,
            time.nanos,
            timezone,
        ))
    }

    /// The current date and time in UTC.
    pub(crate) fn now() -> Self {
        let since_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let value = Self::new(DateKind::DateTime, 1970, 1, 1, 0, 0, 0, 0, Some(0));
        value
            .at_local_nanos(since_epoch.as_nanos() as i128)
            .unwrap_or(value)
    }

    /// Seconds with their fraction, as `seconds-from-*()` return them.
    pub(crate) fn seconds(&self) -> f64 {
        f64::from(self.second) + f64::from(self.nanos) / 1e9
    }

    pub(crate) fn timezone_duration(&self) -> Option<Duration> {
        self.timezone.map(|minutes| Duration {
            kind: DurationKind::DayTime,
            months: 0,
            nanos: i128::from(minutes) * NANOS_PER_MINUTE,
        })
    }
}

/// A timezone given as a duration, for the `adjust-*-to-timezone`
/// functions: a whole number of minutes, at most 14 hours from UTC.
pub(crate) fn timezone_from_duration(duration: &Duration) -> Result<i16, XError> {
    if duration.nanos % NANOS_PER_MINUTE != 0
        || duration.nanos.abs() > i128::from(MAX_TIMEZONE_MINUTES) * NANOS_PER_MINUTE
    {
        return Err(XError::new(
            "FODT0003",
            format!("invalid timezone {duration}"),
        ));
    }
    Ok((duration.nanos / NANOS_PER_MINUTE) as i16)
}

fn write_fraction(f: &mut fmt::Formatter<'_>, nanos: u32) -> fmt::Result {
    if nanos > 0 {
        let digits = format!("{nanos:09}");
        write!(f, ".{}", digits.trim_end_matches('0'))?;
    }
    Ok(())
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let year = |f: &mut fmt::Formatter<'_>| {
            if self.year < 0 {
                write!(f, "-{:04}", -self.year)
            } else {
                write!(f, "{:04}", self.year)
            }
        };
        let time = |f: &mut fmt::Formatter<'_>| {
            write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)?;
            write_fraction(f, self.nanos)
        };
        match self.kind {
            DateKind::DateTime => {
                year(f)?;
                write!(f, "-{:02}-{:02}T", self.month, self.day)?;
                time(f)?;
            }
            DateKind::Date => {
                year(f)?;
                write!(f, "-{:02}-{:02}", self.month, self.day)?;
            }
            DateKind::Time => time(f)?,
            DateKind::GYearMonth => {
                year(f)?;
                write!(f, "-{:02}", self.month)?;
            }
            DateKind::GYear => year(f)?,
            DateKind::GMonthDay => write!(f, "--{:02}-{:02}", self.month, self.day)?,
            DateKind::GDay => write!(f, "---{:02}", self.day)?,
            DateKind::GMonth => write!(f, "--{:02}", self.month)?,
        }
        match self.timezone {
            None => Ok(()),
            Some(0) => f.write_str("Z"),
            Some(minutes) => write!(
                f,
                "{}{:02}:{:02}",
                if minutes < 0 { '-' } else { '+' },
                minutes.abs() / 60,
                minutes.abs() % 60
            ),
        }
    }
}

impl Duration {
    /// Parses the lexical form of `kind` (surrounding whitespace allowed).
    pub(crate) fn parse(kind: DurationKind, text: &str) -> Option<Self> {
        let text = text.trim_matches([' ', '\t', '\n', '\r']);
        let mut lexer = Lexer {
            bytes: text.as_bytes(),
            at: 0,
        };
        let negative = lexer.eat(b'-');
        if !lexer.eat(b'P') {
            return None;
        }
        let number = |run: &[u8]| -> Option<i128> {
            run.iter().try_fold(0i128, |value, b| {
                value.checked_mul(10)?.checked_add(i128::from(b - b'0'))
            })
        };
        let (mut months, mut nanos) = (0i128, 0i128);
        let mut components = 0;
        // Designators in the order they may appear, with the number of
        // months or nanoseconds each unit is.
        let date_units: [(u8, i128, bool); 3] = [
            (b'Y', 12, true),
            (b'M', 1, true),
            (b'D', NANOS_PER_DAY, false),
        ];
        let mut next_unit = 0;
        while lexer.peek().is_some_and(|b| b.is_ascii_digit()) {
            let value = number(lexer.digit_run()?)?;
            let designator = lexer.peek()?;
            lexer.at += 1;
            let offset = date_units[next_unit..]
                .iter()
                .position(|(unit, ..)| *unit == designator)?;
            let (_, size, in_months) = date_units[next_unit + offset];
            next_unit += offset + 1;
            if in_months {
                months = months.checked_add(value.checked_mul(size)?)?;
            } else {
                nanos = nanos.checked_add(value.checked_mul(size)?)?;
            }
            components += 1;
            let has_days = designator == b'D';
            if (kind == DurationKind::DayTime && !has_days)
                || (kind == DurationKind::YearMonth && has_days)
            {
                return None;
            }
        }
        if lexer.eat(b'T') {
            if kind == DurationKind::YearMonth {
                return None;
            }
            let time_units = [
                (b'H', NANOS_PER_HOUR),
                (b'M', NANOS_PER_MINUTE),
                (b'S', NANOS_PER_SECOND),
            ];
            let mut next_unit = 0;
            let mut time_components = 0;
            while lexer.peek().is_some_and(|b| b.is_ascii_digit()) {
                let value = number(lexer.digit_run()?)?;
                let mut fraction = 0;
                let mut has_fraction = false;
                if lexer.eat(b'.') {
                    has_fraction = true;
                    for (index, b) in lexer.digit_run()?.iter().take(9).enumerate() {
                        fraction += i128::from(b - b'0') * 10i128.pow(8 - index as u32);
                    }
                }
                let designator = lexer.peek()?;
                lexer.at += 1;
                let offset = time_units[next_unit..]
                    .iter()
                    .position(|(unit, _)| *unit == designator)?;
                let (unit, size) = time_units[next_unit + offset];
                next_unit += offset + 1;
                if has_fraction && unit != b'S' {
                    return None;
                }
                nanos = nanos
                    .checked_add(value.checked_mul(size)?)?
                    .checked_add(fraction)?;
                time_components += 1;
            }
            if time_components == 0 {
                return None;
            }
            components += time_components;
        }
        if components == 0 || !lexer.done() {
            return None;
        }
        let months = i64::try_from(months).ok()?;
        // Keep nanoseconds well inside i128 for arithmetic.
        if nanos > i128::from(i64::MAX) * NANOS_PER_SECOND {
            return None;
        }
        Some(if negative {
            Self {
                kind,
                months: -months,
                nanos: -nanos,
            }
        } else {
            Self {
                kind,
                months,
                nanos,
            }
        })
    }

    fn day_time(nanos: i128) -> Self {
        Self {
            kind: DurationKind::DayTime,
            months: 0,
            nanos,
        }
    }

    fn year_month(months: i64) -> Self {
        Self {
            kind: DurationKind::YearMonth,
            months,
            nanos: 0,
        }
    }

    /// Casts to another duration type, dropping the components it lacks.
    pub(crate) fn convert(&self, kind: DurationKind) -> Self {
        match kind {
            DurationKind::Duration => Self { kind, ..*self },
            DurationKind::DayTime => Self::day_time(self.nanos),
            DurationKind::YearMonth => Self::year_month(self.months),
        }
    }

    /// The components `years-from-duration()` and friends return.
    pub(crate) fn years(&self) -> i64 {
        self.months / 12
    }

    pub(crate) fn months_part(&self) -> i64 {
        self.months % 12
    }

    pub(crate) fn days(&self) -> i64 {
        (self.nanos / NANOS_PER_DAY) as i64
    }

    pub(crate) fn hours(&self) -> i64 {
        (self.nanos % NANOS_PER_DAY / NANOS_PER_HOUR) as i64
    }

    pub(crate) fn minutes(&self) -> i64 {
        (self.nanos % NANOS_PER_HOUR / NANOS_PER_MINUTE) as i64
    }

    pub(crate) fn seconds(&self) -> f64 {
        (self.nanos % NANOS_PER_MINUTE) as f64 / 1e9
    }

    /// The order of two durations of the same ordered subtype; for other
    /// pairs, only equality is meaningful.
    pub(crate) fn compare(&self, other: &Self) -> Ordering {
        (self.months, self.nanos).cmp(&(other.months, other.nanos))
    }

    /// Whether `lt`, `gt` and friends are defined between the two.
    pub(crate) fn ordered_with(&self, other: &Self) -> bool {
        self.kind == other.kind && self.kind != DurationKind::Duration
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.months == 0 && self.nanos == 0 {
            return f.write_str(if self.kind == DurationKind::YearMonth {
                "P0M"
            } else {
                "PT0S"
            });
        }
        if self.months < 0 || self.nanos < 0 {
            f.write_str("-")?;
        }
        f.write_str("P")?;
        let months = self.months.unsigned_abs();
        if months >= 12 {
            write!(f, "{}Y", months / 12)?;
        }
        if !months.is_multiple_of(12) {
            write!(f, "{}M", months % 12)?;
        }
        let nanos = self.nanos.unsigned_abs();
        let unsigned = |nanos: i128| nanos as u128;
        let days = nanos / unsigned(NANOS_PER_DAY);
        let hours = nanos % unsigned(NANOS_PER_DAY) / unsigned(NANOS_PER_HOUR);
        let minutes = nanos % unsigned(NANOS_PER_HOUR) / unsigned(NANOS_PER_MINUTE);
        let seconds = nanos % unsigned(NANOS_PER_MINUTE) / unsigned(NANOS_PER_SECOND);
        let fraction = (nanos % unsigned(NANOS_PER_SECOND)) as u32;
        if days > 0 {
            write!(f, "{days}D")?;
        }
        if hours > 0 || minutes > 0 || seconds > 0 || fraction > 0 {
            f.write_str("T")?;
            if hours > 0 {
                write!(f, "{hours}H")?;
            }
            if minutes > 0 {
                write!(f, "{minutes}M")?;
            }
            if seconds > 0 || fraction > 0 {
                write!(f, "{seconds}")?;
                write_fraction(f, fraction)?;
                f.write_str("S")?;
            }
        }
        Ok(())
    }
}

fn type_error(op: ArithmeticOp, a: &Atomic, b: &Atomic) -> XError {
    let operator = match op {
        ArithmeticOp::Add => "+",
        ArithmeticOp::Subtract => "-",
        ArithmeticOp::Multiply => "*",
        ArithmeticOp::Divide => "div",
        ArithmeticOp::IntegerDivide => "idiv",
        ArithmeticOp::Modulo => "mod",
    };
    XError::new(
        "XPTY0004",
        format!(
            "`{operator}` is not defined for `{}` and `{}`",
            a.to_string_value(),
            b.to_string_value()
        ),
    )
}

/// XPath `round()`, for durations multiplied or divided by a number.
fn round_half_up(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// A duration scaled by a number (`*`, or `div` with the reciprocal
/// applied by the caller).
fn scale(duration: &Duration, factor: f64, divide: bool) -> Result<Duration, XError> {
    if factor.is_nan() {
        return Err(XError::new(
            "FOCA0005",
            "a duration multiplied or divided by NaN",
        ));
    }
    let apply = |value: f64| {
        if divide {
            value / factor
        } else {
            value * factor
        }
    };
    match duration.kind {
        DurationKind::YearMonth => {
            let months = round_half_up(apply(duration.months as f64));
            if !months.is_finite() || months.abs() >= i64::MAX as f64 {
                return Err(duration_overflow());
            }
            Ok(Duration::year_month(months as i64))
        }
        _ => {
            let nanos = round_half_up(apply(duration.nanos as f64));
            if !nanos.is_finite() || nanos.abs() >= (i64::MAX as f64) * 1e9 {
                return Err(duration_overflow());
            }
            Ok(Duration::day_time(nanos as i128))
        }
    }
}

/// Arithmetic with a date, time or duration operand (after untyped
/// operands have become doubles).
pub(crate) fn arithmetic(op: ArithmeticOp, a: &Atomic, b: &Atomic) -> Result<Atomic, XError> {
    use ArithmeticOp::*;
    use DurationKind::{DayTime, YearMonth};
    let error = || type_error(op, a, b);
    match (a, b) {
        (Atomic::DateTime(x), Atomic::DateTime(y)) => {
            if op != Subtract || x.kind != y.kind || !x.kind.is_ordered() {
                return Err(error());
            }
            Ok(Atomic::Duration(Duration::day_time(
                x.instant() - y.instant(),
            )))
        }
        (Atomic::DateTime(date), Atomic::Duration(duration))
        | (Atomic::Duration(duration), Atomic::DateTime(date)) => {
            let duration_first = matches!(a, Atomic::Duration(_));
            if !matches!(op, Add | Subtract) || (duration_first && op == Subtract) {
                return Err(error());
            }
            let sign = if op == Subtract { -1 } else { 1 };
            let result = match (date.kind, duration.kind) {
                (DateKind::DateTime | DateKind::Date, YearMonth) => {
                    date.add_months(sign * duration.months)?
                }
                (DateKind::DateTime | DateKind::Date | DateKind::Time, DayTime) => {
                    date.add_nanos(i128::from(sign) * duration.nanos)?
                }
                _ => return Err(error()),
            };
            Ok(Atomic::DateTime(result))
        }
        (Atomic::Duration(x), Atomic::Duration(y)) => {
            if x.kind != y.kind || x.kind == DurationKind::Duration {
                return Err(error());
            }
            match op {
                Add | Subtract => {
                    let sign = if op == Subtract { -1 } else { 1 };
                    let months = x
                        .months
                        .checked_add(sign * y.months)
                        .ok_or_else(duration_overflow)?;
                    Ok(Atomic::Duration(Duration {
                        kind: x.kind,
                        months,
                        nanos: x.nanos + i128::from(sign) * y.nanos,
                    }))
                }
                Divide => {
                    let (dividend, divisor) = if x.kind == YearMonth {
                        (x.months as f64, y.months as f64)
                    } else {
                        (x.nanos as f64, y.nanos as f64)
                    };
                    if divisor == 0.0 {
                        return Err(XError::new("FOAR0001", "division by a zero duration"));
                    }
                    Ok(Atomic::Dec(dividend / divisor))
                }
                _ => Err(error()),
            }
        }
        (Atomic::Duration(duration), number) | (number, Atomic::Duration(duration))
            if number.is_numeric() =>
        {
            let duration_first = matches!(a, Atomic::Duration(_));
            if duration.kind == DurationKind::Duration {
                return Err(error());
            }
            let factor = number.as_f64().unwrap_or(f64::NAN);
            match op {
                Multiply => scale(duration, factor, false).map(Atomic::Duration),
                Divide if duration_first => scale(duration, factor, true).map(Atomic::Duration),
                _ => Err(error()),
            }
        }
        _ => Err(error()),
    }
}
