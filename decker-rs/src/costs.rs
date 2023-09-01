use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use serde::Deserialize;

use crate::{short_value, MAXCOINCOST};

// Could have possibly used a tuple struct but
// I don't want people to need to know meaning of indices
// Could possibly have made each of these Option<>
//  and then make callers check if they exist
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug, Deserialize)]
pub struct Cost {
    coin: Option<i8>,
    potion: Option<i8>,
    debt: Option<i8>,
}

pub type CostSet = HashSet<Cost>;

// The c++ implementation tried to const everything in sight
//  so default to non-mutable is hopefully less of a problem
impl Cost {
    // constructor overloading with no missing param support is not clean
    // Can't have overloaded names, ... but can do macros ...
    //but can't put them in impl or traits
    // so would need to put them outside and possibly scope it inside something
    //  but would then need to give it a bigger name
    // I could do the single version if I passed in multiple Options but
    // that makes creating new instance more clunky and verbose

    pub fn new_s(coin: i8) -> Cost {
        Cost {
            coin: Some(coin),
            potion: None,
            debt: None,
        }
    }

    pub fn new(coin: Option<i8>, potion: Option<i8>, debt: Option<i8>) -> Cost {
        Cost { coin, potion, debt }
    }

    pub(crate) fn get_string(&self) -> String {
        format!(
            "({},{},{})",
            self.coin.map_or(String::from(""), |c| c.to_string()),
            self.potion.map_or(String::from(""), |p| format!("{}D", p)),
            self.debt.map_or(String::from(""), |d| format!("{}D", d))
        )
    }

    pub(crate) fn has_debt(&self) -> bool {
        self.debt.is_some()
    }
    fn has_coin(&self) -> bool {
        self.coin.is_some()
    }
    pub(crate) fn has_potion(&self) -> bool {
        self.potion.is_some()
    }
    fn is_coin_only(&self) -> bool {
        self.potion.is_none() && self.debt.is_none()
    }

    // if we were really rusting this maybe this should be an Option
    fn get_coin(&self) -> Option<i8> {
        self.coin
    }

    fn get_rel_cost(&self, delta: i8) -> Cost {
        let mut new_coin = self.coin.unwrap_or(-1) + delta;
        if new_coin < 0 {
            new_coin = 0;
        }
        Cost {
            coin: Some(new_coin),
            ..*self
        }
    }

    pub fn intersects(cs1: &CostSet, cs2: &CostSet) -> bool {
        if cs1.intersection(cs2).next().is_some() {
            return true;
        }
        false
    }
}

// This is a bit tricky
// What I had in c++ was operator==(CostTarget)
//   which then examined type
// 1. Because pub trait CostTarget : PartialEq  => Self as a parameter type
//      which apparently prevents CostTargets being objects
// 2. If I explicitly parameterise <CostTarget> then that
//      would want a dyn
// 3. If I paramterise by <dyn CostTarget> then there is a cycle
//     in the super traits of CostTarget
//   If I can't solve this, the question then becomes ...
//     Who uses the == relation?   My code? Needed by containers?
//
//  For now I'll try to drop the equality operator
//    and see what goes wrong
pub trait CostTarget: std::fmt::Debug {
    // Do I need to return an object back?
    fn add_votes(&self, current_costs: &CostSet, votes: &mut CostVotes) -> bool;
    fn str_rep(&self) -> &String;
}

// This could be a problem:
//TargetSet=std::unordered_set<const CostTarget*, TargetHasher, TargetEq>;
// Two possibilities:
//    either the CostTargets exist solely within a single set
//    or it is meaningfully shared
// Let's hope it's just an efficieny issue for now
// There are references to smart pointers in the doco so maybe I can investigate that
//
//
//   More problems here - to put Boxed CostTarget into a HashSet and allow that
//    set to be copied needs CostTarget To have Clone
//    but adding : Clone to CostTarget causes objections because:
//      because it requires `Self: Sized`
//   Backing out for a tick ...   the problem only comes up if i try to make a copy
//   of a targetset.
//     I think the problem is that copying the set requires copy/cloning the Box which
//      holds the target and it doesn't know how to do that.
//     I could try to get around the issue by writing my own implementation
//      having Cost target implement a copy method which I could make calls to clone
//      sets if I actually need to do it.
//
//    The question is, do I actually need to do that?
//    Can I create the set and move it into the target?
//type TargetSet = BTreeSet<Box<dyn CostTarget>>;

// Attaching the necessary traits for Tree or HashSet to the trait turns out to be
// _difficult_.      I've decided to just make a vector and do duplicate removal
//  manually.

#[derive(Debug)]
struct CostTargetHelper {
    matches_required: i16,
    unmet_weight: i16,
    met_weight: i16,
    cache_string: String,
}

impl CostTargetHelper {
    fn new(matches_needed: i16, mut unmet_w: i16, mut met_w: i16, cs: String) -> CostTargetHelper {
        if unmet_w < met_w {
            std::mem::swap(&mut unmet_w, &mut met_w);
        };
        CostTargetHelper {
            matches_required: matches_needed,
            unmet_weight: unmet_w,
            met_weight: met_w,
            cache_string: cs,
        }
    }
}

#[derive(Debug)]
struct CostRelative {
    helper: CostTargetHelper,
    cost_delta: i8,
    no_less: bool,
}

impl CostTarget for CostRelative {
    fn str_rep(&self) -> &String {
        &self.helper.cache_string
    }

    // What consitutes matching here. People might like to use this to get a more expensive
    // card but technically any cost in range will do
    // also remember delta could be negative
    fn add_votes(&self, current_costs: &CostSet, votes: &mut CostVotes) -> bool {
        let mut matched_count = 0;
        for c in current_costs {
            let adj_cost = c.get_rel_cost(self.cost_delta);
            if current_costs.get(&adj_cost).is_some() {
                matched_count += 1
            }
        }

        // not much thought went into these particular values
        // other than to give a bit of a boost to costs above the current one
        let boost =
            (self.helper.unmet_weight - self.helper.met_weight) as f32 / self.cost_delta as f32;
        let weight = self.helper.met_weight as f32 / current_costs.len() as f32;
        if self.cost_delta < 0 {
            for c in current_costs {
                let Some(coin) = c.get_coin() else { continue };

                // don't let cost drop below zero
                if coin < -self.cost_delta {
                    continue;
                }

                // Must produce a valid cost
                let mut target = c.get_rel_cost(self.cost_delta);
                // FIXME: Doesn't add anything if self.no_less is `true`
                if !self.no_less {
                    // NOTE: This was a bug! `target.get_rel_cost(-1)` will always produce a cost >= 0
                    // Costs in range (0..(coin-delta))
                    while target.get_coin().unwrap() > 0 {
                        votes.add_vote(&target, weight);
                        target = target.get_rel_cost(-1);
                    }
                    votes.add_vote(&target, weight);
                }
            }
        } else {
            // delta >=0
            for c in current_costs {
                if !c.has_coin() {
                    continue;
                }
                // Must produce a valid cost
                let mut target = c.get_rel_cost(self.cost_delta);
                if self.no_less {
                    votes.add_vote(&target, weight + boost);
                } else {
                    // Add delta costs above
                    // Costs in range (coin..(coin+delta))
                    while target != *c {
                        votes.add_vote(&target, weight + boost);
                        target = target.get_rel_cost(-1);
                    }
                    // Add max of delta costs below, but keep coin positive
                    let mut i = 0;
                    // Costs in range (coin..max(0,coin-delta))
                    while (i < self.cost_delta) && (target.get_coin().unwrap() > 0) {
                        votes.add_vote(&target, weight);
                        target = target.get_rel_cost(-1);
                        i += 1;
                    }
                    // Using `get_rel_cost` can't give a cost less than zero
                    votes.add_vote(&target, weight);
                }
            }
        }
        matched_count < self.helper.matches_required
    }
}

impl CostRelative {
    fn new(matches_needed: i16, unmet_w: i16, met_w: i16, delta: i8, strict: bool) -> CostRelative {
        CostRelative {
            helper: CostTargetHelper::new(
                matches_needed,
                unmet_w,
                met_w,
                format!("CR{}{}", strict, delta),
            ),
            cost_delta: delta,
            no_less: strict,
        }
    }
}

#[derive(Debug)]
struct CostUpto {
    helper: CostTargetHelper,
    limit: i8,
}

impl CostUpto {
    fn new(matches_needed: i16, unmet_w: i16, met_w: i16, upper: i8) -> CostUpto {
        CostUpto {
            helper: CostTargetHelper::new(matches_needed, unmet_w, met_w, format!("UT{}", upper)),
            limit: upper,
        }
    }
}

impl CostTarget for CostUpto {
    fn str_rep(&self) -> &String {
        &self.helper.cache_string
    }

    // 1. Count how many "costs" match the criteria
    // 2. Did we find enough "costs"?
    // 3. Choose the weight based on if we found enough costs
    // 4. Spread weight uniformly across "limit" number of new costs
    // 5. Return whether we found enough costs or not
    fn add_votes(&self, current_costs: &CostSet, votes: &mut CostVotes) -> bool {
        let mut match_count = 0;
        for c in current_costs {
            if c.is_coin_only() && c.get_coin().unwrap_or(-1) <= self.limit {
                match_count += 1;
            };
        }
        let weight = (if match_count >= self.helper.matches_required {
            self.helper.met_weight
        } else {
            self.helper.unmet_weight
        } as f32)
            / (self.limit as f32);
        for i in 1..=self.limit {
            votes.add_vote(&Cost::new_s(i), weight);
        }
        match_count < self.helper.matches_required
    }
}

#[derive(Debug)]
struct CostInSet {
    helper: CostTargetHelper,
    costs: CostSet,
}

impl CostInSet {
    fn new(matches_needed: i16, unmet_w: i16, met_w: i16, s: CostSet) -> CostInSet {
        CostInSet {
            helper: CostTargetHelper::new(matches_needed, unmet_w, met_w, {
                let mut res = "IS".to_string();
                let mut items: Vec<String> = vec![];
                for c in &s {
                    items.push(c.get_string())
                }
                // Because I'm (hopefully temporarily) using this for EQ
                // purposes, I can't depend on HashSet to have consistant order
                items.sort();
                for c in &items {
                    res = format!("{}{}", res, c);
                }
                res
            }),
            costs: s,
        }
    }
}

impl CostTarget for CostInSet {
    fn str_rep(&self) -> &String {
        &self.helper.cache_string
    }

    fn add_votes(&self, current_costs: &CostSet, votes: &mut CostVotes) -> bool {
        let mut matched_count = 0;
        for c in current_costs {
            if self.costs.get(c).is_some() {
                matched_count += 1
            }
        }
        let weight = if matched_count >= self.helper.matches_required {
            self.helper.met_weight as f32 / self.costs.len() as f32
        } else {
            self.helper.unmet_weight as f32 / self.costs.len() as f32
        };
        for c in &self.costs {
            votes.add_vote(c, weight);
        }
        matched_count < self.helper.matches_required
    }
}

// This class needed a costcompare functional for the map
// Do I actually need ordering here?
//
//   CostVotes takes in shared_ptr params - that smells like a problem for rust
//     the question is what did I actually use them for?
//     OK, the shared pointer is because I didn't want to copy the cost sets for
//     cards all the time
//       So I could copy set each time ....   Can I do something with lifetimes?
//           - tie lifetime of the CostTarget to the card
//             Costs are already immutable so concurrent changes are not a problem
pub struct CostVotes {
    available_costs: HashSet<Cost>,
    votes: std::collections::HashMap<Cost, f32>,
}

impl CostVotes {
    pub fn new(legal_costs: CostSet) -> CostVotes {
        CostVotes {
            available_costs: legal_costs,
            votes: HashMap::new(),
        }
    }
    pub fn add_vote(&mut self, c: &Cost, diff: f32) {
        if self.available_costs.contains(c) {
            let t = self.votes.entry(*c).or_insert(0.0);
            *t += diff;
        }
    }

    pub fn get_max_weighted(&self, max_cost: &mut CostSet, threshold: f32, tolerance: f32) -> bool {
        // we'll reject any votes below zero
        let mut max: f32 = 0.0;
        for v in self.votes.values() {
            if *v > max {
                // yes this check is not completely well defined
                max = *v; // but "close to max" will do here
            }
        }
        if max < threshold {
            return false;
        }
        for (k, v) in &self.votes {
            if max - v <= tolerance {
                max_cost.insert(*k);
            };
        }
        max > 0.0
    }
}

// Used
// ----
// <= (+/-), relative (0..limit)
// <=, absolute (0..limit)
// >=, absolute (limit..)
// = (+/-), relative iter::once(limit)
// in, (lower..upper)

// Un-used
// -------
// >= (+/-), relative (limit..)
// =, absolute iter::once(limit)
enum Comparison {
    LessThan(i8),
    GreaterThan(i8),
    EqualTo(i8),
}

pub fn decode_cost(s: &str) -> Option<Box<dyn CostTarget>> {
    let matches_required = 6;
    let unmet_weight = 3;
    let met_weight = 1;
    let upto_matches = 3;
    let cost_bound = 30;

    // If it doesn't start with cost, we're in trouble!
    let s = s.strip_prefix("cost")?;

    // Can combine +/- using i32::from_str_radix
    // Handle `cost_in` range first
    if let Some(range) = s.strip_prefix("_in") {
        // If we are missing brackets, return None and fail early!
        let range = range.strip_prefix('(')?.strip_suffix(')')?;
        let (lower, upper) = range.split_once('.')?;
        // Parse both upper and lower, requires positive except accepts i8?
        let lower: i8 = lower.parse::<u8>().ok()? as i8;
        let upper: i8 = upper.parse::<u8>().ok()? as i8;
        let cs = CostSet::from_iter((lower..=upper).map(Cost::new_s));
        // Create final struct
        return Some(Box::new(CostInSet::new(
            upto_matches,
            unmet_weight,
            met_weight,
            cs,
        )));
    }

    let (comp, str_val) = s.split_once('=')?;
    let relative = str_val.starts_with(['+', '-']);
    let val: i8 = str_val.parse().ok()?;

    if val.abs() > cost_bound || val == 0 {
        return None;
    }

    let comp = match comp {
        "<" => Comparison::LessThan(val),
        ">" => Comparison::GreaterThan(val),
        "" => Comparison::EqualTo(val),
        _ => return None,
    };

    let coin_cost = 10;
    let limit = if relative { coin_cost + val } else { val };
    let (upper, lower) = match comp {
        Comparison::LessThan(_) => (0, limit),
        Comparison::GreaterThan(_) => (limit, MAXCOINCOST),
        Comparison::EqualTo(_) => (limit, limit),
    };

    for val in lower..=upper {
        Cost::new_s(val);
    }

    if let Some(stripped) = s.strip_prefix("<=+") {
        let value = short_value(stripped);
        if value <= 0 || value > cost_bound {
            return None;
        }
        return Some(Box::new(CostRelative::new(
            matches_required,
            unmet_weight,
            met_weight,
            value,
            false,
        )));
    } else if let Some(stripped) = s.strip_prefix("<=-") {
        // FIXME: This case doesn't exist and is incorrect! Should use '-value'
        let value = short_value(stripped);
        if value <= 0 || value > cost_bound {
            return None;
        }
        return Some(Box::new(CostRelative::new(
            matches_required,
            unmet_weight,
            met_weight,
            value,
            false,
        )));
    } else if let Some(stripped) = s.strip_prefix("<=") {
        let value = short_value(stripped);
        if value <= 0 || value > cost_bound {
            return None;
        }
        return Some(Box::new(CostUpto::new(
            upto_matches,
            unmet_weight,
            met_weight,
            value,
        )));
    } else if let Some(stripped) = s.strip_prefix("=+") {
        let value = short_value(stripped);
        if value <= 0 || value > cost_bound {
            return None;
        }
        return Some(Box::new(CostRelative::new(
            matches_required,
            unmet_weight,
            met_weight,
            value,
            true,
        )));
    } else if let Some(stripped) = s.strip_prefix("=-") {
        let value = short_value(stripped);
        if value <= 0 || value > cost_bound {
            return None;
        }
        return Some(Box::new(CostRelative::new(
            matches_required,
            unmet_weight,
            met_weight,
            -value,
            true,
        )));
    } else if let Some(stripped) = s.strip_prefix(">=") {
        let value = short_value(stripped);
        if value <= 0 || value > cost_bound {
            return None;
        }
        let mut cs = CostSet::new();
        for v in value..=MAXCOINCOST {
            cs.insert(Cost::new_s(v));
        }
        return Some(Box::new(CostInSet::new(
            upto_matches,
            unmet_weight,
            met_weight,
            cs,
        )));
    }
    None
}
