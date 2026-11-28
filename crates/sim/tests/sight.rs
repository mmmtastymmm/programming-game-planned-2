//! Sight over height (`docs/03`, Line of sight; Q65 as Q68 amended it):
//! ground on the ray that is higher than the seer and above the line
//! blocks it, a machine's range to a tile is stretched and
//! shortened by the levels between them, `attack` gains the blocking and
//! keeps its range, and hearing knows nothing of any of it. T44.
//!
//! The arithmetic lint is relaxed here: this file works its expected
//! values out in plain sums on small numbers, and a sum that overflowed
//! would be a failed test, not a desync, since nothing here reaches the
//! sim's state but through its API.
#![allow(clippy::arithmetic_side_effects)]

use sim::map::{Height, Terrain};
use sim::sense;
use sim::world::{EntityId, Model, Sound, TileState};
use sim::{Command, CommandKind, Data, Map, Sim, TeamId, TilePos, bundle_of};
use std::collections::BTreeSet;

const WIDTH: i32 = 25;
const HEIGHT: i32 = 9;

/// 25 by 9, origin at the centre: x in -12..=12, y in -4..=4. Each team's
/// printer stands at an end of the middle row with its deposit beside
/// it, behind a wall of rock from the north edge to the south, at x = -10
/// and x = 10, so that no printer sees into the field between the walls
/// and what a team sees there is what the bots a test places see. Every
/// tile stands at `base` but those `heights` names, and `rocks` are rock.
fn map_text(base: u8, heights: &[(i32, i32, u8)], rocks: &[(i32, i32)]) -> String {
    let (mut terrain, mut levels) = (String::new(), String::new());
    for y in (-(HEIGHT / 2)..=HEIGHT / 2).rev() {
        for x in -(WIDTH / 2)..=WIDTH / 2 {
            let wall = x.abs() == 10 || rocks.contains(&(x, y));
            terrain.push(match (wall, x.abs() == 11 && y == 0) {
                (true, _) => '#',
                (false, true) => 'o',
                (false, false) => '.',
            });
            let level = heights
                .iter()
                .find(|h| (h.0, h.1) == (x, y))
                .map_or(base, |h| h.2);
            levels.push(char::from(b'0' + level));
        }
        terrain.push('\n');
        levels.push('\n');
    }
    format!(
        "name = \"sight\"\nspeed = 10\nwidth = {WIDTH}\nheight = {HEIGHT}\nterrain = \"\"\"\n{terrain}\"\"\"\nheights = \"\"\"\n{levels}\"\"\"\n[deposit_default]\ncap = 50\nregrowth = 1\namount = 50\n[[team]]\nstart = [-12, 0]\n[[team]]\nstart = [12, 0]\n"
    )
}

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y)
}

/// A match on the field: ground at 4 but for `heights`.
fn field(heights: &[(i32, i32, u8)], rocks: &[(i32, i32)]) -> Sim {
    field_at(4, heights, rocks, Data::load().unwrap())
}

fn field_at(base: u8, heights: &[(i32, i32, u8)], rocks: &[(i32, i32)], data: Data) -> Sim {
    let map = Map::parse(&map_text(base, heights, rocks), &data).unwrap();
    Sim::with_data(map, data)
}

/// A bot of `team` placed by fiat.
fn bot(sim: &mut Sim, team: u32, x: i32, y: i32) -> EntityId {
    sim.world
        .place(Model::Bot, TeamId(team), at(x, y), Some("bot".into()))
}

fn deploy(team: u32, main: &str) -> Command {
    Command::new(
        0,
        TeamId(team),
        0,
        CommandKind::Deploy {
            deployment: "bot".into(),
            bundle: bundle_of(main),
        },
    )
}

/// Run `sim` for `ticks`, with each team's program deployed to its bots
/// on the first: what the bot `id` logged.
fn run(sim: &mut Sim, id: EntityId, mains: &[(u32, &str)], ticks: u64) -> Vec<String> {
    let mut logs = Vec::new();
    for t in 0..ticks {
        let mut cmds = Vec::new();
        if t == 0 {
            for (team, main) in mains {
                let mut c = deploy(*team, main);
                c.tick = sim.world.tick + 1;
                cmds.push(c);
            }
        }
        sim.step(&cmds, &[]);
        if let Some(m) = sim.world.machines.get(&id) {
            logs.extend(m.log.iter().map(|l| l.text.clone()));
        }
    }
    logs
}

const IDLE: &str = "while True:\n    wait(1)\n";

/// What the team's memory holds of a tile after its last vision pass.
fn state(sim: &Sim, team: u32, x: i32, y: i32) -> TileState {
    let idx = sim.world.tile_index(at(x, y)).unwrap();
    sim.world.teams[&TeamId(team)].memory[idx].state
}

/// Every tile of the map the machine `seer` sees, by the one test.
fn seen_by(sim: &Sim, seer: EntityId) -> BTreeSet<TilePos> {
    sim.world
        .map
        .positions()
        .into_iter()
        .filter(|&p| sense::sees_tile(&sim.world, seer, p))
        .collect()
}

/// The rule before height (Q7): within `vision` by squared distance, and
/// no rock on the ray.
fn seen_before_height(sim: &Sim, from: TilePos, vision: i64) -> BTreeSet<TilePos> {
    let w = &sim.world;
    w.map
        .positions()
        .into_iter()
        .filter(|&p| {
            from.dist2(p) <= vision * vision
                && sense::ray(from, p)
                    .into_iter()
                    .all(|q| w.tile(q).unwrap().terrain != Terrain::Rock)
        })
        .collect()
}

#[test]
fn level_ground_sees_as_it_did_before_height() {
    // A rock in the field, so the old rule has something to block.
    let rocks = [(2, 0), (-3, 2), (-3, 1)];
    for base in [0, 4, 9] {
        let mut s = field_at(base, &[], &rocks, Data::load().unwrap());
        for (x, y) in [(0, 0), (-5, 3), (7, -4), (3, 0)] {
            let id = bot(&mut s, 0, x, y);
            let vision = s.world.data.model("bot").vision;
            let before = seen_before_height(&s, at(x, y), vision);
            assert_eq!(seen_by(&s, id), before, "from ({x}, {y}) at {base}");
            assert!(before.len() > 20, "the old rule saw something");
            s.world.remove(id);
        }
        // And the team's view is its machines' views together: here, the
        // one bot's and what the printer sees of its own side of the wall.
        let id = bot(&mut s, 0, 0, 0);
        let visible = sense::team_visible_tiles(&s.world, TeamId(0));
        let in_field: BTreeSet<TilePos> = visible.into_iter().filter(|p| p.x.abs() < 10).collect();
        assert_eq!(in_field, seen_by(&s, id));
        assert!(in_field.contains(&at(2, 0)), "the rock itself is seen");
        assert!(
            !in_field.contains(&at(3, 0)),
            "and what is behind it is not"
        );
        // A rock that stands lower than everything round it still blocks:
        // its terrain does, whatever its height.
        let idx = s.world.tile_index(at(2, 0)).unwrap();
        s.world.set_height(idx, Height::new(0, 9).unwrap());
        assert!(!sense::sees_tile(&s.world, id, at(3, 0)));
    }
}

#[test]
fn a_ridge_between_two_machines_hides_each_from_the_other() {
    // A ridge a level high across the field between two bots, three
    // tiles to each side of it.
    let ridge: Vec<(i32, i32, u8)> = (-4..=4).map(|y| (0, y, 5)).collect();
    let mut s = field(&ridge, &[]);
    let (a, b) = (bot(&mut s, 0, -3, 0), bot(&mut s, 1, 3, 0));
    let w = &mut s.world;
    assert!(!sense::clear(w, at(-3, 0), at(3, 0)));
    assert!(!sense::clear(w, at(3, 0), at(-3, 0)));
    assert!(!sense::team_can_see(w, TeamId(0), b));
    assert!(!sense::team_can_see(w, TeamId(1), a));
    // Each sees the ridge, which is in range and has nothing before it,
    // and nothing on the ground behind it.
    for (seer, far) in [(a, 1), (b, -1)] {
        assert!(sense::sees_tile(w, seer, at(0, 0)), "the ridge is seen");
        for x in 1..=3 {
            assert!(!sense::sees_tile(w, seer, at(far * x, 0)), "behind it");
        }
    }
    // What a program is told: its own team's machines and no rival, and
    // an unknown tile behind the ridge.
    let main = format!(
        "wait(1)\nlog(len([m for m in see() if m.team != me().team]), tile(1, 0) == None, tile(0, 0).height)\n{IDLE}"
    );
    assert_eq!(run(&mut s, a, &[(0, &main)], 4), ["0 True 5"]);

    // On level ground the two see each other, as before, and the tile
    // that was behind the ridge is known.
    let mut s = field(&[], &[]);
    let (a, b) = (bot(&mut s, 0, -3, 0), bot(&mut s, 1, 3, 0));
    assert!(sense::team_can_see(&mut s.world, TeamId(0), b));
    assert!(sense::team_can_see(&mut s.world, TeamId(1), a));
    assert_eq!(run(&mut s, a, &[(0, &main)], 4), ["1 False 4"]);
}

#[test]
fn a_tile_exactly_on_the_line_does_not_block_and_one_above_it_does() {
    // From (-2, 0) at 4 up to (2, 0) at 6, four steps: the line stands at
    // 4.5, 5 and 5.5 over the three tiles between.
    let slope = |mid: u8| field(&[(2, 0, 6), (0, 0, mid)], &[]);
    let s = slope(5);
    assert!(sense::clear(&s.world, at(-2, 0), at(2, 0)), "5 is on it");
    assert!(sense::clear(&s.world, at(2, 0), at(-2, 0)));
    let s = slope(6);
    assert!(!sense::clear(&s.world, at(-2, 0), at(2, 0)), "6 is above");
    // Looking down from the top, a tile no higher than the looker never
    // blocks (Q68), so the same line is clear from its high end.
    assert!(sense::clear(&s.world, at(2, 0), at(-2, 0)));
    // The test is in whole numbers: a tile at 5 a step from the low end
    // stands above 4.5 and blocks, and the same tile a step from the high
    // end stands below 5.5 and does not.
    let s = field(&[(2, 0, 6), (-1, 0, 5)], &[]);
    assert!(!sense::clear(&s.world, at(-2, 0), at(2, 0)));
    assert!(sense::clear(&s.world, at(2, 0), at(-2, 0)), "down is clear");
    let s = field(&[(2, 0, 6), (1, 0, 5)], &[]);
    assert!(sense::clear(&s.world, at(-2, 0), at(2, 0)));
    assert!(sense::clear(&s.world, at(2, 0), at(-2, 0)));
    // A stair is its own line: 4, 5, 6, 7 sees end to end, both ways.
    let s = field(&[(1, 0, 5), (2, 0, 6), (3, 0, 7)], &[]);
    assert!(sense::clear(&s.world, at(0, 0), at(3, 0)));
    assert!(sense::clear(&s.world, at(3, 0), at(0, 0)));
    // The same tiles in another order are not, looking up: 6 a step from
    // the low end is above the line's 5 there. Looking down from 7,
    // nothing on the way is higher than the looker.
    let s = field(&[(1, 0, 6), (2, 0, 5), (3, 0, 7)], &[]);
    assert!(!sense::clear(&s.world, at(0, 0), at(3, 0)));
    assert!(sense::clear(&s.world, at(3, 0), at(0, 0)));
}

#[test]
fn a_machine_on_a_hill_sees_further_and_is_seen_from_less_far() {
    // One tile two levels up. A bot's vision is 6.
    let mut s = field(&[(0, 0, 6)], &[]);
    let top = bot(&mut s, 0, 0, 0);
    let w = &mut s.world;
    // Down to ground two levels below, it sees two tiles further: 8.
    assert!(sense::sees_tile(w, top, at(8, 0)));
    assert!(!sense::sees_tile(w, top, at(9, 0)));
    assert!(sense::sees_tile(w, top, at(-8, 0)));
    // By squared distance: 36 + 16 is within 8 × 8, and 49 + 16 is not.
    assert!(sense::sees_tile(w, top, at(6, 4)));
    assert!(!sense::sees_tile(w, top, at(7, 4)));
    // The team's view holds the same tiles: the pass looks as far as the
    // hill lets it.
    let visible = sense::team_visible_tiles(w, TeamId(0));
    assert!(visible.contains(&at(8, 0)) && !visible.contains(&at(9, 0)));
    let in_field: BTreeSet<TilePos> = visible.into_iter().filter(|p| p.x.abs() < 10).collect();
    assert_eq!(in_field, seen_by(&s, top));

    // Looking up at the hill from the ground, a bot sees two tiles less
    // far: from 4 away and not from 5.
    let near = bot(&mut s, 1, 4, 0);
    let far = bot(&mut s, 1, -5, 0);
    let w = &mut s.world;
    assert!(sense::sees_tile(w, near, at(0, 0)));
    assert!(!sense::sees_tile(w, far, at(0, 0)));
    assert!(sense::sees_tile(w, far, at(-1, 0)), "the ground before it");
    // So the bot on the hill sees a rival that does not see it.
    assert!(sense::team_can_see(w, TeamId(0), far));
    w.remove(near);
    assert!(!sense::team_can_see(w, TeamId(1), top));
    // And the hill hides the ground behind it from below.
    assert!(!sense::sees_tile(w, far, at(1, 0)));

    // The reach is the vision and every level between: from the top of
    // the tables' heights down to their bottom a bot sees 6 + 9 tiles,
    // and the pass that gathers a team's view looks that far.
    let mut s = field_at(0, &[(-6, 0, 9)], &[], Data::load().unwrap());
    let tower = bot(&mut s, 0, -6, 0);
    assert!(sense::sees_tile(&s.world, tower, at(9, 0)), "15 away");
    assert!(!sense::sees_tile(&s.world, tower, at(9, 1)), "past 15");
    let visible = sense::team_visible_tiles(&s.world, TeamId(0));
    assert!(visible.contains(&at(9, 0)) && !visible.contains(&at(9, 1)));
}

#[test]
fn a_machine_in_a_pit_sees_the_tiles_beside_it_and_no_further() {
    // One tile two levels down.
    let mut s = field(&[(0, 0, 2)], &[]);
    let low = bot(&mut s, 0, 0, 0);
    // The ground beside the pit stands above every line out of it, so it
    // sees its own tile and the eight round it: a ray of length one
    // passes through nothing.
    let ring: BTreeSet<TilePos> = (-1..=1)
        .flat_map(|y| (-1..=1).map(move |x| at(x, y)))
        .collect();
    assert_eq!(seen_by(&s, low), ring);
    // Into the pit is another matter (Q68): it is seen from the rim, and
    // from a tile back as well, since the rim stands no higher than the
    // looker. So the bot in it is seen by what it cannot see.
    let rim = bot(&mut s, 1, 1, 0);
    let back = bot(&mut s, 1, 0, 2);
    let w = &mut s.world;
    assert!(sense::sees_tile(w, rim, at(0, 0)));
    assert!(sense::sees_tile(w, back, at(0, 0)));
    assert!(sense::sees_tile(w, back, at(0, -2)), "and across it");
    assert!(!sense::sees_tile(w, low, at(0, 2)));
    w.remove(rim);
    assert!(sense::team_can_see(w, TeamId(1), low));
    assert!(!sense::team_can_see(w, TeamId(0), back));

    // A hollow one level down is a pit all the same.
    let mut s = field(&[(0, 0, 3)], &[]);
    let low = bot(&mut s, 0, 0, 0);
    let w = &s.world;
    assert!(sense::sees_tile(w, low, at(1, 0)) && sense::sees_tile(w, low, at(1, 1)));
    assert!(!sense::sees_tile(w, low, at(2, 0)), "the rim is in the way");

    // Range, apart from the line: a trench a level down, in which no
    // tile stands above a line along it. A bot at its end sees along it
    // as on level ground, 6 tiles and not 7.
    let trench: Vec<(i32, i32, u8)> = (0..=8).map(|x| (x, 0, 3)).collect();
    let mut s = field(&trench, &[]);
    let low = bot(&mut s, 0, 0, 0);
    assert!(sense::sees_tile(&s.world, low, at(6, 0)));
    assert!(!sense::sees_tile(&s.world, low, at(7, 0)), "level: 6");
    // With its far end back up at 4, that end is a level above the bot:
    // seen from 5 away and not from 6.
    let trench: Vec<(i32, i32, u8)> = (0..=4).map(|x| (x, 0, 3)).collect();
    let mut s = field(&trench, &[]);
    let low = bot(&mut s, 0, 0, 0);
    assert!(
        sense::sees_tile(&s.world, low, at(5, 0)),
        "5 away, a level up"
    );
    let trench: Vec<(i32, i32, u8)> = (0..=5).map(|x| (x, 0, 3)).collect();
    let mut s = field(&trench, &[]);
    let low = bot(&mut s, 0, 0, 0);
    assert!(
        sense::sees_tile(&s.world, low, at(5, 0)),
        "level, in the trench"
    );
    assert!(
        !sense::sees_tile(&s.world, low, at(6, 0)),
        "6 away, a level up"
    );

    // Nothing below zero: at the bottom of the tables' heights a bot does
    // not see the top of them even on the tile beside it, 6 + 0 - 9
    // being below zero, and it still sees its own tile.
    let mut s = field_at(
        0,
        &[(1, 0, 9), (0, 1, 6), (-1, 0, 5)],
        &[],
        Data::load().unwrap(),
    );
    let low = bot(&mut s, 0, 0, 0);
    let w = &s.world;
    assert!(sense::sees_tile(w, low, at(0, 0)));
    assert!(!sense::sees_tile(w, low, at(1, 0)), "9 up: below zero");
    // 6 up leaves a range of zero, which reaches no tile but its own.
    assert!(!sense::sees_tile(w, low, at(0, 1)), "6 up: zero");
    // 5 up leaves one: the tile beside it.
    assert!(sense::sees_tile(w, low, at(-1, 0)), "5 up: one");
}

#[test]
fn lower_ground_is_seen_past_ground_no_higher_than_the_seer() {
    // A hollow two levels down, three tiles wide, with level ground
    // between it and a bot three tiles off: the bank stands above the
    // line down into it and blocks nothing, being no higher than the bot.
    let hollow = [(0, 0, 2), (1, 0, 2), (2, 0, 2)];
    let mut s = field(&hollow, &[]);
    let high = bot(&mut s, 0, -3, 0);
    for x in 0..=2 {
        assert!(sense::sees_tile(&s.world, high, at(x, 0)), "({x}, 0)");
    }
    // And from in the hollow, the bank hides the ground beyond it.
    let low = bot(&mut s, 1, 1, 0);
    let w = &mut s.world;
    assert!(sense::sees_tile(w, low, at(-1, 0)), "the bank itself");
    assert!(!sense::sees_tile(w, low, at(-2, 0)) && !sense::sees_tile(w, low, at(-3, 0)));
    assert!(sense::team_can_see(w, TeamId(0), low));
    assert!(!sense::team_can_see(w, TeamId(1), high));

    // A rise in a valley, lower than the looker, hides nothing behind it:
    // from 6, over a floor at 2 with a bump at 4, the floor past the bump
    // is seen, though the bump stands above the line down to it.
    let mut valley: Vec<(i32, i32, u8)> = vec![(-3, 0, 6), (0, 0, 4)];
    valley.extend((-2..=3).filter(|x| *x != 0).map(|x| (x, 0, 2)));
    let mut s = field(&valley, &[]);
    let top = bot(&mut s, 0, -3, 0);
    assert!(sense::sees_tile(&s.world, top, at(2, 0)));
    // From the floor, the same bump hides the floor beyond it.
    let floor = bot(&mut s, 1, -2, 0);
    assert!(!sense::sees_tile(&s.world, floor, at(2, 0)));

    // Ground higher than the looker still hides what is lower behind it.
    let mut s = field(&[(-1, 0, 5), (1, 0, 2)], &[]);
    let high = bot(&mut s, 0, -3, 0);
    assert!(sense::sees_tile(&s.world, high, at(-1, 0)), "the ridge");
    assert!(!sense::sees_tile(&s.world, high, at(1, 0)), "behind it");
    assert!(sense::sees_tile(&s.world, high, at(-1, 2)), "off its line");

    // And range still counts: two levels down is seen from two tiles
    // further than level ground, 8 for a bot, and no further.
    let far: Vec<(i32, i32, u8)> = vec![(4, 0, 2), (5, 0, 2), (6, 0, 2)];
    let mut s = field(&far, &[]);
    let high = bot(&mut s, 0, -3, 0);
    assert!(sense::sees_tile(&s.world, high, at(5, 0)), "8 away");
    assert!(!sense::sees_tile(&s.world, high, at(6, 0)), "9 away");
    assert!(!sense::sees_tile(&s.world, high, at(4, 1)), "level, 7 away");
}

#[test]
fn the_ray_is_walked_from_the_seer_so_sight_is_not_symmetric() {
    // A knight's move: the walk from (0, 0) to (2, 1) passes through
    // (1, 0), and the walk back through (1, 1) (`docs/03`).
    assert_eq!(sense::ray(at(0, 0), at(2, 1)), [at(1, 0)]);
    assert_eq!(sense::ray(at(2, 1), at(0, 0)), [at(1, 1)]);
    // A tile a level up on the first walk and not on the second.
    let mut s = field(&[(1, 0, 5)], &[]);
    let (a, b) = (bot(&mut s, 0, 0, 0), bot(&mut s, 1, 2, 1));
    let w = &mut s.world;
    assert!(!sense::clear(w, at(0, 0), at(2, 1)));
    assert!(sense::clear(w, at(2, 1), at(0, 0)));
    assert!(!sense::sees_tile(w, a, at(2, 1)) && sense::sees_tile(w, b, at(0, 0)));
    assert!(!sense::team_can_see(w, TeamId(0), b));
    assert!(sense::team_can_see(w, TeamId(1), a));
    // And the other way about, with the high tile on the walk back.
    let mut s = field(&[(1, 1, 5)], &[]);
    let (a, b) = (bot(&mut s, 0, 0, 0), bot(&mut s, 1, 2, 1));
    let w = &mut s.world;
    assert!(sense::team_can_see(w, TeamId(0), b));
    assert!(!sense::team_can_see(w, TeamId(1), a));

    // The steps are counted from the seer too. Up a slope from 4 to 7 in
    // three steps, a tile at 6 one step from the low end is above the
    // line's 5; taken for the tile one step from the high end, where the
    // line is at 6, it would not block. The same slope run the other way
    // on the map is counted from its own low end just the same.
    let s = field(&[(1, 0, 6), (2, 0, 5), (3, 0, 7)], &[]);
    assert!(!sense::clear(&s.world, at(0, 0), at(3, 0)));
    let s = field(&[(-1, 0, 6), (-2, 0, 5), (-3, 0, 7)], &[]);
    assert!(!sense::clear(&s.world, at(0, 0), at(-3, 0)));
    let s = field(&[(1, 0, 5), (2, 0, 6), (3, 0, 7)], &[]);
    assert!(sense::clear(&s.world, at(0, 0), at(3, 0)));
    assert!(sense::clear(&s.world, at(3, 0), at(0, 0)));
}

/// The tables with a bot's reach in an attack made three tiles, so that
/// there is ground between an attacker and its target.
fn long_arms() -> Data {
    let mut data = Data::load().unwrap();
    data.machines.models.get_mut("bot").unwrap().attack_range = 3;
    data
}

const ATTACK: &str = "wait(1)\ntry:\n    attack([m for m in see() if m.team != me().team][0].id)\n    log('hit')\nexcept ValueError:\n    log('refused')\nwhile True:\n    wait(1)\n";

#[test]
fn attack_is_blocked_by_higher_ground_and_keeps_its_range() {
    // An attacker at (-1, 0), its target at (1, 0), and a spotter of the
    // attacker's team beside the target, so the team sees it whatever
    // stands between the two: what refuses the attack is the attacker's
    // own line.
    let fight = |heights: &[(i32, i32, u8)], data: Data, from: (i32, i32)| {
        let mut s = field_at(4, heights, &[], data);
        let attacker = bot(&mut s, 0, from.0, from.1);
        bot(&mut s, 0, 1, 1);
        let target = bot(&mut s, 1, 1, 0);
        // The spotter runs the team's program too, and hits the target
        // from beside it: only the attacker's log is read.
        let logs = run(&mut s, attacker, &[(0, ATTACK)], 8);
        assert!(sense::team_can_see(&mut s.world, TeamId(0), target));
        logs
    };
    // Level ground: the attack is begun.
    assert_eq!(fight(&[], long_arms(), (-1, 0)), ["hit"]);
    // A tile a level up between them: refused.
    assert_eq!(fight(&[(0, 0, 5)], long_arms(), (-1, 0)), ["refused"]);
    // A tile a level down between them is under the line.
    assert_eq!(fight(&[(0, 0, 3)], long_arms(), (-1, 0)), ["hit"]);

    // The range is the tables' and height does nothing to it. On a hill
    // two levels up a bot sees two tiles further and hits no further:
    // with the shipped range of one, a target two tiles off is refused.
    let shipped = Data::load().unwrap();
    assert_eq!(fight(&[(-1, 0, 6)], shipped, (-1, 0)), ["refused"]);
    // And from a hollow five levels down, where sight reaches one tile
    // and a range shortened the same way would reach none, the tile
    // beside it is still in reach.
    let shipped = Data::load().unwrap();
    assert_eq!(fight(&[(0, 0, 0), (1, 0, 5)], shipped, (0, 0)), ["hit"]);
    // At the edge of arms of three, over a dip that is under the line.
    let dip = [(-1, 0, 2), (0, 0, 3)];
    assert_eq!(fight(&dip, long_arms(), (-2, 0)), ["hit"]);
}

#[test]
fn an_attack_lands_only_if_the_line_is_still_clear() {
    // The ground between the two rises while the attack is under way:
    // the blocking is tested again on completion (`docs/02`, `attack`).
    let health_after = |raise: bool| {
        let mut s = field_at(4, &[], &[], long_arms());
        let attacker = bot(&mut s, 0, -1, 0);
        let target = bot(&mut s, 1, 1, 0);
        let full = s.world.machines[&target].health;
        let mut logs = Vec::new();
        let mut raised = false;
        for t in 0..10u64 {
            let cmds = if t == 0 {
                let mut c = deploy(0, ATTACK);
                c.tick = s.world.tick + 1;
                vec![c]
            } else {
                vec![]
            };
            s.step(&cmds, &[]);
            logs.extend(
                s.world.machines[&attacker]
                    .log
                    .iter()
                    .map(|l| l.text.clone()),
            );
            if raise && !raised && s.world.machines[&attacker].busy() == Some("attack") {
                let idx = s.world.tile_index(at(0, 0)).unwrap();
                s.world.set_height(idx, Height::new(5, 9).unwrap());
                raised = true;
            }
        }
        assert_eq!(raise, raised);
        // Begun either way, with a clear line at the time.
        assert_eq!(logs, ["hit"]);
        (full, s.world.machines[&target].health)
    };
    let (full, hit) = health_after(false);
    assert!(hit < full, "the attack landed");
    let (full, spared) = health_after(true);
    assert_eq!(spared, full, "the rise came between them first");
}

#[test]
fn hearing_is_unchanged_by_height() {
    // A listener of team 0 at (0, 0) and a sound of loudness 3 made on
    // the row east of it: a bot's hearing is 4, so it carries 7 tiles.
    let heard = |heights: &[(i32, i32, u8)], base: u8, x: i32| {
        let mut s = field_at(base, heights, &[], Data::load().unwrap());
        let listener = bot(&mut s, 0, 0, 0);
        s.world.sounds_prev = vec![Sound {
            cause: "dig".into(),
            pos: at(x, 0),
            loudness: 3,
            tick: 0,
        }];
        let heard = sense::team_hears(&s.world, TeamId(0)) == [0];
        (heard, sense::sees_tile(&s.world, listener, at(x, 0)))
    };
    // Level ground: heard at 7 and not at 8; seen at 6 and not at 7.
    assert_eq!(heard(&[], 4, 6), (true, true));
    assert_eq!(heard(&[], 4, 7), (true, false));
    assert_eq!(heard(&[], 4, 8), (false, false));
    // A ridge between them hides the tile and not the sound.
    let ridge = [(3, 0, 9)];
    assert_eq!(heard(&ridge, 4, 6), (true, false));
    assert_eq!(heard(&ridge, 4, 7), (true, false));
    assert_eq!(heard(&ridge, 4, 8), (false, false));
    // The listener on a hill sees further and hears no further.
    let hill = [(0, 0, 9)];
    assert_eq!(heard(&hill, 4, 8), (false, true));
    assert_eq!(heard(&hill, 4, 7), (true, true));
    // The listener at the bottom and the sound at the top, where sight
    // has no range at all: heard as on level ground.
    let pit = [(7, 0, 9), (8, 0, 9)];
    assert_eq!(heard(&pit, 0, 7), (true, false));
    assert_eq!(heard(&pit, 0, 8), (false, false));
}

#[test]
fn a_dug_tile_a_filled_tile_and_a_mined_rock_change_what_the_team_sees_at_the_next_pass() {
    // A bot at (0, 0). East of it a tile a level up hides the row behind
    // it; west of it a rock does.
    let mut s = field(&[(1, 0, 5)], &[(-1, 0)]);
    let id = bot(&mut s, 0, 0, 0);
    let main = format!("wait(3)\ndig(1, 0)\nwait(3)\nfill(1, 0)\nwait(3)\nmine(-1, 0)\n{IDLE}");
    let level = |s: &Sim| s.world.tile(at(1, 0)).unwrap().height.level();
    let rock = |s: &Sim| s.world.tile(at(-1, 0)).unwrap().terrain == Terrain::Rock;
    // What the team holds of the tiles behind each, tick by tick, beside
    // the ground as it stood when that tick's vision pass ran.
    let mut seen = Vec::new();
    for t in 0..40u64 {
        let cmds = if t == 0 {
            let mut c = deploy(0, &main);
            c.tick = s.world.tick + 1;
            vec![c]
        } else {
            vec![]
        };
        s.step(&cmds, &[]);
        seen.push((
            level(&s),
            rock(&s),
            state(&s, 0, 2, 0) == TileState::Visible,
            state(&s, 0, -2, 0) == TileState::Visible,
        ));
        // The pass of the very tick the ground changed in sees by the
        // new ground: the tile behind is seen exactly while nothing
        // stands above the line to it.
        let (level, rock, east, west) = seen[seen.len() - 1];
        assert_eq!(east, level <= 4, "tick {t}: the tile behind the rise");
        assert_eq!(west, !rock, "tick {t}: the tile behind the rock");
        // And the cache the pass read is the view by definition.
        let by_definition: BTreeSet<TilePos> = seen_by(&s, id)
            .into_iter()
            .chain(seen_by(&s, printer_of(&s, 0)))
            .collect();
        assert_eq!(
            sense::team_visible_tiles(&s.world, TeamId(0)),
            by_definition
        );
        let held: BTreeSet<TilePos> = s
            .world
            .map
            .positions()
            .into_iter()
            .filter(|p| state(&s, 0, p.x, p.y) == TileState::Visible)
            .collect();
        assert_eq!(held, by_definition, "tick {t}: the pass and the definition");
    }
    // All three happened, in order: hidden, dug into view, filled out of
    // it, and the rock mined.
    let mut story: Vec<(u8, bool, bool, bool)> = Vec::new();
    for s in seen {
        if story.last() != Some(&s) {
            story.push(s);
        }
    }
    assert_eq!(
        story,
        [
            (5, true, false, false),
            (4, true, true, false),
            (5, true, false, false),
            (5, false, false, true),
        ]
    );
    // A tile that left the view is remembered as it was last seen.
    assert_eq!(state(&s, 0, 2, 0), TileState::Remembered);
}

/// The team's printer.
fn printer_of(sim: &Sim, team: u32) -> EntityId {
    sim.world
        .machines
        .values()
        .find(|m| m.team == TeamId(team) && m.model == Model::Printer)
        .unwrap()
        .id
}
