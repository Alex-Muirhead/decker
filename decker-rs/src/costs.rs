use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::{short_value, MAXCOINCOST};

// Could have possibly used a tuple struct but
// I don't want people to need to know meaning of indices
// Could possibly have made each of these Option<>
//  and then make callers check if they exist
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct Cost {
    coin: i8,
    potion: i8,
    debt: i8,
}

pub type CostSet = HashSet<Cost>;

// The c++ implementation tried to const everything in sight
//  so default to non-mutable is hopefully less of a problem
impl Cost {
    const NOCOST: i8 = -1;

    // constructor overloading with no missing param support is not clean
    // Can't have overloaded names, ... but can do macros ...
    //but can't put them in impl or traits
    // so would need to put them outside and possibly scope it inside something
    //  but would then need to give it a bigger name
    // I could do the single version if I passed in multiple Options but
    // that makes creating new instance more clunky and verbose

    pub fn new_s(coin: i8) -> Cost {
        Cost {
            coin,
            potion: Cost::NOCOST,
            debt: Cost::NOCOST,
        }
    }

    pub fn new(
        coin: i8,
        has_coin: bool,
        potion: i8,
        has_potion: bool,
        debt: i8,
        has_debt: bool,
    ) -> Cost {
        Cost {
            coin: if has_coin { coin } else { Cost::NOCOST },
            potion: if has_potion { potion } else { Cost::NOCOST },
            debt: if has_debt { debt } else { Cost::NOCOST },
        }
    }

    pub fn valid(&self) -> bool {
        !((self.coin == Cost::NOCOST)
            && (self.potion == Cost::NOCOST)
            && (self.debt == Cost::NOCOST))
    }

    pub(crate) fn get_string(&self) -> String {
        format!(
            "({},{},{})",
            if self.has_coin() {
                self.coin.to_string()
            } else {
                String::from("")
            },
            if self.has_potion() {
                format!("{}P", self.potion)
            } else {
                String::from("")
            },
            if self.has_debt() {
                format!("{}D", self.debt)
            } else {
                String::from("")
            }
        )
    }

    pub(crate) fn has_debt(&self) -> bool {
        self.debt != Cost::NOCOST
    }
    fn has_coin(&self) -> bool {
        self.coin != Cost::NOCOST
    }
    pub(crate) fn has_potion(&self) -> bool {
        self.potion != Cost::NOCOST
    }
    fn is_coin_only(&self) -> bool {
        self.potion == Cost::NOCOST && self.debt == Cost::NOCOST
    }

    // if we were really rusting this maybe this should be an Option
    fn get_coin(&self) -> i8 {
        self.coin
    }

    fn get_rel_cost(&self, delta: i8) -> Cost {
        let mut new_coin = self.coin + delta;
        if new_coin < 0 {
            new_coin = 0;
        }
        Cost {
            coin: new_coin,
            potion: self.potion,
            debt: self.debt,
        }
    }

    pub fn intersects(cs1: &CostSet, cs2: &CostSet) -> bool {
        if cs1.intersection(cs2).next().is_some() {
            return true;
        }
        false
    }

    pub fn dummy() -> Cost {
        Cost {
            coin: Cost::NOCOST,
            potion: Cost::NOCOST,
            debt: Cost::NOCOST,
        }
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
pub trait CostTarget {
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

pub type TargetPtr = Rc<dyn CostTarget>;
pub type Targets = Vec<TargetPtr>;

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
                if !c.has_coin() {
                    // costs without coin components can't do coin relative costs
                    continue;
                }
                if c.get_coin() < -self.cost_delta
                // don't let cost drop below zero
                {
                    continue;
                }
                let mut target = c.get_rel_cost(self.cost_delta);
                if !self.no_less {
                    while target.get_coin() >= 0 {
                        votes.add_vote(&target, weight);
                        target = target.get_rel_cost(-1);
                    }
                }
            }
        } else {
            // delta >=0
            for c in current_costs {
                if !c.has_coin() {
                    continue;
                }
                let mut target = c.get_rel_cost(self.cost_delta);
                if self.no_less {
                    votes.add_vote(&target, weight + boost);
                } else {
                    while target != *c {
                        votes.add_vote(&target, weight + boost);
                        target = target.get_rel_cost(-1);
                    }
                    let mut i = 0;
                    while (i < self.cost_delta) && (target.get_coin() > 0) {
                        votes.add_vote(&target, weight);
                        target = target.get_rel_cost(-1);
                        i += 1;
                    }
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

    fn add_votes(&self, current_costs: &CostSet, votes: &mut CostVotes) -> bool {
        let mut match_count = 0;
        for c in current_costs {
            if c.is_coin_only() && c.get_coin() <= self.limit {
                match_count += 1;
            };
        }
        let weight = if match_count >= self.helper.matches_required {
            (self.helper.met_weight as f32) / (self.limit as f32)
        } else {
            (self.helper.unmet_weight as f32) / (self.limit as f32)
        };
        for i in 1..=self.limit {
            votes.add_vote(&Cost::new_s(i), weight);
        }
        match_count < self.helper.matches_required
    }
}

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
    available_costs: CostSet,
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

pub fn decode_cost(s: &str) -> Option<TargetPtr> {
    let matches_required = 6;
    let unmet_weight = 3;
    let met_weight = 1;
    let upto_matches = 3;
    let cost_bound = 30;
    if let Some(stripped) = s.strip_prefix("cost<=+") {
        let value = short_value(stripped);
        if value <= 0 || value > cost_bound {
            return None;
        }
        return Some(Rc::new(CostRelative::new(
            matches_required,
            unmet_weight,
            met_weight,
            value,
            false,
        )));
    } else if let Some(stripped) = s.strip_prefix("cost<=-") {
        let value = short_value(stripped);
        if value <= 0 || value > cost_bound {
            return None;
        }
        return Some(Rc::new(CostRelative::new(
            matches_required,
            unmet_weight,
            met_weight,
            value,
            false,
        )));
    } else if let Some(stripped) = s.strip_prefix("cost<=") {
        let value = short_value(stripped);
        if value <= 0 || value > cost_bound {
            return None;
        }
        return Some(Rc::new(CostUpto::new(
            upto_matches,
            unmet_weight,
            met_weight,
            value,
        )));
    } else if let Some(stripped) = s.strip_prefix("cost=+") {
        let value = short_value(stripped);
        if value <= 0 || value > cost_bound {
            return None;
        }
        return Some(Rc::new(CostRelative::new(
            matches_required,
            unmet_weight,
            met_weight,
            value,
            true,
        )));
    } else if let Some(stripped) = s.strip_prefix("cost=-") {
        let value = short_value(stripped);
        if value <= 0 || value > cost_bound {
            return None;
        }
        return Some(Rc::new(CostRelative::new(
            matches_required,
            unmet_weight,
            met_weight,
            -value,
            true,
        )));
    } else if let Some(stripped) = s.strip_prefix("cost>=") {
        let value = short_value(stripped);
        if value <= 0 || value > cost_bound {
            return None;
        }
        let mut cs = CostSet::new();
        for v in value..=MAXCOINCOST {
            cs.insert(Cost::new_s(v));
        }
        return Some(Rc::new(CostInSet::new(
            upto_matches,
            unmet_weight,
            met_weight,
            cs,
        )));
    } else if s.starts_with("cost_in(") {
        let sep = match s.find('.') {
            None => {
                return None;
            }
            Some(v) => v,
        };
        let lower = short_value(&s["cost_in(".len()..sep]);
        let upper = short_value(&s[sep + ".".len()..s.len() - ")".len()]);
        if (lower <= 0) || (upper <= 0) {
            return None;
        }
        let mut cs = CostSet::new();
        for v in lower..=upper {
            cs.insert(Cost::new_s(v));
        }
        return Some(Rc::new(CostInSet::new(
            upto_matches,
            unmet_weight,
            met_weight,
            cs,
        )));
    }
    None
}
