//! The cost and limit tables: `data/language/costs.toml` and
//! `data/language/limits.toml`. The docs own what each row means; these
//! files own the numbers; and this module refuses either when they disagree
//! — a row that names a value with no key, or a key with no row, is a load
//! error, so the docs and the data cannot drift silently (T10).

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataError {
    pub file: &'static str,
    pub message: String,
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file, self.message)
    }
}

/// Every cost the evaluator charges, one field per row of
/// `docs/01-language/costs.md`, plus the game's own rows by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Costs {
    pub op_statement: u64,
    pub op_name: u64,
    pub op_literal: u64,
    pub op_operator: u64,
    pub op_attribute: u64,
    pub op_subscript: u64,
    pub op_call: u64,
    pub op_argument: u64,
    pub op_construct: u64,
    pub op_display: u64,
    pub op_iteration: u64,
    pub op_raise: u64,
    pub op_import: u64,
    pub op_pattern: u64,
    pub factor_traverse: u64,
    pub factor_copy: u64,
    pub factor_sort: u64,
    pub factor_char: u64,
    pub builtin_len: u64,
    pub builtin_range: u64,
    pub builtin_minmax: u64,
    pub builtin_sum: u64,
    pub builtin_abs: u64,
    pub builtin_sorted: u64,
    pub builtin_enumerate: u64,
    pub builtin_zip: u64,
    pub builtin_anyall: u64,
    pub builtin_convert: u64,
    pub builtin_construct: u64,
    pub builtin_isinstance: u64,
    pub builtin_round: u64,
    pub method_list_append: u64,
    pub method_list_extend: u64,
    pub method_list_insert: u64,
    pub method_list_pop: u64,
    pub method_list_search: u64,
    pub method_list_sort: u64,
    pub method_list_reverse: u64,
    pub method_list_clear: u64,
    pub method_list_copy: u64,
    pub method_dict_lookup: u64,
    pub method_dict_view: u64,
    pub method_dict_update: u64,
    pub method_dict_clear: u64,
    pub method_dict_copy: u64,
    pub method_set_single: u64,
    pub method_set_clear: u64,
    pub method_set_copy: u64,
    pub method_str_split: u64,
    pub method_str_join: u64,
    pub method_str_strip: u64,
    pub method_str_affix: u64,
    pub method_str_scan: u64,
    pub method_str_map: u64,
    pub method_str_test: u64,
    /// The `[game]` table: one key per game builtin, shaped by `docs/02`.
    pub game: BTreeMap<String, u64>,
}

/// Every limit, one field per row of `docs/01-language/execution.md`'s table
/// that names a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    pub budget_tick: u64,
    pub budget_hook_fault: u64,
    pub budget_hook_dying: u64,
    pub budget_hook_wait_per_tick: u64,
    pub depth_call: u64,
    pub depth_nesting: u64,
    pub size_collection: u64,
    pub size_live_values: u64,
    pub bundle_files: u64,
    pub bundle_file_bytes: u64,
}

/// Flatten a TOML document to `a.b.c → integer`, refusing anything that is
/// not a non-negative integer.
fn flatten(file: &'static str, text: &str) -> Result<BTreeMap<String, u64>, DataError> {
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| DataError {
        file,
        message: e.to_string(),
    })?;
    let mut out = BTreeMap::new();
    fn walk(
        file: &'static str,
        prefix: &str,
        v: &toml::Value,
        out: &mut BTreeMap<String, u64>,
    ) -> Result<(), DataError> {
        match v {
            toml::Value::Table(t) => {
                for (k, v) in t {
                    let key = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{prefix}.{k}")
                    };
                    walk(file, &key, v, out)?;
                }
                Ok(())
            }
            toml::Value::Integer(i) => {
                let u = u64::try_from(*i).map_err(|_| DataError {
                    file,
                    message: format!("`{prefix}` must be a non-negative integer"),
                })?;
                out.insert(prefix.to_string(), u);
                Ok(())
            }
            _ => Err(DataError {
                file,
                message: format!("`{prefix}` must be an integer"),
            }),
        }
    }
    walk(file, "", &toml::Value::Table(table), &mut out)?;
    Ok(out)
}

/// Take exactly the expected keys out of a flattened table, refusing a
/// missing one or a leftover one.
fn take_exact(
    file: &'static str,
    map: &mut BTreeMap<String, u64>,
    expected: &[&str],
) -> Result<Vec<u64>, DataError> {
    let mut out = Vec::with_capacity(expected.len());
    let mut missing = Vec::new();
    for key in expected {
        match map.remove(*key) {
            Some(v) => out.push(v),
            None => missing.push(*key),
        }
    }
    if !missing.is_empty() {
        return Err(DataError {
            file,
            message: format!(
                "the doc names rows with no key in the data: {}",
                missing.join(", ")
            ),
        });
    }
    Ok(out)
}

fn refuse_leftovers(file: &'static str, map: &BTreeMap<String, u64>) -> Result<(), DataError> {
    if map.is_empty() {
        Ok(())
    } else {
        let keys: Vec<&str> = map.keys().map(String::as_str).collect();
        Err(DataError {
            file,
            message: format!(
                "the data has keys with no row in the doc: {}",
                keys.join(", ")
            ),
        })
    }
}

const COST_KEYS: &[&str] = &[
    "op.statement",
    "op.name",
    "op.literal",
    "op.operator",
    "op.attribute",
    "op.subscript",
    "op.call",
    "op.argument",
    "op.construct",
    "op.display",
    "op.iteration",
    "op.raise",
    "op.import",
    "op.pattern",
    "factor.traverse",
    "factor.copy",
    "factor.sort",
    "factor.char",
    "builtin.len",
    "builtin.range",
    "builtin.minmax",
    "builtin.sum",
    "builtin.abs",
    "builtin.sorted",
    "builtin.enumerate",
    "builtin.zip",
    "builtin.anyall",
    "builtin.convert",
    "builtin.construct",
    "builtin.isinstance",
    "builtin.round",
    "method.list.append",
    "method.list.extend",
    "method.list.insert",
    "method.list.pop",
    "method.list.search",
    "method.list.sort",
    "method.list.reverse",
    "method.list.clear",
    "method.list.copy",
    "method.dict.lookup",
    "method.dict.view",
    "method.dict.update",
    "method.dict.clear",
    "method.dict.copy",
    "method.set.single",
    "method.set.clear",
    "method.set.copy",
    "method.str.split",
    "method.str.join",
    "method.str.strip",
    "method.str.affix",
    "method.str.scan",
    "method.str.map",
    "method.str.test",
];

const LIMIT_KEYS: &[&str] = &[
    "budget.tick",
    "budget.hook.fault",
    "budget.hook.dying",
    "budget.hook.wait_per_tick",
    "depth.call",
    "depth.nesting",
    "size.collection",
    "size.live_values",
    "bundle.files",
    "bundle.file_bytes",
];

impl Costs {
    pub fn parse(text: &str) -> Result<Costs, DataError> {
        const FILE: &str = "data/language/costs.toml";
        let mut map = flatten(FILE, text)?;
        let game: BTreeMap<String, u64> = map
            .iter()
            .filter_map(|(k, v)| k.strip_prefix("game.").map(|g| (g.to_string(), *v)))
            .collect();
        map.retain(|k, _| !k.starts_with("game."));
        let v = take_exact(FILE, &mut map, COST_KEYS)?;
        refuse_leftovers(FILE, &map)?;
        let mut it = v.into_iter();
        let mut next = || it.next().unwrap_or(0);
        Ok(Costs {
            op_statement: next(),
            op_name: next(),
            op_literal: next(),
            op_operator: next(),
            op_attribute: next(),
            op_subscript: next(),
            op_call: next(),
            op_argument: next(),
            op_construct: next(),
            op_display: next(),
            op_iteration: next(),
            op_raise: next(),
            op_import: next(),
            op_pattern: next(),
            factor_traverse: next(),
            factor_copy: next(),
            factor_sort: next(),
            factor_char: next(),
            builtin_len: next(),
            builtin_range: next(),
            builtin_minmax: next(),
            builtin_sum: next(),
            builtin_abs: next(),
            builtin_sorted: next(),
            builtin_enumerate: next(),
            builtin_zip: next(),
            builtin_anyall: next(),
            builtin_convert: next(),
            builtin_construct: next(),
            builtin_isinstance: next(),
            builtin_round: next(),
            method_list_append: next(),
            method_list_extend: next(),
            method_list_insert: next(),
            method_list_pop: next(),
            method_list_search: next(),
            method_list_sort: next(),
            method_list_reverse: next(),
            method_list_clear: next(),
            method_list_copy: next(),
            method_dict_lookup: next(),
            method_dict_view: next(),
            method_dict_update: next(),
            method_dict_clear: next(),
            method_dict_copy: next(),
            method_set_single: next(),
            method_set_clear: next(),
            method_set_copy: next(),
            method_str_split: next(),
            method_str_join: next(),
            method_str_strip: next(),
            method_str_affix: next(),
            method_str_scan: next(),
            method_str_map: next(),
            method_str_test: next(),
            game,
        })
    }

    /// The cost of a game builtin, or `None` if it has no row — which the
    /// caller treats as a load error, not as free.
    pub fn game_cost(&self, name: &str) -> Option<u64> {
        self.game.get(name).copied()
    }
}

impl Limits {
    pub fn parse(text: &str) -> Result<Limits, DataError> {
        const FILE: &str = "data/language/limits.toml";
        let mut map = flatten(FILE, text)?;
        let v = take_exact(FILE, &mut map, LIMIT_KEYS)?;
        refuse_leftovers(FILE, &map)?;
        let mut it = v.into_iter();
        let mut next = || it.next().unwrap_or(0);
        Ok(Limits {
            budget_tick: next(),
            budget_hook_fault: next(),
            budget_hook_dying: next(),
            budget_hook_wait_per_tick: next(),
            depth_call: next(),
            depth_nesting: next(),
            size_collection: next(),
            size_live_values: next(),
            bundle_files: next(),
            bundle_file_bytes: next(),
        })
    }
}

/// The repository's own data files, for tests and the headless driver.
pub const COSTS_TOML: &str = include_str!("../../../data/language/costs.toml");
pub const LIMITS_TOML: &str = include_str!("../../../data/language/limits.toml");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_repository_data_loads() {
        let c = Costs::parse(COSTS_TOML).expect("costs");
        assert_eq!(c.op_statement, 1);
        assert_eq!(c.factor_sort, 2);
        assert!(c.game_cost("see").is_some());
        let l = Limits::parse(LIMITS_TOML).expect("limits");
        assert!(l.budget_tick > 0);
        assert!(l.depth_nesting > 0);
    }

    /// `docs/01-language/costs.md` prices every operation by naming a key
    /// of `costs.toml` in backticks — `op.operator`, `factor.traverse`,
    /// `method.list.append`. A key the doc never names is a price nobody
    /// can read; a name the table lacks is a price nobody pays (T10: the
    /// docs and the data cannot drift silently). Game rows are `docs/02`'s
    /// and are checked by name there.
    #[test]
    fn every_cost_key_is_named_by_the_doc_and_vice_versa() {
        const DOC: &str = include_str!("../../../docs/01-language/costs.md");
        const MACHINES: &str = include_str!("../../../docs/02-machines.md");
        let map = flatten("costs.toml", COSTS_TOML).expect("costs");
        let mut named: Vec<String> = Vec::new();
        // A backticked chunk is a formula — `op.operator + n × factor.copy`
        // — so the keys are its tokens, not the chunk.
        for chunk in DOC.split('`').skip(1).step_by(2) {
            for token in chunk.split(|c: char| !(c.is_ascii_lowercase() || c == '.' || c == '_')) {
                // `builtin.name` and `method.type.name` are the doc's
                // schema for a row, not rows; `op.name` is a row.
                const SCHEMA: [&str; 3] = ["factor.name", "builtin.name", "method.type.name"];
                let is_key = token.split_once('.').is_some_and(|(head, _)| {
                    matches!(head, "op" | "factor" | "builtin" | "method")
                }) && !SCHEMA.contains(&token);
                if is_key {
                    named.push(token.to_string());
                }
            }
        }
        named.sort();
        named.dedup();
        let keys: Vec<&String> = map.keys().filter(|k| !k.starts_with("game.")).collect();
        let unnamed: Vec<&&String> = keys.iter().filter(|k| !named.contains(k)).collect();
        assert!(
            unnamed.is_empty(),
            "costs.toml keys costs.md never names: {unnamed:?}"
        );
        let unpriced: Vec<&String> = named.iter().filter(|n| !map.contains_key(*n)).collect();
        assert!(
            unpriced.is_empty(),
            "costs.md names keys costs.toml lacks: {unpriced:?}"
        );
        let costs = Costs::parse(COSTS_TOML).expect("costs");
        let unknown: Vec<&String> = costs
            .game
            .keys()
            .filter(|name| !MACHINES.contains(&format!("`{name}(")))
            .collect();
        assert!(
            unknown.is_empty(),
            "[game] rows docs/02 has no builtin for: {unknown:?}"
        );
    }

    #[test]
    fn a_row_with_no_key_or_a_key_with_no_row_is_a_load_error() {
        let missing = COSTS_TOML.replace("sort = 2", "");
        let e = Costs::parse(&missing).expect_err("missing key");
        assert!(e.message.contains("factor.sort"), "{e}");
        let extra = COSTS_TOML.replace("[op]\n", "[op]\nbogus = 1\n");
        let e = Costs::parse(&extra).expect_err("extra key");
        assert!(e.message.contains("op.bogus"), "{e}");
        let negative = LIMITS_TOML.replace("tick = 200", "tick = -1");
        assert!(Limits::parse(&negative).is_err());
        let extra = LIMITS_TOML.replace("[depth]\n", "[depth]\nstack = 9\n");
        assert!(Limits::parse(&extra).is_err());
    }
}
