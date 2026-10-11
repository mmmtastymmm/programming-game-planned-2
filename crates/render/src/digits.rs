//! The seven-segment number on a bot's top face (`docs/07`, Machines on
//! the map; Q46): its entity id's last three digits, lit in its team's
//! color. The segments are fixed to the lid, a lattice that holds the
//! number in any of the four quarter turns, and only which of them are lit
//! changes, so the number reads upright from the camera however the bot
//! faces. Plain Rust, so it is tested headless; the view spawns the
//! segments and lights them.
//!
//! The lattice is in cells, `x` to the bot's right and `y` to its front.
//! The number fills a 5-by-2 band — three digits a cell wide and two tall,
//! a blank column between them — and turns about [`CENTER`], a half-cell
//! point, so every turn lands on the same integer lattice.

/// Digits on the display; an id past them shows its last three.
pub const DIGITS: usize = 3;

/// Where the band turns, in cells: half a cell above the band's middle, so
/// the number sits that much toward the reader's side in every turn.
pub const CENTER: (f32, f32) = (2.5, 1.5);

/// A lattice edge: its two ends, in cells, the lesser first.
pub type Edge = ((i32, i32), (i32, i32));

/// The segments `a`–`g` in the usual order — top, upper right, lower
/// right, bottom, lower left, upper left, middle — as edges of a digit's
/// one-by-two cell, origin at its lower left.
const SEGMENTS: [Edge; 7] = [
    ((0, 2), (1, 2)),
    ((1, 1), (1, 2)),
    ((1, 0), (1, 1)),
    ((0, 0), (1, 0)),
    ((0, 0), (0, 1)),
    ((0, 1), (0, 2)),
    ((0, 1), (1, 1)),
];

/// Which segments a digit lights.
pub fn segments_of(d: u32) -> [bool; 7] {
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

/// `p` turned `quarter` quarter turns anticlockwise (seen from above)
/// about [`CENTER`]. Doubled coordinates keep the half-cell center exact.
fn turn(p: (i32, i32), quarter: u8) -> (i32, i32) {
    let (cx, cy) = (5, 3);
    let (mut x, mut y) = (2 * p.0 - cx, 2 * p.1 - cy);
    for _ in 0..quarter % 4 {
        (x, y) = (-y, x);
    }
    ((x + cx) / 2, (y + cy) / 2)
}

fn edge(a: (i32, i32), b: (i32, i32)) -> Edge {
    if a <= b { (a, b) } else { (b, a) }
}

/// Segment `s` of the digit in place `place` (0 leftmost), turned.
fn placed(place: usize, s: usize, quarter: u8) -> Edge {
    let (a, b) = SEGMENTS[s];
    let dx = 2 * place as i32;
    edge(
        turn((a.0 + dx, a.1), quarter),
        turn((b.0 + dx, b.1), quarter),
    )
}

/// Every segment on the lid: each place's seven, in every turn, once.
pub fn lattice() -> Vec<Edge> {
    let mut out: Vec<Edge> = (0..4)
        .flat_map(|q| (0..DIGITS).flat_map(move |p| (0..7).map(move |s| placed(p, s, q))))
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Which of `lattice`'s segments spell `id` turned `quarter` quarter
/// turns: upright to a reader who looks at the bot from that side.
pub fn lit(lattice: &[Edge], id: u32, quarter: u8) -> Vec<bool> {
    let mut on = vec![false; lattice.len()];
    for (place, digit) in display(id).into_iter().enumerate() {
        let Some(d) = digit else { continue };
        for (s, lit_here) in segments_of(d).into_iter().enumerate() {
            if lit_here && let Ok(i) = lattice.binary_search(&placed(place, s, quarter)) {
                on[i] = true;
            }
        }
    }
    on
}

/// The quarter turn that reads upright, from the angle between the
/// reader's up and the bot's front, anticlockwise seen from above.
pub fn quarter_for(angle: f32) -> u8 {
    let q = (angle / std::f32::consts::FRAC_PI_2).round() as i32;
    q.rem_euclid(4) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_digit_lights_its_segments() {
        let count = |d| segments_of(d).iter().filter(|s| **s).count();
        let counts: Vec<usize> = (0..10).map(count).collect();
        assert_eq!(counts, [6, 2, 5, 5, 4, 5, 6, 3, 7, 6]);
        assert!(!segments_of(0)[6], "zero has no middle bar");
    }

    #[test]
    fn the_display_shows_the_last_three_digits_right_aligned() {
        assert_eq!(display(7), [None, None, Some(7)]);
        assert_eq!(display(17), [None, Some(1), Some(7)]);
        assert_eq!(display(305), [Some(3), Some(0), Some(5)]);
        assert_eq!(display(1234), [Some(2), Some(3), Some(4)]);
        assert_eq!(display(1007), [None, None, Some(7)]);
        assert_eq!(display(0), [None, None, Some(0)]);
    }

    #[test]
    fn the_lattice_is_fixed_and_holds_every_turn() {
        let l = lattice();
        assert_eq!(l.len(), 52);
        // Turning the lattice a quarter gives the same lattice: the lid
        // looks the same from every side.
        let mut turned: Vec<Edge> = l
            .iter()
            .map(|(a, b)| edge(turn(*a, 1), turn(*b, 1)))
            .collect();
        turned.sort_unstable();
        assert_eq!(turned, l);
        for e in &l {
            let len = (e.1.0 - e.0.0).abs() + (e.1.1 - e.0.1).abs();
            assert_eq!(len, 1, "{e:?} is one cell long");
        }
    }

    #[test]
    fn every_turn_lights_the_whole_number_on_the_lattice() {
        let l = lattice();
        for id in [8, 17, 305, 888, 1234] {
            let want: usize = display(id)
                .into_iter()
                .flatten()
                .map(|d| segments_of(d).iter().filter(|s| **s).count())
                .sum();
            let mut seen = Vec::new();
            for q in 0..4 {
                let on = lit(&l, id, q);
                assert_eq!(on.iter().filter(|b| **b).count(), want, "{id} turn {q}");
                seen.push(on);
            }
            // A quarter's lit set is the last quarter's turned.
            for q in 0..4u8 {
                let mut turned: Vec<Edge> = l
                    .iter()
                    .zip(&seen[q as usize])
                    .filter(|(_, on)| **on)
                    .map(|(e, _)| edge(turn(e.0, 1), turn(e.1, 1)))
                    .collect();
                turned.sort_unstable();
                let next: Vec<Edge> = l
                    .iter()
                    .zip(&seen[((q + 1) % 4) as usize])
                    .filter(|(_, on)| **on)
                    .map(|(e, _)| *e)
                    .collect();
                assert_eq!(turned, next, "{id} turn {q} to {}", q + 1);
            }
        }
    }

    #[test]
    fn the_nearest_quarter_reads_upright() {
        use std::f32::consts::FRAC_PI_2;
        assert_eq!(quarter_for(0.0), 0);
        assert_eq!(quarter_for(0.7), 0, "just under halfway stays");
        assert_eq!(quarter_for(0.9), 1);
        assert_eq!(quarter_for(2.0 * FRAC_PI_2), 2);
        assert_eq!(quarter_for(-FRAC_PI_2), 3);
        assert_eq!(quarter_for(5.0 * FRAC_PI_2), 1);
    }
}
