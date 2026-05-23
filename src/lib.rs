use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc};
use glam::Vec2;
use swiss_eph::safe::{CalcFlags, Planet, calc, julday};

#[derive(Debug, Clone)]
pub struct PlanetPosition {
    pub name: &'static str,
    pub angle: f64,
    pub distance: f64,
    pub speed: f64,
}

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
pub struct MoonState {
    /// The absolute geometric angle (0 to 360) where 0 is the Vernal Equinox (Aries 0° East)
    pub zodiac_angle: f64,
    /// Moon phase illumination percentage (0.0 = New Moon, 1.0 = Full Moon)
    pub phase: f64,
}

impl NaturalSquaresEngine {
    pub fn calculate_planetary_positions(utc: DateTime<Utc>) -> Vec<PlanetPosition> {
        let local_tz = utc + Duration::hours(2);

        let hour_fraction = local_tz.hour() as f64 / 24.0
            + local_tz.minute() as f64 / 1440.0
            + local_tz.second() as f64 / 86400.0;

        let jd_ut = julday(
            utc.year(),
            utc.month() as i32,
            utc.day() as i32,
            hour_fraction * 24.0,
        );

        // Natively handled by the idiomatic `calc` function
        let flags = CalcFlags::new().with_speed();

        let targets = [
            (Planet::Sun, "Sun"),
            (Planet::Moon, "Moon"),
            (Planet::Mercury, "Mercury"),
            (Planet::Venus, "Venus"),
            (Planet::Mars, "Mars"),
            (Planet::Jupiter, "Jupiter"),
            (Planet::Saturn, "Saturn"),
            (Planet::Uranus, "Uranus"),
            (Planet::Neptune, "Neptune"),
        ];

        targets
            .iter()
            .map(|(body, name)| {
                // Fixed: Using `calc` with native types directly, no manual casting or bit-extraction needed
                match calc(jd_ut, *body, flags) {
                    Ok(res) => PlanetPosition {
                        name,
                        angle: res.longitude,
                        distance: res.distance,
                        speed: res.longitude_speed,
                    },
                    Err(_) => PlanetPosition {
                        name,
                        angle: 0.0,
                        distance: 0.0,
                        speed: 0.0,
                    },
                }
            })
            .collect()
    }

    /// Returns the current day count of the year (e.g., Jan 1st = 1, May 23rd = 143)
    pub fn get_current_day_count(utc: DateTime<Utc>) -> u32 {
        // .ordinal() natively returns a 1-based day-of-year integer (1 to 366)
        utc.ordinal()
    }
    /// Computes the Moon's zodiac angle and illumination fraction using high-precision J2000 epoch baselines.
    pub fn calculate_moon_state(utc: chrono::DateTime<chrono::Utc>) -> MoonState {
        use chrono::Datelike;
        use chrono::Timelike;

        // 1. Calculate Julian Date relative to J2000.0 (January 1, 2000, 12:00 UTC)
        let year = utc.year() as f64;
        let month = utc.month() as f64;
        let day = utc.day() as f64;

        let hour_fraction =
            utc.hour() as f64 / 24.0 + utc.minute() as f64 / 1440.0 + utc.second() as f64 / 86400.0;

        // Simplified Julian Date calculation valid for 1901-2099
        let base_jd = 367.0 * year
            - ((7.0 * (year + ((month + 9.0) / 12.0).floor())) / 4.0).floor()
            + ((275.0 * month) / 9.0).floor()
            + day
            + 1721013.5;

        let jd = base_jd + hour_fraction;
        let days_since_j2000 = jd - 2451545.0;

        // 2. Calculate Mean Longitude of the Moon (Mean position along the ecliptic)
        // At J2000, the Moon was at 218.316° and progresses at roughly 13.176396° per day
        let moon_mean_long = (218.316 + 13.17639648 * days_since_j2000).rem_euclid(360.0);

        // 3. Calculate Mean Anomaly of the Moon (Its position along its own elliptical orbit)
        let moon_mean_anomaly = (134.963 + 13.06499295 * days_since_j2000).to_radians();

        // 4. Calculate Mean Anomaly of the Sun (Needed for the phase and minor pertubations)
        let sun_mean_anomaly = (357.529 + 0.98560028 * days_since_j2000).to_radians();

        // 5. Apply primary Evection and Keplerian orbit corrections to find True Geocentric Ecliptic Longitude
        // This shifts it from an ideal circle to its actual warped speed through the sky
        let correction = 6.289 * moon_mean_anomaly.sin()
            + 1.274 * (2.0 * moon_mean_long.to_radians() - moon_mean_anomaly).sin()
            - 0.658 * (2.0 * (moon_mean_long.to_radians() - sun_mean_anomaly)).sin()
            - 0.214 * (2.0 * moon_mean_anomaly).sin();

        // This is our precise 0-360 true position of the moon relative to the Vernal Equinox!
        let true_moon_longitude = (moon_mean_long + correction).rem_euclid(360.0);

        // 6. Calculate Moon Phase Illumination (Age / Elongation relative to the Sun)
        // Sun's mean longitude relative to J2000
        let sun_mean_long = (280.460 + 0.9856474 * days_since_j2000).rem_euclid(360.0);
        // Elongation is the angular distance between Moon and Sun
        let elongation = (true_moon_longitude - sun_mean_long)
            .rem_euclid(360.0)
            .to_radians();

        // Phase calculation: 0.0 (New Moon), 0.5 (Quarter), 1.0 (Full Moon)
        let phase = (1.0 - elongation.cos()) / 2.0;

        MoonState {
            zodiac_angle: true_moon_longitude,
            phase,
        }
    }
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
    // Converts the true structural ecliptic intersections
    // of the 13 IAU constellations into geometric degrees.
    // Aries 0° is locked to the modern astronomical
    // coordinate baseline. Natively counter-clockwise.
    // =====================================================
    pub fn zodiac_to_angle(index: u32) -> f32 {
        // Precise physical ecliptic boundary entry points (Degrees of Ecliptic Longitude)
        let boundary_deg = match index {
            0 => 29.0,   // ♈ Aries (Sun enters approx April 19)
            1 => 53.0,   // ♉ Taurus (Sun enters approx May 14)
            2 => 90.0,   // ♊ Gemini (Sun enters approx June 20)
            3 => 119.0,  // ♋ Cancer (Sun enters approx July 21)
            4 => 138.0,  // ♌ Leo (Sun enters approx August 10)
            5 => 174.0,  // ♍ Virgo (Sun enters approx September 16)
            6 => 217.0,  // ♎ Libra (Sun enters approx October 31)
            7 => 241.0,  // ♏ Scorpio (Sun enters approx November 23)
            8 => 248.0,  // ⛎ Ophiuchus (Sun enters exactly Nov 30 - Dec 17)
            9 => 266.0,  // ♐ Sagittarius (Sun enters approx December 18)
            10 => 299.0, // ♑ Capricorn (Sun enters approx January 19)
            11 => 327.0, // ♒ Aquarius (Sun enters approx February 16)
            12 => 351.0, // ♓ Pisces (Sun enters approx March 12)
            _ => 0.0,
        };

        boundary_deg as f32
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

        // 1. Calculate Fractional Day of Year (Jan 1 = 1.0)
        // Using a standard high-precision baseline where Day 80 is the Vernal Equinox
        let day_of_year = utc.ordinal() as f64
            + (utc.hour() as f64 / 24.0)
            + (utc.minute() as f64 / 1440.0)
            + (utc.second() as f64 / 86400.0);

        // 2. High-Precision Solar Declination Angle approximation (in Degrees)
        // Standard solar orbital position relative to the celestial equator
        let solar_noon_anomaly = (360.0 / 365.2422) * (day_of_year - 80.0);
        let declination_deg = 23.44 * solar_noon_anomaly.to_radians().sin();

        // 3. Calculate Local Apparent Solar Time (Hour Angle)
        // Find fractional UTC hours elapsed
        let utc_hours =
            utc.hour() as f64 + (utc.minute() as f64 / 60.0) + (utc.second() as f64 / 3600.0);

        // Convert geographic longitude directly to a solar time offset (15 degrees per hour)
        let local_solar_time_hours = utc_hours + (longitude_deg / 15.0);

        // Normalize to a standard 24-hour day framework
        let local_solar_time_wrapped = local_solar_time_hours.rem_euclid(24.0);

        // Calculate Solar Hour Angle (H): Noon is 0 degrees.
        // 1 hour = 15 degrees.
        // This scales perfectly with your counter-clockwise 24h UI ring.
        let hour_angle_deg = (local_solar_time_wrapped - 12.0) * 15.0;

        // 4. Determine Horizon Threshold Arc using Spherical Trigonometry
        let lat_rad = latitude_deg.to_radians();
        let decl_rad = declination_deg.to_radians();

        let cos_h0 = -lat_rad.sin() * decl_rad.sin() / (lat_rad.cos() * decl_rad.cos());

        // 5. Boundary protections for Polar Regions (24hr Night / 24hr Day)
        if cos_h0 >= 1.0 {
            // Sun never rises (Polar Night)
            return false;
        } else if cos_h0 <= -1.0 {
            // Sun never sets (Midnight Sun)
            return true;
        }

        // Calculate the sunset hour angle maximum arc limit
        let sunset_hour_angle_limit = cos_h0.acos().to_degrees();

        // 6. Return state: True if the absolute hour angle is within day limits
        hour_angle_deg.abs() <= sunset_hour_angle_limit
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
