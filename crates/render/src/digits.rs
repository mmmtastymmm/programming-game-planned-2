//! The seven-segment number on a bot's top face (`docs/07`, Machines on
//! the map; Q46): its entity id's last three digits, lit in its team's
//! color, unlit segments dark. Plain Rust, so it is tested headless; the
//! view spawns the segments and turns the display to face the camera.

/// Digits on the display; an id past them shows its last three.
pub const DIGITS: usize = 3;

/// The segments `a`–`g` in the usual order: top, upper right, lower right,
/// bottom, lower left, upper left, middle.
pub type Segments = [bool; 7];

/// Which segments a digit lights.
pub fn segments_of(d: u32) -> Segments {
    let bits: u8 = match d {
        0 => 0b011_1111,
        1 => 0b000_0110,
        2 => 0b101_1011,
        3 => 0b100_1111,
        4 => 0b110_0110,
        5 => 0b110_1101,
        6 => 0b111_1101,
        7 => 0b000_0111,
        8 => 0b111_1111,
        _ => 0b110_1111,
    };
    std::array::from_fn(|i| bits >> i & 1 == 1)
}

/// The display for an entity id: its last three digits, right-aligned,
/// leading places blank (`None`).
pub fn display(id: u32) -> [Option<u32>; DIGITS] {
    let n = id % 1000;
    let len = if n >= 100 {
        3
    } else if n >= 10 {
        2
    } else {
        1
    };
    std::array::from_fn(|i| {
        let place = DIGITS - 1 - i;
        (place < len).then(|| n / 10u32.pow(place as u32) % 10)
    })
}

/// A segment's center and size in a digit `w` wide and `h` tall with
/// segments `t` thick, origin at the digit's center, `+y` up: `(x, y, w, h)`.
pub fn segment_rect(i: usize, w: f32, h: f32, t: f32) -> (f32, f32, f32, f32) {
    let (hx, hy) = (w / 2.0 - t / 2.0, h / 2.0 - t / 2.0);
    let across = (w - 2.0 * t, t);
    let down = (t, h / 2.0 - t);
    let (x, y, (sw, sh)) = match i {
        0 => (0.0, hy, across),
        1 => (hx, hy / 2.0, down),
        2 => (hx, -hy / 2.0, down),
        3 => (0.0, -hy, across),
        4 => (-hx, -hy / 2.0, down),
        5 => (-hx, hy / 2.0, down),
        _ => (0.0, 0.0, across),
    };
    (x, y, sw, sh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_digit_lights_its_segments() {
        let count = |d| segments_of(d).iter().filter(|s| **s).count();
        let counts: Vec<usize> = (0..10).map(count).collect();
        assert_eq!(counts, [6, 2, 5, 5, 4, 5, 6, 3, 7, 6]);
        assert_eq!(
            segments_of(1),
            [false, true, true, false, false, false, false]
        );
        assert!(!segments_of(0)[6], "zero has no middle bar");
        assert!(segments_of(8).iter().all(|s| *s));
    }

    #[test]
    fn the_display_shows_the_last_three_digits_right_aligned() {
        assert_eq!(display(7), [None, None, Some(7)]);
        assert_eq!(display(17), [None, Some(1), Some(7)]);
        assert_eq!(display(305), [Some(3), Some(0), Some(5)]);
        assert_eq!(display(1234), [Some(2), Some(3), Some(4)]);
        assert_eq!(
            display(1007),
            [None, None, Some(7)],
            "the last three of 1007"
        );
        assert_eq!(display(0), [None, None, Some(0)]);
    }

    #[test]
    fn segments_fill_the_digit_without_overlapping() {
        let (w, h, t) = (11.0, 18.0, 2.5);
        let rects: Vec<_> = (0..7).map(|i| segment_rect(i, w, h, t)).collect();
        for (x, y, sw, sh) in &rects {
            assert!(x.abs() + sw / 2.0 <= w / 2.0 + 1e-4 && y.abs() + sh / 2.0 <= h / 2.0 + 1e-4);
        }
        for (i, a) in rects.iter().enumerate() {
            for b in &rects[i + 1..] {
                let apart = (a.0 - b.0).abs() >= (a.2 + b.2) / 2.0 - 1e-4
                    || (a.1 - b.1).abs() >= (a.3 + b.3) / 2.0 - 1e-4;
                assert!(apart, "{a:?} overlaps {b:?}");
            }
        }
    }
}
