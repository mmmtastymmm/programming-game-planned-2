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
}

impl Terrain {
    pub fn name(self) -> &'static str {
        match self {
            Terrain::Ground => "ground",
            Terrain::Rock => "rock",
        }
    }
}

/// A tile's height (`docs/03`, Height; Q65): a whole number of levels. It
/// is its own type so that no bare number is taken for one, and the only
/// way to make one from outside checks it against the most the tables
/// allow, so a height out of range does not exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Height(u8);

impl Height {
    /// `level`, if it is at most `max`.
    pub fn new(level: u8, max: u8) -> Option<Height> {
        (level <= max).then_some(Height(level))
    }

    pub fn level(self) -> u8 {
        self.0
    }

    /// How the ground lies for something on a tile of this height that
    /// looks to a tile of height `to`, where a step may cross `step`
    /// levels.
    pub fn slope_to(self, to: Height, step: u8) -> Slope {
        let levels = self.0.abs_diff(to.0);
        match to.0.cmp(&self.0) {
            std::cmp::Ordering::Equal => Slope::Level,
            std::cmp::Ordering::Greater if levels <= step => Slope::Up,
            std::cmp::Ordering::Greater => Slope::Cliff,
            std::cmp::Ordering::Less if levels <= step => Slope::Down,
            std::cmp::Ordering::Less => Slope::Drop(levels),
        }
    }
}

/// How the ground lies between a tile and one beside it (`docs/03`, A
/// step and A drop). Every rule that looks at two tiles — a move, where a
/// printed bot appears, a map's check, and in time a move's ticks, a fall
/// and what is drawn — matches on this, so that none can leave a case
/// out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slope {
    /// The same height.
    Level,
    /// Higher, by no more than a step.
    Up,
    /// Lower, by no more than a step.
    Down,
    /// Higher by more than a step: nothing climbs it.
    Cliff,
    /// Lower by more than a step, by this many levels: a fall.
    Drop(u8),
}

impl Slope {
    /// Whether a bot may **step** across it (`docs/03`): not up a cliff,
    /// and not over a drop, which only a `move` that means to may take.
    pub fn is_step(self) -> bool {
        match self {
            Slope::Level | Slope::Up | Slope::Down => true,
            Slope::Cliff | Slope::Drop(_) => false,
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
    pub height: Height,
    /// The parts of water it starts with (`docs/09`, The map's water).
    pub water: u32,
    /// A spring's rate, parts a tick, if one is here; and a sink's.
    pub spring: Option<u32>,
    pub sink: Option<u32>,
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
    /// The ground water the match begins with, in parts (`docs/09`).
    pub ground_water: u64,
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
        // A height for every tile (Q65): a grid of digits shaped as the
        // terrain is, or none, when every tile stands at the default and
        // a `~` tile the lake's depth below it.
        let hd = data.world.height;
        let height_rows: Option<Vec<Vec<char>>> = match t.get("heights") {
            None => None,
            Some(v) => {
                let text = v
                    .as_str()
                    .ok_or_else(|| err("`heights` is not a string".into()))?;
                let lines: Vec<Vec<char>> = text
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| l.chars().collect())
                    .collect();
                if lines.len() != height as usize {
                    return Err(err(format!(
                        "`heights` has {} rows, `height` is {height}",
                        lines.len()
                    )));
                }
                Some(lines)
            }
        };
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
                height: Height::default(),
                water: 0,
                spring: None,
                sink: None,
                deposit: None
            };
            (width as usize).saturating_mul(height as usize)
        ];
        // The tiles the map marks `~`: ground that starts under water.
        let mut lake = vec![false; (width as usize).saturating_mul(height as usize)];
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
            if let Some(hr) = &height_rows
                && hr[row_from_north].len() != width as usize
            {
                return Err(err(format!(
                    "`heights` row {} has {} columns, `width` is {width}",
                    row_from_north.saturating_add(1),
                    hr[row_from_north].len()
                )));
            }
            let y = (height as i64)
                .saturating_sub(1)
                .saturating_sub(row_from_north as i64)
                .saturating_sub(i64::from(half_h));
            for (col, ch) in chars.iter().enumerate() {
                let x = (col as i64).saturating_sub(i64::from(half_w));
                let pos = TilePos::new(x as i32, y as i32);
                let under_water = *ch == '~';
                let (terrain, has_deposit) = match ch {
                    '.' | '~' => (Terrain::Ground, false),
                    'o' => (Terrain::Ground, true),
                    '#' => (Terrain::Rock, false),
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
                let level = match &height_rows {
                    Some(hr) => {
                        let digit = hr[row_from_north][col];
                        digit
                            .to_digit(10)
                            .map(|d| d as u8)
                            .ok_or_else(|| err(format!("`heights` holds `{digit}`, not a digit")))?
                    }
                    None if under_water => hd.default.saturating_sub(hd.lake_depth),
                    None => hd.default,
                };
                let tile_height = Height::new(level, hd.max).ok_or_else(|| {
                    err(format!(
                        "the height {level} at ({x}, {y}) is above `height.max`, {}",
                        hd.max
                    ))
                })?;
                let idx = index(pos, width, height).ok_or_else(|| err("index".into()))?;
                tiles[idx] = TileSpec {
                    terrain,
                    height: tile_height,
                    water: 0,
                    spring: None,
                    sink: None,
                    deposit,
                };
                lake[idx] = under_water;
            }
        }
        fill_lakes(&mut tiles, &lake, width, height, data).map_err(err)?;
        // Springs and sinks (`docs/09`): each a tile and a rate.
        for (key, is_spring) in [("spring", true), ("sink", false)] {
            let Some(list) = t.get(key) else { continue };
            let arr = list
                .as_array()
                .ok_or_else(|| err(format!("`{key}` is not a list of tables")))?;
            for entry in arr {
                let at = pos_of(
                    entry
                        .get("at")
                        .ok_or_else(|| err(format!("a `[[{key}]]` lacks `at`")))?,
                )?;
                let rate = entry
                    .get("rate")
                    .and_then(toml::Value::as_integer)
                    .and_then(|r| u32::try_from(r).ok())
                    .filter(|r| *r >= 1 && *r <= data.world.water.max_rate)
                    .ok_or_else(|| {
                        err(format!(
                            "the {key} at ({}, {}) needs a whole `rate` from 1 to `water.max_rate`, {}",
                            at.x, at.y, data.world.water.max_rate
                        ))
                    })?;
                let idx = index(at, width, height).ok_or_else(|| {
                    err(format!("the {key} at ({}, {}) is off the map", at.x, at.y))
                })?;
                let tile = &mut tiles[idx];
                if tile.terrain != Terrain::Ground
                    || tile.deposit.is_some()
                    || tile.spring.is_some()
                    || tile.sink.is_some()
                {
                    return Err(err(format!(
                        "the {key} at ({}, {}) needs a ground tile of its own, with no deposit",
                        at.x, at.y
                    )));
                }
                if is_spring {
                    tile.spring = Some(rate);
                } else {
                    tile.sink = Some(rate);
                }
            }
        }
        let ground_water = match t.get("ground_water") {
            None => 0,
            Some(v) => v
                .as_integer()
                .and_then(|n| u64::try_from(n).ok())
                .ok_or_else(|| err("`ground_water` is not a whole number of parts".into()))?,
        };
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
            ground_water,
            source: text.to_string(),
        };
        map.validate(data)?;
        Ok(map)
    }

    /// `docs/03`'s validity: every starting tile buildable, no two the
    /// same, every team with at least one, every start able to reach a
    /// deposit by steps a bot may take.
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
                if tile.terrain != Terrain::Ground
                    || tile.deposit.is_some()
                    || tile.water >= data.world.water.wet
                    || tile.spring.is_some()
                    || tile.sink.is_some()
                {
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
                    && tile
                        .height
                        .slope_to(nt.height, data.world.height.step)
                        .is_step()
                    && nt.water <= data.world.water.wade
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
        self.walk().map(|(p, _)| p).collect()
    }

    /// Every tile in row-then-column order, the order the state hash
    /// writes them in, as its position and its index in a tile vector
    /// ([`index`]): a walk that builds no list, for a caller that makes it
    /// often (T41).
    pub fn walk(&self) -> impl Iterator<Item = (TilePos, usize)> + use<> {
        let (width, height) = (self.width, self.height);
        let half_w = width.wrapping_div(2);
        let half_h = height.wrapping_div(2);
        (0..height).rev().flat_map(move |row| {
            (0..width).map(move |col| {
                (
                    TilePos::new(col.wrapping_sub(half_w), row.wrapping_sub(half_h)),
                    (row as usize)
                        .saturating_mul(width as usize)
                        .saturating_add(col as usize),
                )
            })
        })
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

/// `docs/09`, The map's water: each set of `~` tiles joined by adjacency
/// starts full to its rim. Its surface is the height of the lowest open
/// tile beside it that is not `~`, or one level above its own highest
/// tile where nothing open is beside it, and every tile in it must stand
/// below that.
fn fill_lakes(
    tiles: &mut [TileSpec],
    lake: &[bool],
    width: i32,
    height: i32,
    data: &Data,
) -> Result<(), String> {
    let parts = data.world.water.parts;
    let half_w = width.wrapping_div(2);
    let half_h = height.wrapping_div(2);
    let pos_of_index = |i: usize| {
        TilePos::new(
            (i.checked_rem(width as usize).unwrap_or(0) as i32).wrapping_sub(half_w),
            (i.checked_div(width as usize).unwrap_or(0) as i32).wrapping_sub(half_h),
        )
    };
    let mut done = vec![false; tiles.len()];
    for start in 0..tiles.len() {
        if !lake[start] || done[start] {
            continue;
        }
        // One set of `~` tiles, and the open tiles beside it.
        let mut set = vec![start];
        let mut rim: Option<u8> = None;
        done[start] = true;
        let mut next = 0;
        while next < set.len() {
            let here = set[next];
            next = next.saturating_add(1);
            for n in pos_of_index(here).neighbours() {
                let Some(ni) = index(n, width, height) else {
                    continue;
                };
                if lake[ni] {
                    if !done[ni] {
                        done[ni] = true;
                        set.push(ni);
                    }
                } else if tiles[ni].terrain == Terrain::Ground {
                    let level = tiles[ni].height.level();
                    rim = Some(rim.map_or(level, |r| r.min(level)));
                }
            }
        }
        let highest = set.iter().map(|i| tiles[*i].height.level()).max();
        let surface = rim.unwrap_or_else(|| highest.unwrap_or(0).saturating_add(1));
        for &i in &set {
            let own = tiles[i].height.level();
            if own >= surface {
                let p = pos_of_index(i);
                return Err(format!(
                    "the `~` tile at ({}, {}) stands at {own}, not below its water's surface, {surface}",
                    p.x, p.y
                ));
            }
            tiles[i].water = u32::from(surface.saturating_sub(own)).saturating_mul(parts);
        }
    }
    Ok(())
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
        // `~` is ground under water (Q65): two levels of it, full to the
        // ground beside it.
        let lake = m.tile(TilePos::new(8, 5)).unwrap();
        assert_eq!(lake.terrain, Terrain::Ground);
        assert_eq!(lake.water, 2 * data.world.water.parts);
        assert_eq!(
            m.teams,
            vec![vec![TilePos::new(-9, 4)], vec![TilePos::new(8, -5)]]
        );
        assert_eq!(m.positions()[0], TilePos::new(-12, 7));
        assert_eq!(m.positions()[1], TilePos::new(-11, 7));
        // The walk is the same order, sorted as the hash sorts, and each
        // position comes with the index the bounds rule gives it.
        let walked: Vec<(TilePos, usize)> = m.walk().collect();
        assert_eq!(walked.len(), m.tiles.len());
        let mut sorted = m.positions();
        sorted.sort_by(row_then_column);
        assert_eq!(sorted, m.positions());
        for (p, i) in walked {
            assert_eq!(index(p, m.width, m.height), Some(i));
        }
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
