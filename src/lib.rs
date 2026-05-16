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
    // ZODIAC
    //
    // 1 sign = 30°
    // =====================================================

    pub fn zodiac_to_angle(index: u32) -> f32 {
        (index as f32) * 30.0
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
