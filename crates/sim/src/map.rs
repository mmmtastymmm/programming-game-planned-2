//! The map (`docs/03`, The map): one TOML file per match fixing the
//! rectangle, every tile's terrain, every deposit's cap and regrowth, the
//! starting speed and each team's starting tiles. A map's bytes are part
//! of a replay's identity.

use crate::data::{Data, DataError};
use lang::Num;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Integer tile coordinates, origin at the centre, `x` east and `y` north
/// (`docs/03`). There is no float position in `sim`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

impl TilePos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Squared Euclidean distance, in `i64` so no map can overflow it.
    pub fn dist2(self, o: TilePos) -> i64 {
        let dx = i64::from(self.x).saturating_sub(i64::from(o.x));
        let dy = i64::from(self.y).saturating_sub(i64::from(o.y));
        dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
    }

    /// The four neighbours in the order `n`, `e`, `s`, `w` (`docs/02`).
    pub fn neighbours(self) -> [TilePos; 4] {
        [
            TilePos::new(self.x, self.y.saturating_add(1)),
            TilePos::new(self.x.saturating_add(1), self.y),
            TilePos::new(self.x, self.y.saturating_sub(1)),
            TilePos::new(self.x.saturating_sub(1), self.y),
        ]
    }

    pub fn is_adjacent(self, o: TilePos) -> bool {
        self.neighbours().contains(&o)
    }
}

/// Row then column (`docs/03`): north to south, then west to east.
pub fn row_then_column(a: &TilePos, b: &TilePos) -> std::cmp::Ordering {
    b.y.cmp(&a.y).then(a.x.cmp(&b.x))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Terrain {
    Ground,
    Rock,
    Water,
}

impl Terrain {
    pub fn name(self) -> &'static str {
        match self {
            Terrain::Ground => "ground",
            Terrain::Rock => "rock",
            Terrain::Water => "water",
        }
    }
}

/// A deposit as the map places it: per resource kind, an amount, a cap and
/// a regrowth per tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepositSpec {
    pub amount: BTreeMap<String, Num>,
    pub cap: BTreeMap<String, Num>,
    pub regrowth: BTreeMap<String, Num>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileSpec {
    pub terrain: Terrain,
    pub deposit: Option<DepositSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    pub name: String,
    pub speed: u64,
    pub width: i32,
    pub height: i32,
    /// Row-major from the south-west corner: index `(y + h/2) * width + (x + w/2)`.
    pub tiles: Vec<TileSpec>,
    /// Each team's starting tiles, in team order; a printer stands on each.
    pub teams: Vec<Vec<TilePos>>,
    /// The file's bytes, for the replay's identity.
    pub source: String,
}

impl Map {
    /// Parse and validate a map file against the tables.
    pub fn parse(text: &str, data: &Data) -> Result<Map, DataError> {
        const FILE: &str = "map";
        let err = |m: String| DataError {
            file: FILE,
            message: m,
        };
        let t: toml::Table = text
            .parse()
            .map_err(|e: toml::de::Error| err(e.to_string()))?;
        let get = |k: &str| t.get(k).ok_or_else(|| err(format!("missing key `{k}`")));
        let name = get("name")?
            .as_str()
            .ok_or_else(|| err("`name` is not a string".into()))?
            .to_string();
        let speed = get("speed")?
            .as_integer()
            .ok_or_else(|| err("`speed` is not an integer".into()))?;
        let speed = u64::try_from(speed).map_err(|_| err("`speed` is negative".into()))?;
        if !data.world.speed_steps.contains(&speed) {
            return Err(err(format!(
                "`speed` {speed} is not one of world.toml's steps"
            )));
        }
        let width = get("width")?
            .as_integer()
            .ok_or_else(|| err("`width`".into()))?;
        let height = get("height")?
            .as_integer()
            .ok_or_else(|| err("`height`".into()))?;
        let (width, height) = (
            i32::try_from(width).map_err(|_| err("`width`".into()))?,
            i32::try_from(height).map_err(|_| err("`height`".into()))?,
        );
        if width < 1 || height < 1 || width > 512 || height > 512 {
            return Err(err("`width` and `height` are between 1 and 512".into()));
        }
        let terrain = get("terrain")?
            .as_str()
            .ok_or_else(|| err("`terrain` is not a string".into()))?;
        let rows: Vec<&str> = terrain.lines().filter(|l| !l.trim().is_empty()).collect();
        if rows.len() != height as usize {
            return Err(err(format!(
                "`terrain` has {} rows, `height` is {height}",
                rows.len()
            )));
        }
        let default_dep = t
            .get("deposit_default")
            .map(|v| deposit_of(v, data))
            .transpose()?;
        let mut overrides: BTreeMap<TilePos, DepositSpec> = BTreeMap::new();
        if let Some(list) = t.get("deposit") {
            let arr = list
                .as_array()
                .ok_or_else(|| err("`deposit` is not a list".into()))?;
            for entry in arr {
                let at = pos_of(
                    entry
                        .get("at")
                        .ok_or_else(|| err("a `[[deposit]]` lacks `at`".into()))?,
                )?;
                overrides.insert(at, deposit_of(entry, data)?);
            }
        }
        let half_w = width.wrapping_div(2);
        let half_h = height.wrapping_div(2);
        let mut tiles = vec![
            TileSpec {
                terrain: Terrain::Ground,
                deposit: None
            };
            (width as usize).saturating_mul(height as usize)
        ];
        // The first row is the north edge.
        for (row_from_north, row) in rows.iter().enumerate() {
            let chars: Vec<char> = row.chars().collect();
            if chars.len() != width as usize {
                return Err(err(format!(
                    "`terrain` row {} has {} columns, `width` is {width}",
                    row_from_north.saturating_add(1),
                    chars.len()
                )));
            }
            let y = (height as i64)
                .saturating_sub(1)
                .saturating_sub(row_from_north as i64)
                .saturating_sub(i64::from(half_h));
            for (col, ch) in chars.iter().enumerate() {
                let x = (col as i64).saturating_sub(i64::from(half_w));
                let pos = TilePos::new(x as i32, y as i32);
                let (terrain, has_deposit) = match ch {
                    '.' => (Terrain::Ground, false),
                    'o' => (Terrain::Ground, true),
                    '#' => (Terrain::Rock, false),
                    '~' => (Terrain::Water, false),
                    other => {
                        return Err(err(format!(
                            "`terrain` holds `{other}`, not one of . o # ~"
                        )));
                    }
                };
                let deposit = if has_deposit || overrides.contains_key(&pos) {
                    if !data.world.terrain[terrain.name()].passable {
                        return Err(err(format!(
                            "a deposit at ({x}, {y}) sits on impassable terrain"
                        )));
                    }
                    Some(
                        overrides
                            .get(&pos)
                            .cloned()
                            .or_else(|| default_dep.clone())
                            .ok_or_else(|| err(format!("the deposit at ({x}, {y}) has no `[deposit_default]` and no override")))?,
                    )
                } else {
                    None
                };
                let idx = index(pos, width, height).ok_or_else(|| err("index".into()))?;
                tiles[idx] = TileSpec { terrain, deposit };
            }
        }
        let teams_v = get("team")?
            .as_array()
            .ok_or_else(|| err("`team` is not a list of tables".into()))?;
        let mut teams = Vec::new();
        for (i, tv) in teams_v.iter().enumerate() {
            let mut starts = Vec::new();
            if let Some(s) = tv.get("start") {
                starts.push(pos_of(s)?);
            }
            if let Some(list) = tv.get("starts") {
                for s in list
                    .as_array()
                    .ok_or_else(|| err("`starts` is not a list".into()))?
                {
                    starts.push(pos_of(s)?);
                }
            }
            if starts.is_empty() {
                return Err(err(format!("team {i} has no starting tile")));
            }
            teams.push(starts);
        }
        let map = Map {
            name,
            speed,
            width,
            height,
            tiles,
            teams,
            source: text.to_string(),
        };
        map.validate(data)?;
        Ok(map)
    }

    /// `docs/03`'s validity: every starting tile buildable, no two the
    /// same, every team with at least one, every start able to reach a
    /// deposit over passable terrain.
    fn validate(&self, data: &Data) -> Result<(), DataError> {
        let err = |m: String| DataError {
            file: "map",
            message: m,
        };
        let mut seen = BTreeSet::new();
        for (team, starts) in self.teams.iter().enumerate() {
            for &s in starts {
                let Some(tile) = self.tile(s) else {
                    return Err(err(format!(
                        "team {team}'s start ({}, {}) is off the map",
                        s.x, s.y
                    )));
                };
                if tile.terrain != Terrain::Ground || tile.deposit.is_some() {
                    return Err(err(format!(
                        "team {team}'s start ({}, {}) is not buildable",
                        s.x, s.y
                    )));
                }
                if !seen.insert(s) {
                    return Err(err(format!("two teams start on ({}, {})", s.x, s.y)));
                }
                if !self.reaches_a_deposit(s, data) {
                    return Err(err(format!(
                        "team {team}'s start ({}, {}) cannot reach a deposit",
                        s.x, s.y
                    )));
                }
            }
        }
        Ok(())
    }

    fn reaches_a_deposit(&self, from: TilePos, data: &Data) -> bool {
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from);
        seen.insert(from);
        while let Some(p) = queue.pop_front() {
            let Some(tile) = self.tile(p) else { continue };
            if tile.deposit.is_some() {
                return true;
            }
            for n in p.neighbours() {
                if let Some(nt) = self.tile(n)
                    && data.world.terrain[nt.terrain.name()].passable
                    && seen.insert(n)
                {
                    queue.push_back(n);
                }
            }
        }
        false
    }

    pub fn in_bounds(&self, p: TilePos) -> bool {
        index(p, self.width, self.height).is_some()
    }

    pub fn tile(&self, p: TilePos) -> Option<&TileSpec> {
        index(p, self.width, self.height).map(|i| &self.tiles[i])
    }

    /// Every tile position in row-then-column order.
    pub fn positions(&self) -> Vec<TilePos> {
        let half_w = self.width.wrapping_div(2);
        let half_h = self.height.wrapping_div(2);
        let mut out = Vec::with_capacity(self.tiles.len());
        for row in (0..self.height).rev() {
            for col in 0..self.width {
                out.push(TilePos::new(
                    col.wrapping_sub(half_w),
                    row.wrapping_sub(half_h),
                ));
            }
        }
        out
    }

    /// The south-west corner and the north-east corner.
    pub fn bounds(&self) -> (TilePos, TilePos) {
        let half_w = self.width.wrapping_div(2);
        let half_h = self.height.wrapping_div(2);
        (
            TilePos::new(0i32.wrapping_sub(half_w), 0i32.wrapping_sub(half_h)),
            TilePos::new(
                self.width.wrapping_sub(1).wrapping_sub(half_w),
                self.height.wrapping_sub(1).wrapping_sub(half_h),
            ),
        )
    }
}

/// The index of `p` in a row-major-from-the-south-west tile vector, or
/// `None` off the rectangle. The one statement of the bounds rule.
pub fn index(p: TilePos, width: i32, height: i32) -> Option<usize> {
    let half_w = width.wrapping_div(2);
    let half_h = height.wrapping_div(2);
    let col = p.x.checked_add(half_w)?;
    let row = p.y.checked_add(half_h)?;
    if col < 0 || row < 0 || col >= width || row >= height {
        return None;
    }
    Some(
        (row as usize)
            .saturating_mul(width as usize)
            .saturating_add(col as usize),
    )
}

fn pos_of(v: &toml::Value) -> Result<TilePos, DataError> {
    let err = |m: &str| DataError {
        file: "map",
        message: m.to_string(),
    };
    let a = v.as_array().ok_or_else(|| err("a position is `[x, y]`"))?;
    if a.len() != 2 {
        return Err(err("a position is `[x, y]`"));
    }
    let x = a[0]
        .as_integer()
        .and_then(|i| i32::try_from(i).ok())
        .ok_or_else(|| err("x"))?;
    let y = a[1]
        .as_integer()
        .and_then(|i| i32::try_from(i).ok())
        .ok_or_else(|| err("y"))?;
    Ok(TilePos::new(x, y))
}

/// A deposit row: `cap`, `regrowth`, `amount` for every resource kind, each
/// bounded by `world.toml`'s maxima. A plain integer applies to every kind.
fn deposit_of(v: &toml::Value, data: &Data) -> Result<DepositSpec, DataError> {
    let err = |m: String| DataError {
        file: "map",
        message: m,
    };
    let field = |k: &str| -> Result<BTreeMap<String, Num>, DataError> {
        let raw = v
            .get(k)
            .ok_or_else(|| err(format!("a deposit lacks `{k}`")))?;
        let mut out = BTreeMap::new();
        match raw {
            toml::Value::Integer(i) => {
                let n = Num::from_int(i128::from(*i))
                    .map_err(|_| err(format!("`{k}` is out of range")))?;
                for kind in &data.machines.kinds {
                    out.insert(kind.clone(), n);
                }
            }
            toml::Value::Table(t) => {
                for kind in &data.machines.kinds {
                    let x = t
                        .get(kind)
                        .ok_or_else(|| err(format!("`{k}` lacks `{kind}`")))?;
                    let i = x
                        .as_integer()
                        .ok_or_else(|| err(format!("`{k}.{kind}` is not an integer")))?;
                    out.insert(
                        kind.clone(),
                        Num::from_int(i128::from(i))
                            .map_err(|_| err(format!("`{k}` is out of range")))?,
                    );
                }
            }
            _ => return Err(err(format!("`{k}` is an integer or a table by kind"))),
        }
        Ok(out)
    };
    let cap = field("cap")?;
    let regrowth = field("regrowth")?;
    let amount = field("amount")?;
    for kind in &data.machines.kinds {
        let (max_cap, max_regrowth) = data
            .world
            .deposit_max
            .get(kind)
            .copied()
            .unwrap_or((Num::ZERO, Num::ZERO));
        if cap[kind] > max_cap
            || regrowth[kind] > max_regrowth
            || amount[kind] > cap[kind]
            || amount[kind] < Num::ZERO
        {
            return Err(err(format!(
                "a deposit's `{kind}` exceeds world.toml's maxima or its own cap"
            )));
        }
    }
    Ok(DepositSpec {
        amount,
        cap,
        regrowth,
    })
}

pub const FIRST_MAP: &str = include_str!("../../../data/maps/first.toml");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_map_loads_with_the_origin_at_the_centre() {
        let data = Data::load().unwrap();
        let m = Map::parse(FIRST_MAP, &data).unwrap();
        assert_eq!((m.width, m.height), (24, 16));
        assert_eq!(m.bounds(), (TilePos::new(-12, -8), TilePos::new(11, 7)));
        assert!(m.in_bounds(TilePos::new(11, 7)) && !m.in_bounds(TilePos::new(12, 7)));
        // Row 2 of the file (y = 6): `..o.........##..........`
        assert_eq!(
            m.tile(TilePos::new(-10, 6)).unwrap().terrain,
            Terrain::Ground
        );
        assert!(m.tile(TilePos::new(-10, 6)).unwrap().deposit.is_some());
        assert_eq!(m.tile(TilePos::new(0, 6)).unwrap().terrain, Terrain::Rock);
        assert_eq!(m.tile(TilePos::new(8, 5)).unwrap().terrain, Terrain::Water);
        assert_eq!(
            m.teams,
            vec![vec![TilePos::new(-9, 4)], vec![TilePos::new(8, -5)]]
        );
        assert_eq!(m.positions()[0], TilePos::new(-12, 7));
        assert_eq!(m.positions()[1], TilePos::new(-11, 7));
    }

    #[test]
    fn invalid_maps_are_refused() {
        let data = Data::load().unwrap();
        let on_rock = FIRST_MAP.replace("start = [-9, 4]", "start = [0, 6]");
        assert!(
            Map::parse(&on_rock, &data)
                .unwrap_err()
                .message
                .contains("buildable")
        );
        let same = FIRST_MAP.replace("start = [8, -5]", "start = [-9, 4]");
        assert!(
            Map::parse(&same, &data)
                .unwrap_err()
                .message
                .contains("two teams")
        );
        let bad_speed = FIRST_MAP.replace("speed = 10", "speed = 7");
        assert!(
            Map::parse(&bad_speed, &data)
                .unwrap_err()
                .message
                .contains("speed")
        );
        let too_rich = FIRST_MAP.replace("cap = 500", "cap = 5000");
        assert!(
            Map::parse(&too_rich, &data)
                .unwrap_err()
                .message
                .contains("maxima")
        );
    }
}
