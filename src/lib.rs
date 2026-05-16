use chrono::{Datelike, NaiveDate};
use glam::Vec2;

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Right,
    Up,
    Left,
    Down,
}

pub struct GridPoint {
    pub value: u32,
    pub pos: Vec2,
    pub angle: f32,
}

pub struct NaturalSquaresEngine {
    pub epoch_date: NaiveDate,
}

impl NaturalSquaresEngine {
    pub fn new(epoch: NaiveDate) -> Self {
        Self { epoch_date: epoch }
    }

    // =====================================================
    // PRICE -> ANGLE
    // =====================================================

    pub fn calculate_vibration_angle(price: f64) -> f32 {
        if price <= 0.0 {
            return 0.0;
        }

        ((price.sqrt() % 2.0) * 180.0) as f32
    }

    // =====================================================
    // COORD -> ANGLE
    //
    // EAST = 0°
    // NORTH = 90°
    // WEST = 180°
    // SOUTH = 270°
    // =====================================================

    pub fn coord_to_angle(x: i32, y: i32) -> f32 {
        let angle = (y as f32).atan2(x as f32).to_degrees();

        if angle < 0.0 { angle + 360.0 } else { angle }
    }

    // =====================================================
    // DATE -> ANGLE
    // =====================================================

    pub fn date_to_angle(&self, current_date: NaiveDate, clockwise: bool) -> f32 {
        let days = current_date.ordinal() as f32;

        let total_days = 365.2422;

        let angle = (days / total_days) * 360.0;

        if clockwise { angle } else { 360.0 - angle }
    }

    // =====================================================
    // 24 HOUR SYSTEM
    //
    // 1 hour = 15°
    // =====================================================

    pub fn hour_to_angle(hour: u32) -> f32 {
        ((hour - 1) as f32) * 15.0
    }

    // =====================================================
    // ZODIAC CALCULATOR (13-Sign Astronomical System)
    //
    // Converts the true structural Right Ascension (RA)
    // boundary hours into geometry based on your system's
    // baseline math (1 hour = 15°).
    // Aries is hard-locked at index 0 to 0.0° (East).
    // =====================================================
    pub fn zodiac_to_angle(index: u32) -> f32 {
        // High-precision astronomical Right Ascension boundaries (in decimal hours)
        let ra_hour = match index {
            0 => 0.0,      // ♈ Aries (Vernal Equinox baseline anchor)
            1 => 1.7513,   // ♉ Taurus
            2 => 4.2611,   // ♊ Gemini
            3 => 6.2231,   // ♋ Cancer
            4 => 7.6106,   // ♌ Leo
            5 => 10.0411,  // ♍ Virgo
            6 => 12.9664,  // ♎ Libra
            7 => 14.3533,  // ♏ Scorpio
            8 => 14.9061,  // ⛎ Ophiuchus
            9 => 16.1147,  // ♐ Sagittarius
            10 => 18.3231, // ♑ Capricorn
            11 => 20.1253, // ♒ Aquarius
            12 => 21.6947, // ♓ Pisces
            _ => 0.0,
        };

        // Convert the RA hours value directly to geometric degrees (15° per hour)
        let angle = ra_hour * 15.0;

        angle as f32
    }

    // =====================================================
    // YEAR RING
    // =====================================================

    // where Jan 1 starts on the circle
    pub const YEAR_START_DEG: f32 = 282.0;

    pub fn day_of_year_to_angle(day: f32, clockwise: bool) -> f32 {
        let total_days = 365.2422;

        let mut angle = (day / total_days) * 360.0;

        // master offset
        angle += Self::YEAR_START_DEG;

        // wrap
        angle %= 360.0;

        if !clockwise {
            angle = 360.0 - angle;
        }

        angle
    }

    pub fn date_to_year_angle(date: NaiveDate, clockwise: bool) -> f32 {
        let day = (date.ordinal() - 1) as f32;

        Self::day_of_year_to_angle(day, clockwise)
    }

    // OPTIONAL: equinox alignment helper
    pub fn equinox_offset(&self, offset_days: f32) -> f32 {
        (offset_days / 365.2422) * 360.0
    }

    /// Calculates Local Sidereal Time (LST) in degrees [0, 360)
    /// for a given UTC timestamp and geographic longitude.
    /// Longitude: East is positive, West is negative (e.g., 25.45 for UTC+2 regions).
    pub fn calculate_local_sidereal_time_deg(
        utc_time: chrono::DateTime<chrono::Utc>,
        longitude_deg: f64,
    ) -> f64 {
        use chrono::Datelike;
        use chrono::Timelike;

        // 1. Convert to Julian Date (JD) at 0h UTC for the current day
        let year = utc_time.year() as f64;
        let month = utc_time.month() as f64;

        let (y, m) = if month <= 2.0 {
            (year - 1.0, month + 12.0)
        } else {
            (year, month)
        };

        let a = (y / 100.0).floor();
        let b = 2.0 - a + (a / 4.0).floor();

        let jd_0h = (365.25 * (y + 4716.0)).floor()
            + (30.6001 * (m + 1.0)).floor()
            + day_of_year_to_angle_helper_jd_day(b);

        fn day_of_year_to_angle_helper_jd_day(b: f64) -> f64 {
            b - 1524.5
        }

        // 2. Calculate time centuries since J2000.0
        let t = (jd_0h - 2451545.0) / 36525.0;

        // 3. IAU 1982 formula for Greenwich Mean Sidereal Time (GMST) at 0h UTC (in seconds)
        let gmst_0h =
            24110.54841 + (8640184.812866 * t) + (0.093104 * t * t) - (6.2e-6 * t * t * t);

        // Convert GMST seconds to degrees (15 degrees per hour, 24 hours = 86400 seconds)
        let mut gmst_0h_deg = (gmst_0h / 86400.0 * 360.0) % 360.0;
        if gmst_0h_deg < 0.0 {
            gmst_0h_deg += 360.0;
        }

        // 4. Add the elapsed universal time of the current day with the sidereal rate multiplier (1.00273790935)
        let utc_seconds = (utc_time.hour() as f64 * 3600.0)
            + (utc_time.minute() as f64 * 60.0)
            + (utc_time.second() as f64)
            + (utc_time.nanosecond() as f64 / 1_000_000_000.0);

        let elapsed_deg = utc_seconds * (15.0 / 3600.0) * 1.00273790935;

        // 5. Compute Local Sidereal Time by incorporating your longitude
        let mut lst_deg = gmst_0h_deg + elapsed_deg + longitude_deg;
        lst_deg %= 360.0;
        if lst_deg < 0.0 {
            lst_deg += 360.0;
        }

        lst_deg
    }

    pub fn is_daylight(
        utc: chrono::DateTime<chrono::Utc>,
        longitude_deg: f64,
        latitude_deg: f64,
    ) -> bool {
        use chrono::Timelike;

        // Local sidereal time
        let lst = Self::calculate_local_sidereal_time_deg(utc, longitude_deg);

        // Convert to hour angle (simplified solar model)
        let hour_angle = (lst % 360.0) - 180.0;

        // Solar declination approximation (good enough for visual system)
        let day_of_year = utc.ordinal() as f64;
        let decl = (23.44f64.to_radians()
            * (360.0 / 365.2422 * (day_of_year - 81.0)).to_radians().sin())
        .to_degrees();

        // Convert latitude + declination into horizon threshold
        let lat = latitude_deg;

        let cos_h0 = -lat.to_radians().sin() * decl.to_radians().sin()
            / (lat.to_radians().cos() * decl.to_radians().cos());

        // clamp safety
        let cos_h0 = cos_h0.clamp(-1.0, 1.0);

        let sunset_hour_angle = cos_h0.acos().to_degrees();

        // daylight if sun is within horizon arc
        hour_angle.abs() < sunset_hour_angle
    }
}

pub struct SpiralIterator {
    current_value: u32,
    x: i32,
    y: i32,
    steps_in_current_dir: i32,
    dir: Direction,
    segment_length: i32,
    is_second_segment: bool,
}

impl Default for SpiralIterator {
    fn default() -> Self {
        Self {
            current_value: 1,

            x: 0,
            y: 0,

            // EAST START
            dir: Direction::Right,

            steps_in_current_dir: 0,

            segment_length: 1,

            is_second_segment: false,
        }
    }
}

impl Iterator for SpiralIterator {
    type Item = (u32, i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        let res = (self.current_value, self.x, self.y);

        match self.dir {
            Direction::Right => self.x += 1,
            Direction::Up => self.y += 1,
            Direction::Left => self.x -= 1,
            Direction::Down => self.y -= 1,
        }

        self.steps_in_current_dir += 1;

        if self.steps_in_current_dir == self.segment_length {
            self.steps_in_current_dir = 0;

            // COUNTER CLOCKWISE
            self.dir = match self.dir {
                Direction::Right => Direction::Up,
                Direction::Up => Direction::Left,
                Direction::Left => Direction::Down,
                Direction::Down => Direction::Right,
            };

            if self.is_second_segment {
                self.segment_length += 1;
                self.is_second_segment = false;
            } else {
                self.is_second_segment = true;
            }
        }

        self.current_value += 1;

        Some(res)
    }
}
