use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;

use std::collections::hash_map::Entry::Occupied;
use std::convert::TryInto;

use std::borrow::Borrow;

use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;

use std::hash::{Hash, Hasher};

use ConsResult::*;

use std::cmp::Ordering;
use Ordering::*;

use std::convert::TryFrom;

use std::cell::RefCell;
use std::process::exit;
use std::rc::Rc;

static MAXCOINCOST: i8 = 11;

// Just in case I need to go to 16 (Don't know why I would need that)
type Short = i8;
type UShort = u8;

// so we aren't depending on usize
type Unsigned = u64;

const MANY: Unsigned = 5000;

fn get_legal_options() -> HashMap<String, String> {
    let mut res: HashMap<String, String> = HashMap::new();
    res.insert(
        String::from("--seed"),
        String::from("Seed for random number generator. 0 will use a default value."),
    );
    res.insert(
        String::from("--badrand"),
        String::from("Use bad (but cross platform) random number generator"),
    );
    res.insert(
        String::from("--boxes"),
        String::from("Which boxes to include in the collection."),
    );
    res.insert(
        String::from("--groups"),
        String::from("Which groups to include in the collection."),
    );
    res.insert(
        String::from("--boxfile"),
        String::from("Filename listing boxes and which groups they contain"),
    );
    res.insert(
        String::from("--cardfile"),
        String::from("Filename listing all cards."),
    );
    res.insert(
        String::from("--help"),
        String::from("List command options."),
    );
    res.insert(
        String::from("--list"),
        String::from("Dump contents of collection and exit."),
    );
    res.insert(
        String::from("--landscape-count"),
        String::from("How many landscape cards to include (does not include artefacts etc)."),
    );
    res.insert(
        String::from("--why"),
        String::from("Explain why cards were added."),
    );
    res.insert(
        String::from("--no-validate"),
        String::from("Do not validate collection."),
    );
    res.insert(
        String::from("--exclude"),
        String::from("Do not allow any of these cards."),
    );
    res.insert(
        String::from("--include"),
        String::from("This card must be in the selection."),
    );
    res.insert(
        String::from("--info"),
        String::from("Show info about selected cards."),
    );
    res.insert(
        String::from("--no-attack-react"),
        String::from("Disable automatic adding reacts to attacks."),
    );
    res.insert(
        String::from("--no-anti-cursor"),
        String::from("Disable automatic adding of trash cards if cards give curses."),
    );
    res.insert(
        String::from("--max-cost-repeat"),
        String::from("Set the maximum number of times a cost can occur."),
    );
    res.insert(String::from("--min-type"), String::from("eg --min-type=Treasure:5 means that the selection will can contain at least 5 treasures."));
    res.insert(String::from("--max-type"), String::from("eg --max-type=Treasure:5 means that the selection will can contain at most 5 treasures."));
    res.insert(String::from("--max-prefixes"), String::from("Most prefixes (groups and related groups) which can be included. Eg: Cornucopia would also allow Cornucopia-prizes."));
    res
}

// Could have possibly used a tuple struct but
// I don't want people to need to know meaning of indices
// Could possibly have made each of these Option<>
//  and then make callers check if they exist
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct Cost {
    coin: Short,
    potion: Short,
    debt: Short,
}

type CostSet = HashSet<Cost>;

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

    fn new_s(coin: i8) -> Cost {
        Cost {
            coin: coin,
            potion: Cost::NOCOST,
            debt: Cost::NOCOST,
        }
    }

    fn new(
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

    fn valid(&self) -> bool {
        !((self.coin == Cost::NOCOST)
            && (self.potion == Cost::NOCOST)
            && (self.debt == Cost::NOCOST))
    }

    fn get_string(&self) -> String {
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

    fn has_debt(&self) -> bool {
        self.debt != Cost::NOCOST
    }
    fn has_coin(&self) -> bool {
        self.coin != Cost::NOCOST
    }
    fn has_potion(&self) -> bool {
        self.potion != Cost::NOCOST
    }
    fn is_coin_only(&self) -> bool {
        self.potion == Cost::NOCOST && self.debt == Cost::NOCOST
    }

    // if we were really rusting this maybe this should be an Option
    fn get_coin(&self) -> Short {
        self.coin
    }

    fn get_rel_cost(&self, delta: Short) -> Cost {
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

    fn intersects(cs1: &CostSet, cs2: &CostSet) -> bool {
        for _ in cs1.intersection(&cs2) {
            return true;
        }
        false
    }

    fn dummy() -> Cost {
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
    fn add_votes(&self, &CostSet, &mut CostVotes) -> bool;
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

type TargetPtr = Rc<dyn CostTarget>;
type Targets = Vec<TargetPtr>;

struct CostTargetHelper {
    matches_required: i16,
    unmet_weight: i16,
    met_weight: i16,
    cache_string: String,
}

impl CostTargetHelper {
    fn new(matches_needed: i16, mut unmet_w: i16, mut met_w: i16, cs: String) -> CostTargetHelper {
        if unmet_w < met_w {
            let temp = unmet_w;
            unmet_w = met_w;
            met_w = temp;
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
    cost_delta: Short,
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
            match current_costs.get(&adj_cost) {
                Some(_) => matched_count += 1,
                None => (),
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
                        i = i + 1;
                    }
                    votes.add_vote(&target, weight);
                }
            }
        }
        return matched_count < self.helper.matches_required;
    }
}

impl CostRelative {
    fn new(
        matches_needed: i16,
        unmet_w: i16,
        met_w: i16,
        delta: Short,
        strict: bool,
    ) -> CostRelative {
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
    limit: Short,
}

impl CostUpto {
    fn new(matches_needed: i16, unmet_w: i16, met_w: i16, upper: Short) -> CostUpto {
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
            if c.is_coin_only() {
                if c.get_coin() <= self.limit {
                    match_count += 1;
                };
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
        return match_count < self.helper.matches_required;
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
            match self.costs.get(c) {
                Some(_) => matched_count += 1,
                None => (),
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
        return matched_count < self.helper.matches_required;
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
    fn new(legal_costs: CostSet) -> CostVotes {
        CostVotes {
            available_costs: legal_costs,
            votes: HashMap::new(),
        }
    }
    fn add_vote(&mut self, c: &Cost, diff: f32) {
        if self.available_costs.contains(c) {
            let t = self.votes.entry(*c).or_insert(0.0);
            *t += diff;
        }
    }

    fn get_max_weighted(&self, max_cost: &mut CostSet, threshold: f32, tolerance: f32) -> bool {
        // we'll reject any votes below zero
        let mut max: f32 = 0.0;
        for (_, v) in &self.votes {
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
        return max > 0.0;
    }
}

// To do random stream, I need traits
// This should go in a namespace eventually
pub trait RandStream {
    fn get(&mut self) -> Unsigned;
    fn init_seed(&self) -> Unsigned;
}

// This was hidden as an implementation detail in c++
// Can I do something like that here?
struct BadRand {
    seed: Unsigned,
    cap: Unsigned,
    step: Unsigned,
    init: Unsigned,
}

fn make_bad_rand(s: Unsigned, bound: Unsigned) -> BadRand {
    let cap = bound;
    let mut setstep: Unsigned = 1; // Not convinced this init is necessary
    let mut f = bound / 2 + 1;
    while f < cap {
        let mut i: Unsigned = 2;
        while i < f {
            if f % i == 0 {
                break;
            };
            i = i + 1;
        }
        if i == f {
            setstep = f;
            break;
        }
        f = f + 1;
    }
    if f == cap {
        setstep = 1;
    }
    BadRand {
        seed: s,
        cap: bound,
        init: s,
        step: setstep,
    }
}

impl RandStream for BadRand {
    fn get(&mut self) -> Unsigned {
        if self.cap == 0 {
            return 0;
        }
        let newseed = (self.seed + self.step) % self.cap;
        self.seed = newseed;
        newseed
    }
    fn init_seed(&self) -> Unsigned {
        self.init
    }
}

fn get_rand_stream(s: Unsigned, cap: Unsigned, _use_bad_random: bool) -> impl RandStream {
    // eventually want to make this conditional on use_bad_random
    make_bad_rand(s, cap)
}

// What to do about vectors?
struct Card {
    name: String,
    pile: String,
    card_group: String,
    supply: bool,
    kingdom: bool,
    types: Vec<String>,
    cost: Cost,
    keywords: Vec<String>,
    kw_interactions: Vec<String>,
    other_interactions: Vec<String>,
    cost_targets: Targets,
}

type CardPtr = Rc<Card>;
type Cards = Vec<CardPtr>;

impl Card {
    // need to work out how many of these need to be copies and how many can be moved
    // Look at uses for Card constructor and work this out
    // A move constructor would have been fine for what I used it for
    fn new(
        card_name: &str,
        card_pile: &str,
        group_name: &str,
        card_in_supply: bool,
        card_is_kingdom: bool,
        card_types: Vec<String>,
        c: &Cost,
        card_keywords: Vec<String>,
        interacts_keywords: Vec<String>,
        interacts_other: Vec<String>,
        targets: Targets,
    ) -> Card {
        Card {
            name: String::from(card_name),
            pile: String::from(card_pile),
            card_group: String::from(group_name),
            supply: card_in_supply,
            kingdom: card_is_kingdom,
            types: card_types,
            cost: *c,
            keywords: card_keywords,
            kw_interactions: interacts_keywords,
            other_interactions: interacts_other,
            cost_targets: targets,
        }
    }
    fn get_name(&self) -> &str {
        &self.name
    }
    fn get_card_group(&self) -> &str {
        &self.card_group
    }
    fn get_pile_name(&self) -> &str {
        &self.pile
    }
    fn get_supply(&self) -> bool {
        self.supply
    }
    fn get_kingdom(&self) -> bool {
        self.kingdom
    }
    fn get_types(&self) -> &[String] {
        &self.types
    }
    fn get_cost(&self) -> &Cost {
        &self.cost
    }
    fn get_keywords(&self) -> &[String] {
        &self.keywords
    }
    fn get_kw_interactions(&self) -> &[String] {
        &self.kw_interactions
    }
    fn get_other_interactions(&self) -> &[String] {
        &self.other_interactions
    }
    fn get_cost_targets(&self) -> &Targets {
        &self.cost_targets
    }
}

impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Card {}

impl Hash for Card {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

// typedef not declared like variables
type CardSet = HashSet<CardPtr>;

struct Pile {
    name: String,
    card_group: String,
    supply: bool,
    kingdom: bool,
    types: HashSet<String>,
    costs: CostSet,
    keywords: HashSet<String>,
    kw_interactions: HashSet<String>,
    other_interactions: HashSet<String>,
    cards: Cards,
    targets: Targets,
}

type PilePtr = Rc<Pile>;

impl PartialEq for Pile {
    fn eq(&self, other: &Pile) -> bool {
        self.name == other.name
    }
}

impl Eq for Pile {}

impl Hash for Pile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Ord for Pile {
    fn cmp(&self, other: &Self) -> Ordering {
        return self.name.cmp(&other.name);
    }
}

impl PartialOrd for Pile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Pile {
    fn new(name: &str) -> Pile {
        Pile {
            name: String::from(name),
            card_group: String::new(),
            supply: false,
            kingdom: false,
            types: HashSet::new(),
            costs: CostSet::new(),
            keywords: HashSet::new(),
            kw_interactions: HashSet::new(),
            other_interactions: HashSet::new(),
            cards: Cards::new(),
            targets: Targets::new(),
        }
    }

    fn add_cost_target(&mut self, new_target: &TargetPtr) {
        for t in &self.targets {
            if t.str_rep() == new_target.str_rep() {
                return;
            }
        }
        self.targets.push(new_target.clone());
    }

    // This could be iffy if we were pointing to an external
    // card item but this design has Cards owned by Piles anyway
    fn add_card(&mut self, c: Card) {
        for cd in &self.cards {
            if c == *cd.borrow() {
                return;
            }
        }
        for t in c.get_types() {
            self.types.insert(t.to_string());
        }
        self.costs.insert(*c.get_cost());
        for t in c.get_keywords() {
            self.keywords.insert(t.to_string());
        }
        for t in c.get_keywords() {
            self.keywords.insert(t.to_string());
        }
        for t in c.get_kw_interactions() {
            self.kw_interactions.insert(t.to_string());
        }
        for t in c.get_other_interactions() {
            self.other_interactions.insert(t.to_string());
        }
        self.card_group = c.get_card_group().to_string();
        self.supply = c.get_supply() || self.supply;
        self.kingdom = c.get_kingdom() || self.kingdom;
        for t in c.get_cost_targets() {
            self.add_cost_target(t);
        }
        self.cards.push(Rc::new(c));
    }

    fn get_card_group(&self) -> &str {
        &self.card_group
    }
    fn get_supply(&self) -> bool {
        self.supply
    }
    fn get_kingdom(&self) -> bool {
        self.kingdom
    }
    // returning const references is not alien to c++
    fn get_types(&self) -> &HashSet<String> {
        &self.types
    }
    fn get_costs(&self) -> &CostSet {
        &self.costs
    }
    fn get_keywords(&self) -> &HashSet<String> {
        &self.keywords
    }
    fn get_kw_interactions(&self) -> &HashSet<String> {
        &self.kw_interactions
    }
    // In list of things I use, note a lot of string sets
    fn get_other_interactions(&self) -> &HashSet<String> {
        &self.other_interactions
    }
    fn get_name(&self) -> &str {
        &self.name
    }
    fn get_cards(&self) -> &Cards {
        &self.cards
    }
    fn get_targets(&self) -> &Targets {
        &self.targets
    }
}

type PileSet = BTreeSet<PilePtr>;
type Piles = Vec<PilePtr>;

struct Config {
    args: StringMultiMap,
    rand: Box<dyn RandStream>,
    why: bool,
    more_info: bool,
    optional_extras: UShort,
    validate: bool,
    list_collection: bool,
    disable_anti_cursors: bool,
    disable_attack_react: bool,
    max_cost_repeat: UShort,
    min_types: HashMap<String, UShort>,
    max_types: HashMap<String, UShort>,
    piles: PileSet,
    includes: PileSet,
}

fn bane_constraint(col: &CardCollectionPtr) -> ConstraintPtr {
    let has_yw = NameProperty::make_ptr(&"Young Witch".to_string());
    let mut cs = CostSet::new();
    cs.insert(Cost::new_s(2));
    cs.insert(Cost::new_s(3));
    let bane_cost = CostAndTypeProperty::make_ptr_set("Action".to_string(), cs);
    let begin = match col.get_iterators(&bane_cost) {
        None => {
            return Constraint::unsatisfiable(&"Failed bane constraint".to_string());
        }
        Some(v) => v,
    };
    let fix = FindBane::make_ptr(&col, &begin);
    let has_bane = NoteProperty::make_ptr(&"hasBane".to_string());
    // if we have less than 1 YoungWitch do nothing
    // if we have less than 1 hasBane note actionRequired   (only ever have 1 note)
    //.can accept more is empty (1,1)
    // from 1 .. MANY hasBane go inactive
    // more than MANY -> fail
    return Constraint::make_ptr_full(
        "bane".to_string(),
        Some(has_yw),
        &has_bane,
        Some(fix),
        1,
        1,
        1,
        MANY,
    );
}

fn prosp_constraint(col: &CardCollectionPtr) -> ConstraintPtr {
    let group_pros = CardGroupProperty::make_ptr(&"Prosperity".to_string());
    let has_pros_base = NoteProperty::make_ptr(&"addedProsperity-base".to_string());
    let fix = AddGroup::make_ptr(&col, &"Prosperity-base".to_string());
    // if we have less than 5 Prosperity cards do nothing
    // if we have less than 1 note, action required
    return Constraint::make_ptr_full(
        "prospBasics".to_string(),
        Some(group_pros),
        &has_pros_base,
        Some(fix),
        5,
        1,
        1,
        MANY,
    );
}

fn curser_constraint(col: &CardCollectionPtr, threshold: Unsigned) -> Option<ConstraintPtr> {
    let curser = KeywordProperty::make_ptr(&"curser".to_string(), false);
    let trash = KeywordProperty::make_ptr(&"trash_any".to_string(), true);
    let begin = match col.get_iterators(&trash) {
        Some(v) => v,
        None => return None,
    };
    let fix = FindPile::make_ptr(&col, &begin);
    return Some(Constraint::make_ptr_full(
        "counterCurser".to_string(),
        Some(curser),
        &trash,
        Some(fix),
        threshold,
        1,
        1,
        MANY,
    ));
}

fn attack_react_constraint(col: &CardCollectionPtr, threshold: Unsigned) -> Option<ConstraintPtr> {
    let attack = TypeProperty::make_ptr(&"Attack".to_string(), true);
    // only want kingdom and supply piles
    let react = OtherInteractionProperty::make_ptr(&"react(Attack)".to_string(), true);
    let begin = match col.get_iterators(&react) {
        Some(v) => v,
        None => return None,
    };
    let fix = FindPile::make_ptr(&col, &begin);
    return Some(Constraint::make_ptr_full(
        "counterAttack".to_string(),
        Some(attack),
        &react,
        Some(fix),
        threshold,
        1,
        1,
        MANY,
    ));
}

impl Config {
    fn get_string(&self) -> String {
        let mut res: String = "".to_string();
        for (k, v) in &self.args {
            let mut par = "".to_string();
            let mut first = true;
            for s in v {
                par = if first {
                    first = false;
                    format!("{}={}", k, s)
                } else {
                    format!("{},{}", par, s)
                }
            }
            res = format!("{} {}", res, par);
        }
        res
    }

    fn build_constraints(&mut self, col: &CardCollectionPtr) -> Result<Vec<ConstraintPtr>, String> {
        let fail_prop = FailProperty::make_ptr();
        let mut cons: Vec<ConstraintPtr> = vec![];
        cons.push(bane_constraint(col));
        cons.push(prosp_constraint(col));

        // let's abuse the Constraint machinery
        // The missing poition check will be zero both when there are no Potion costs
        //  and when there are Potion costs, but Potion cards are in the deck.
        // ie transition= 0 -> 1 -> 0
        //  when it hits 1 we want to add the potion which will drop it back
        // So, we want the main property to always fail to force the action to run.
        //  Since the precondition will remain 0 from then on, so the always failing main
        //   doesn't matter.
        let c = Constraint::make_ptr_full(
            "AddPotion".to_string(),
            Some(MissingPotionProperty::make_ptr()),
            &fail_prop.clone(),
            Some(AddGroup::make_ptr(&col, &"Alchemy-base".to_string())),
            1,
            MANY,
            MANY,
            MANY,
        );
        cons.push(c);

        let c = Constraint::make_ptr_full(
            "AddProsperityCards".to_string(),
            Some(NeedProsperity::make_ptr(
                (self.rand.get() % 10).try_into().unwrap(),
            )),
            &fail_prop.clone(),
            Some(AddProsperity::make_ptr(&col)),
            1,
            MANY,
            MANY,
            MANY,
        );
        cons.push(c);

        let c = Constraint::make_ptr_full(
            "AddInteractingGroup".to_string(),
            Some(MissingInteractingCardGroupProperty::make_ptr()),
            &fail_prop.clone(),
            Some(AddMissingDependencyGroup::make_ptr(&col)),
            1,
            MANY,
            MANY,
            MANY,
        );
        cons.push(c);

        let c = Constraint::make_ptr_full(
            "AddInteractingCard".to_string(),
            Some(MissingInteractingCardProperty::make_ptr()),
            &fail_prop,
            Some(AddMissingDependency::make_ptr(&col)),
            1,
            MANY,
            MANY,
            MANY,
        );
        cons.push(c);

        let c = Constraint::make_ptr_full(
            "AddHexForDoom".to_string(),
            Some(MissingGroupForKeywordProperty::make_ptr(
                &"Doom".to_string(),
                &"Nocturne-Hexes".to_string(),
            )),
            &fail_prop,
            Some(AddGroup::make_ptr(&col, &"Nocturne-Hexes".to_string())),
            1,
            MANY,
            MANY,
            MANY,
        );
        cons.push(c);

        let c = Constraint::make_ptr_full(
            "AddBoonForFate".to_string(),
            Some(MissingGroupForKeywordProperty::make_ptr(
                &"Fate".to_string(),
                &"Nocturne-Boons".to_string(),
            )),
            &fail_prop,
            Some(AddGroup::make_ptr(&col, &"Nocturne-Boons".to_string())),
            1,
            MANY,
            MANY,
            MANY,
        );
        cons.push(c);

        if !self.disable_anti_cursors {
            match curser_constraint(col, 1) {
                Some(c) => {
                    cons.push(c);
                }
                None => (),
            }
        }
        if !self.disable_attack_react {
            match attack_react_constraint(col, 2) {
                Some(c) => {
                    cons.push(c);
                }
                None => (),
            }
        }
        if self.max_cost_repeat > 0 {
            let c = Constraint::make_ptr(
                "RepeatedCosts".to_string(),
                &RepeatedCostProperty::make_ptr(self.max_cost_repeat.into()),
                None,
                0,
                1,
            );
            cons.push(c);
        }

        for (k, v) in &self.min_types {
            // Note there are two TypeProperties in use here
            // When getting iterators for possible piles to add we only want to
            // consider piles which are in the supply and kingdom cards
            //  (eg we don't want Platinum being picked as a possibility)
            // However when searching to see if we have too many, we want
            // to include all cards already selected.
            let type_name = k;
            let type_count = v;

            let searcher = TypeProperty::make_ptr(type_name, true);
            let t_begin = match col.get_iterators(&searcher) {
                Some(v) => v,
                None => {
                    return Err(format!("No matches found for type {}", type_name));
                }
            };
            // we count all treasures, but we don't want non-kingdom cards to be selected to satisfy a constraint
            let s = format!("At least {} {}s", type_count, type_name);
            let c = Constraint::make_ptr(
                s,
                &TypeProperty::make_ptr(type_name, false),
                Some(FindPile::make_ptr(&col, &t_begin)),
                (*type_count).into(),
                MANY,
            );
            cons.push(c);
        }
        for (k, v) in &self.max_types {
            let type_name = k;
            let type_count = v;

            let searcher = TypeProperty::make_ptr(type_name, false);
            match col.get_iterators(&searcher) {
                Some(_) => {
                    // we don't care if there are no cards for a max constraint
                    let s = format!("At most {} {}s", type_count, type_name);
                    let c = Constraint::make_ptr(
                        s,
                        &TypeProperty::make_ptr(type_name, false),
                        None,
                        0,
                        (*type_count).into(),
                    );
                    cons.push(c);
                }
                None => (),
            }
        }
        // now we need to find any keyword interactions
        let mut interacts_kw: BTreeSet<String> = BTreeSet::new();
        for p in col.get_piles() {
            for s in p.get_kw_interactions() {
                interacts_kw.insert(s.to_string());
            }
        }
        for s in &interacts_kw {
            if s == "gain" {
                let p1 = KeywordProperty::make_ptr(&"gain".to_string(), true);
                let p2 = KeywordProperty::make_ptr(&"+buy".to_string(), true);
                let prop = EitherProperty::make_ptr(&p1, &p2);
                let name = "Provide interacted keyword (gain/+buy)";
                match col.get_iterators(&prop) {
                    Some(begin) => {
                        let c = Constraint::make_ptr_full(
                            name.to_string(),
                            Some(HangingInteractsWith::make_ptr3(
                                &"gain".to_string(),
                                &"gain".to_string(),
                                &"+buy".to_string(),
                            )),
                            &fail_prop,
                            Some(FindPile::make_ptr(&col, &begin)),
                            1,
                            MANY,
                            MANY,
                            MANY,
                        );
                        cons.push(c);
                    }
                    None => (),
                }
            } else if s == "trash" {
                let p1 = KeywordProperty::make_ptr(&"trash_any".to_string(), true);
                let p2 = KeywordProperty::make_ptr(&"trash_limited".to_string(), true);
                let prop = EitherProperty::make_ptr(&p1, &p2);
                let name = "Provide interacted keyword (trash_any/trash_limited)";
                match col.get_iterators(&prop) {
                    Some(begin) => {
                        let prop = HangingInteractsWith::make_ptr3(
                            &"trash".to_string(),
                            &"trash_limited".to_string(),
                            &"trash_any".to_string(),
                        );
                        let c = Constraint::make_ptr_full(
                            name.to_string(),
                            Some(prop),
                            &fail_prop,
                            Some(FindPile::make_ptr(&col, &begin)),
                            1,
                            MANY,
                            MANY,
                            MANY,
                        );
                        cons.push(c);
                    }
                    None => (),
                };
            } else {
                let prop = KeywordProperty::make_ptr(s, true);
                match col.get_iterators(&prop) {
                    Some(begin) => {
                        let name = format!("Provide interacted keyword {}", s);
                        let c = Constraint::make_ptr_full(
                            name,
                            Some(HangingInteractsWith::make_ptr2(s, s)),
                            &fail_prop,
                            Some(FindPile::make_ptr(&col, &begin)),
                            1,
                            MANY,
                            MANY,
                            MANY,
                        );
                        cons.push(c);
                    }
                    None => (),
                };
            };
        }
        return Ok(cons);
    }
}

trait Property {
    fn is_selection_property(&self) -> bool;

    // no method overloading :-(

    fn pile_meets(&self, p: &PilePtr) -> bool;
    fn selection_meets(&self, s: &SelectionPtr) -> bool;
}

#[derive(Clone)]
struct PropertyPtr {
    state: Rc<dyn Property>,
}

impl PartialEq for PropertyPtr {
    fn eq(&self, other: &Self) -> bool {
        Rc::as_ptr(&self.state) == Rc::as_ptr(&other.state)
    }

    fn ne(&self, other: &Self) -> bool {
        Rc::as_ptr(&self.state) == Rc::as_ptr(&other.state)
    }
}

impl Eq for PropertyPtr {}

impl Hash for PropertyPtr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.state).hash(state);
    }
}

impl PropertyPtr {
    fn is_selection_property(&self) -> bool {
        self.state.is_selection_property()
    }

    fn pile_meets(&self, p: &PilePtr) -> bool {
        self.state.pile_meets(p)
    }

    fn selection_meets(&self, s: &SelectionPtr) -> bool {
        self.state.selection_meets(s)
    }
}

struct KingdomAndSupplyProperty {}

impl KingdomAndSupplyProperty {
    fn make_ptr() -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(KingdomAndSupplyProperty {}),
        }
    }
}

impl Property for KingdomAndSupplyProperty {
    fn is_selection_property(&self) -> bool {
        false
    }

    fn pile_meets(&self, p: &PilePtr) -> bool {
        p.get_kingdom() && p.get_supply()
    }

    fn selection_meets(&self, _s: &SelectionPtr) -> bool {
        false
    }
}

struct TypeProperty {
    type_name: String,
    kingdom_and_supply: bool,
}

impl TypeProperty {
    fn make_ptr(has_type: &String, restrict_to_kingdom_and_supply: bool) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(TypeProperty {
                type_name: has_type.clone(),
                kingdom_and_supply: restrict_to_kingdom_and_supply,
            }),
        }
    }
}

impl Property for TypeProperty {
    fn is_selection_property(&self) -> bool {
        false
    }

    fn pile_meets(&self, p: &PilePtr) -> bool {
        if self.kingdom_and_supply && (!p.get_kingdom() || !p.get_supply()) {
            return false;
        }
        return p.get_types().contains(&self.type_name);
    }

    fn selection_meets(&self, _s: &SelectionPtr) -> bool {
        false
    }
}

struct NameProperty {
    name: String,
}

impl NameProperty {
    fn make_ptr(name: &String) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(NameProperty {
                name: name.to_string(),
            }),
        }
    }
}

impl Property for NameProperty {
    fn is_selection_property(&self) -> bool {
        false
    }

    fn pile_meets(&self, p: &PilePtr) -> bool {
        self.name == p.get_name()
    }

    fn selection_meets(&self, _s: &SelectionPtr) -> bool {
        false
    }
}

struct CostAndTypeProperty {
    cost_prop: PropertyPtr,
    type_prop: TypeProperty,
}

impl CostAndTypeProperty {
    fn make_ptr_set(type_name: String, cost: CostSet) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(CostAndTypeProperty {
                cost_prop: CostProperty::make_ptr_set(cost, true),
                type_prop: TypeProperty {
                    type_name: type_name,
                    kingdom_and_supply: true,
                },
            }),
        }
    }
}

impl Property for CostAndTypeProperty {
    fn is_selection_property(&self) -> bool {
        false
    }

    fn pile_meets(&self, p: &PilePtr) -> bool {
        self.cost_prop.pile_meets(p) && self.type_prop.pile_meets(p)
    }

    fn selection_meets(&self, _s: &SelectionPtr) -> bool {
        false
    }
}

struct NoteProperty {
    text: String,
}

impl NoteProperty {
    fn make_ptr(text: &String) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(NoteProperty {
                text: text.to_string(),
            }),
        }
    }
}

impl Property for NoteProperty {
    fn is_selection_property(&self) -> bool {
        true
    }

    fn pile_meets(&self, _p: &PilePtr) -> bool {
        false
    }

    fn selection_meets(&self, s: &SelectionPtr) -> bool {
        s.has_note(&self.text)
    }
}

struct EitherProperty {
    prop1: PropertyPtr,
    prop2: PropertyPtr,
}

impl EitherProperty {
    fn make_ptr(prop1: &PropertyPtr, prop2: &PropertyPtr) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(EitherProperty {
                prop1: prop1.clone(),
                prop2: prop2.clone(),
            }),
        }
    }
}

impl Property for EitherProperty {
    fn is_selection_property(&self) -> bool {
        self.prop1.is_selection_property() || self.prop2.is_selection_property()
    }

    fn pile_meets(&self, p: &PilePtr) -> bool {
        self.prop1.pile_meets(p) || self.prop2.pile_meets(p)
    }

    fn selection_meets(&self, s: &SelectionPtr) -> bool {
        self.prop1.selection_meets(s) || self.prop2.selection_meets(s)
    }
}

struct CardGroupProperty {
    group_name: String,
}

impl CardGroupProperty {
    fn make_ptr(group_name: &String) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(CardGroupProperty {
                group_name: group_name.clone(),
            }),
        }
    }
}

impl Property for CardGroupProperty {
    fn is_selection_property(&self) -> bool {
        false
    }

    fn pile_meets(&self, p: &PilePtr) -> bool {
        p.get_card_group() == self.group_name
    }

    fn selection_meets(&self, _s: &SelectionPtr) -> bool {
        false
    }
}

struct OptionalExtraProperty {}

impl OptionalExtraProperty {
    fn make_ptr() -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(OptionalExtraProperty {}),
        }
    }
}

impl Property for OptionalExtraProperty {
    fn is_selection_property(&self) -> bool {
        false
    }

    fn pile_meets(&self, p: &PilePtr) -> bool {
        return !p.get_supply()
            && !p.get_kingdom()
            && (p.get_types().contains("Event")
                || p.get_types().contains("Project")
                || p.get_types().contains("Landmark")
                || p.get_types().contains("Way"));
    }

    fn selection_meets(&self, _s: &SelectionPtr) -> bool {
        false
    }
}

struct OtherInteractionProperty {
    other_interact: String,
    kingdom_and_supply: bool,
}

impl OtherInteractionProperty {
    fn make_ptr(other_interact: &String, kingdom_and_supply: bool) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(OtherInteractionProperty {
                other_interact: other_interact.clone(),
                kingdom_and_supply,
            }),
        }
    }
}

impl Property for OtherInteractionProperty {
    fn is_selection_property(&self) -> bool {
        false
    }

    fn pile_meets(&self, p: &PilePtr) -> bool {
        if self.kingdom_and_supply && (!p.get_supply() || !p.get_kingdom()) {
            return false;
        }
        p.get_other_interactions().contains(&self.other_interact)
    }

    fn selection_meets(&self, _s: &SelectionPtr) -> bool {
        false
    }
}

struct MissingPotionProperty {}

impl MissingPotionProperty {
    fn make_ptr() -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(MissingPotionProperty {}),
        }
    }
}

impl Property for MissingPotionProperty {
    fn is_selection_property(&self) -> bool {
        true
    }

    fn pile_meets(&self, _p: &PilePtr) -> bool {
        false
    }

    fn selection_meets(&self, s: &SelectionPtr) -> bool {
        let mut found = false;
        let mut have_potion = false;
        for p in s.get_piles() {
            if p.get_name() == "Potion" {
                have_potion = true;
                continue;
            }
            for c in p.get_costs() {
                if c.has_potion() {
                    found = true;
                    break;
                }
            }
        }
        return found && !have_potion;
    }
}

struct MissingGroupForKeywordProperty {
    type_needed: String,
    note: String,
}

impl MissingGroupForKeywordProperty {
    fn make_ptr(type_needed: &String, group_needed: &String) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(MissingGroupForKeywordProperty {
                type_needed: type_needed.to_string(),
                note: format!("added{}", group_needed),
            }),
        }
    }
}

impl Property for MissingGroupForKeywordProperty {
    fn is_selection_property(&self) -> bool {
        true
    }

    fn pile_meets(&self, _p: &PilePtr) -> bool {
        false
    }

    fn selection_meets(&self, s: &SelectionPtr) -> bool {
        for p in s.get_piles() {
            for it in p.get_types() {
                if it.starts_with(&self.type_needed) {
                    if !s.has_note(&self.note) {
                        return true;
                    }
                }
            }
        }
        return false;
    }
}

struct MissingInteractingCardGroupProperty {}

impl MissingInteractingCardGroupProperty {
    fn make_ptr() -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(MissingInteractingCardGroupProperty {}),
        }
    }
}

impl Property for MissingInteractingCardGroupProperty {
    fn is_selection_property(&self) -> bool {
        true
    }

    fn pile_meets(&self, _p: &PilePtr) -> bool {
        false
    }

    fn selection_meets(&self, s: &SelectionPtr) -> bool {
        for p in s.get_piles() {
            for it in p.get_other_interactions() {
                if it.starts_with("group(") {
                    let glen = "group(".len();
                    let need_name = &it[glen..it.len() - ')'.len_utf8()];
                    if !s.has_note(&format!("added{}", need_name)) {
                        return true;
                    }
                }
            }
        }
        return false;
    }
}

struct MissingInteractingCardProperty {}

impl MissingInteractingCardProperty {
    fn make_ptr() -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(MissingInteractingCardProperty {}),
        }
    }
}

impl Property for MissingInteractingCardProperty {
    fn is_selection_property(&self) -> bool {
        true
    }

    fn pile_meets(&self, _p: &PilePtr) -> bool {
        false
    }

    fn selection_meets(&self, s: &SelectionPtr) -> bool {
        let mut need: BTreeSet<String> = BTreeSet::new();
        for p in s.get_piles() {
            for it in p.get_other_interactions() {
                if it.starts_with("card(") {
                    let prefix_len = "card(".len();
                    let need_name = &it[prefix_len..it.len() - ')'.len_utf8()];
                    need.insert(need_name.to_string());
                }
            }
        }
        if need.len() == 0 {
            return false;
        }
        for name in need {
            let mut found = false;
            for c in s.get_cards() {
                if c.get_name() == name {
                    found = true;
                    break;
                }
            }
            if !found {
                return true;
            }
        }
        return false;
    }
}

struct FailProperty {}

impl FailProperty {
    fn make_ptr() -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(FailProperty {}),
        }
    }
}

impl Property for FailProperty {
    fn is_selection_property(&self) -> bool {
        false
    }

    fn pile_meets(&self, _p: &PilePtr) -> bool {
        false
    }

    fn selection_meets(&self, _s: &SelectionPtr) -> bool {
        false
    }
}

struct RepeatedCostProperty {
    max_repeats: Unsigned,
}

impl RepeatedCostProperty {
    fn make_ptr(max_repeats: Unsigned) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(RepeatedCostProperty { max_repeats }),
        }
    }
}

impl Property for RepeatedCostProperty {
    fn is_selection_property(&self) -> bool {
        true
    }

    fn pile_meets(&self, _p: &PilePtr) -> bool {
        false
    }

    fn selection_meets(&self, s: &SelectionPtr) -> bool {
        let mut counts: HashMap<Cost, Unsigned> = HashMap::new();
        for c in s.get_cost_set() {
            counts.insert(*c, 0);
        }
        for p in s.get_piles() {
            for c in p.get_costs() {
                if let Occupied(mut r) = counts.entry(*c) {
                    *r.get_mut() += 1;
                }
            }
        }
        for (_first, second) in counts {
            if second > self.max_repeats {
                return true;
            }
        }
        return false;
    }
}

struct CostProperty {
    single_cost: Cost,
    costs: CostSet,
    supply_only: bool,
}

impl CostProperty {
    fn make_ptr_set(costs: CostSet, supply_only: bool) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(CostProperty {
                single_cost: Cost::dummy(),
                costs,
                supply_only,
            }),
        }
    }
}

impl Property for CostProperty {
    fn is_selection_property(&self) -> bool {
        false
    }

    fn pile_meets(&self, p: &PilePtr) -> bool {
        if self.supply_only && !p.get_supply() {
            return false;
        }
        if self.single_cost.valid() {
            return p.get_costs().contains(&self.single_cost);
        }
        // we need to find if there is a non-empty intersection
        // between the cost sets. I'm not using std::set_intersection
        // because I don't need to construct the intersection
        return Cost::intersects(p.get_costs(), &self.costs);
    }

    fn selection_meets(&self, _s: &SelectionPtr) -> bool {
        false
    }
}

struct HangingInteractsWith {
    interacts_with: String,
    kw: String,
    alt_kw: String,
}

impl HangingInteractsWith {
    fn make_ptr2(interacts_with: &String, kw: &String) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(HangingInteractsWith {
                interacts_with: interacts_with.to_string(),
                kw: kw.to_string(),
                alt_kw: "".to_string(),
            }),
        }
    }

    fn make_ptr3(interacts_with: &String, kw: &String, alt_kw: &String) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(HangingInteractsWith {
                interacts_with: interacts_with.to_string(),
                kw: kw.to_string(),
                alt_kw: alt_kw.to_string(),
            }),
        }
    }
}

impl Property for HangingInteractsWith {
    fn is_selection_property(&self) -> bool {
        true
    }

    fn pile_meets(&self, _p: &PilePtr) -> bool {
        false
    }

    fn selection_meets(&self, s: &SelectionPtr) -> bool {
        if !s
            .get_interacts_keywords()
            .contains_key(&self.interacts_with)
        {
            return false; // no card has that interacts with
        }
        if s.get_keywords().contains_key(&self.kw) {
            return false; // we have both interaction and keyword
        }
        if s.get_keywords().contains_key(&self.alt_kw) {
            return false; // we have both interaction and keyword
        }
        return true;
    }
}

struct KeywordProperty {
    keyword: String,
    kingdom_and_supply: bool,
}

impl KeywordProperty {
    fn make_ptr(keyword: &String, restrict_to_kingdom_and_supply: bool) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(KeywordProperty {
                keyword: keyword.clone(),
                kingdom_and_supply: restrict_to_kingdom_and_supply,
            }),
        }
    }
}

impl Property for KeywordProperty {
    fn is_selection_property(&self) -> bool {
        false
    }

    fn pile_meets(&self, p: &PilePtr) -> bool {
        if self.kingdom_and_supply && (!p.get_kingdom() || !p.get_supply()) {
            return false;
        }
        return p.get_keywords().contains(&self.keyword);
    }

    fn selection_meets(&self, _s: &SelectionPtr) -> bool {
        false
    }
}

struct KeywordInteractionProperty {
    keyword: String,
}

impl KeywordInteractionProperty {}

impl Property for KeywordInteractionProperty {
    fn is_selection_property(&self) -> bool {
        false
    }

    fn pile_meets(&self, p: &PilePtr) -> bool {
        return p.get_kw_interactions().contains(&self.keyword);
    }

    fn selection_meets(&self, _s: &SelectionPtr) -> bool {
        false
    }
}

struct NeedProsperity {
    threshold: UShort,
}

impl NeedProsperity {
    fn make_ptr(threshold: UShort) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(NeedProsperity { threshold }),
        }
    }
}

impl Property for NeedProsperity {
    fn is_selection_property(&self) -> bool {
        true
    }

    fn pile_meets(&self, _p: &PilePtr) -> bool {
        false
    }

    fn selection_meets(&self, s: &SelectionPtr) -> bool {
        let col = s.get_collection();
        let colony = match col.get_pile_for_card(&"Colony".to_string()) {
            Some(m) => m,
            None => {
                return false;
            }
        };
        let platinum = match col.get_pile_for_card(&"Platinum".to_string()) {
            Some(m) => m,
            None => {
                return false;
            }
        };
        let has_col = s.contains(&colony);
        let has_plat = s.contains(&platinum);
        if has_col && has_plat {
            return false;
        };
        if has_col != has_plat {
            return true;
        };
        // count how many prosperity cards we have
        let mut total = 0;
        for pp in s.get_piles() {
            if pp.get_card_group().starts_with(&"Prosperity".to_string()) {
                total += 1;
            }
        }
        return (self.threshold > 0) && (self.threshold <= total);
    }
}

trait ConstraintAction {
    // send back a selection or an error message
    fn apply(&self, label: &str, start: &SelectionPtr) -> Result<SelectionPtr, String>;
}

struct ConstraintActionPtr(Rc<dyn ConstraintAction>);

impl ConstraintActionPtr {
    fn apply(&self, label: &str, start: &SelectionPtr) -> Result<SelectionPtr, String> {
        return self.0.apply(label, start);
    }
}

struct FindBane {
    begin: CollectionIterator,
    col: CardCollectionPtr,
}

impl FindBane {
    fn make_ptr(col: &CardCollectionPtr, begin_it: &CollectionIterator) -> ConstraintActionPtr {
        ConstraintActionPtr {
            0: Rc::new(FindBane {
                begin: begin_it.clone(),
                col: col.clone(),
            }),
        }
    }
}

impl ConstraintAction for FindBane {
    fn apply(&self, label: &str, start: &SelectionPtr) -> Result<SelectionPtr, String> {
        for it in &mut self.begin.clone() {
            if !start.contains(&it) {
                let mut new_sel = start.duplicate_state();
                new_sel.increase_required_piles();
                if new_sel.add_pile(&it) {
                    new_sel.tag_pile(&it, &"Bane".to_string());
                    new_sel.tag_pile(&it, &format!("<why:{}>", label));
                    new_sel.add_note(&"hasBane".to_string());
                    let res = self.col.build_selection(&SelectionPtr::from_state(new_sel));
                    if let Ok(_) = res {
                        return res;
                    }
                }
            }
        }
        return Err("".to_string());
    }
}

struct AddGroup {
    group: String,
    coll: CardCollectionPtr,
}

impl AddGroup {
    fn make_ptr(coll: &CardCollectionPtr, group: &String) -> ConstraintActionPtr {
        ConstraintActionPtr {
            0: Rc::new(AddGroup {
                group: group.to_string(),
                coll: coll.clone(),
            }),
        }
    }
}

impl ConstraintAction for AddGroup {
    fn apply(&self, label: &str, start: &SelectionPtr) -> Result<SelectionPtr, String> {
        let mut new_sel = start.duplicate_state();
        let p = CardGroupProperty::make_ptr(&self.group);
        let begin = match self.coll.get_iterators(&p) {
            Some(v) => v,
            None => {
                return Err(format!(
                    "Tried to add group ({}) but no cards belonging to it found in the collection.",
                    self.group
                ))
            }
        };
        for it in begin {
            if new_sel.add_pile(&it)
            // Not catching individual fails
            {
                new_sel.tag_pile(&it, &format!("<why:{}>", label));
            };
        } // maybe some got added some other way?
        new_sel.add_note(&format!("added{}", self.group));
        return self
            .coll
            .build_selection(&SelectionPtr::from_state(new_sel));
    }
}

struct FindPile {
    begin: CollectionIterator,
    col: CardCollectionPtr,
}

impl FindPile {
    fn make_ptr(col: &CardCollectionPtr, begin_it: &CollectionIterator) -> ConstraintActionPtr {
        ConstraintActionPtr {
            0: Rc::new(FindPile {
                begin: begin_it.clone(),
                col: col.clone(),
            }),
        }
    }
}

impl ConstraintAction for FindPile {
    fn apply(&self, label: &str, start: &SelectionPtr) -> Result<SelectionPtr, String> {
        for it in self.begin.clone() {
            if !start.contains(&it) {
                let mut new_sel = start.duplicate_state();
                if new_sel.add_pile(&it) {
                    new_sel.tag_pile(&it, &format!("<why?{}>", label));
                    if let Ok(s) = self.col.build_selection(&SelectionPtr::from_state(new_sel)) {
                        return Ok(s);
                    }
                }
            };
        }
        return Err("".to_string());
    }
}

struct AddMissingDependency {
    col: CardCollectionPtr,
}

impl AddMissingDependency {
    fn make_ptr(col: &CardCollectionPtr) -> ConstraintActionPtr {
        ConstraintActionPtr {
            0: Rc::new(AddMissingDependency { col: col.clone() }),
        }
    }
}

impl ConstraintAction for AddMissingDependency {
    fn apply(&self, _label: &str, start: &SelectionPtr) -> Result<SelectionPtr, String> {
        let mut need = BTreeMap::<String, String>::new();
        for p in start.get_piles()
        // This code is duplicated in Missing...Property
        {
            for it in p.get_other_interactions() {
                if it.starts_with("card(") {
                    let prefix_len = "card(".len();
                    let need_name = &it[prefix_len..it.len() - '('.len_utf8()];
                    need.insert(need_name.to_string(), p.get_name().to_string());
                }
            }
        }
        if need.len() == 0 {
            return Err(
                "AddMissingDependency applied but no cards have card() OtherInteractions"
                    .to_string(),
            );
        };
        for (first, second) in &need {
            let p = match self.col.get_pile_for_card(first) {
                Some(v) => v,
                None => {
                    return Err(format!("Unable to find a pile containing {}", first));
                }
            };
            if !start.contains(&p) {
                let mut new_sel = start.duplicate_state();
                if new_sel.add_pile(&p) {
                    new_sel.tag_pile(&p, &format!("<why?card:{} interacts with it>", second));
                    return self.col.build_selection(&SelectionPtr::from_state(new_sel));
                };
            };
        }
        return Err("AddMissingDependency applied but nothing seemed missing".to_string());
    }
}

struct AddMissingDependencyGroup {
    col: CardCollectionPtr,
}

impl AddMissingDependencyGroup {
    fn make_ptr(col: &CardCollectionPtr) -> ConstraintActionPtr {
        ConstraintActionPtr {
            0: Rc::new(AddMissingDependencyGroup { col: col.clone() }),
        }
    }
}

impl ConstraintAction for AddMissingDependencyGroup {
    fn apply(&self, _label: &str, start: &SelectionPtr) -> Result<SelectionPtr, String> {
        // The c++ version only initialised when needed
        // but an Option seemed clunky
        let mut new_sel = start.duplicate_state();
        let mut acted = false;
        for p in start.get_piles() {
            for it in p.get_other_interactions() {
                if it.starts_with("group(") {
                    let prefix_len = "group(".len();
                    let need_name = &it[prefix_len..it.len() - ')'.len_utf8()];
                    if !start.has_note(&format!("added{}", need_name)) {
                        let ps = CardGroupProperty::make_ptr(&need_name.to_string());
                        let piles = match start.get_collection().get_iterators(&ps) {
                            None => {
                                return Err(format!(
                                    "Unable to find required group named {}",
                                    need_name
                                ))
                            }
                            Some(r) => r,
                        };
                        acted = true;
                        for i in piles {
                            if new_sel.add_pile(&i) {
                                new_sel.tag_pile(
                                    &i,
                                    &format!("<why?cards:{} needs it>", p.get_name()),
                                );
                            } else {
                                return Err(format!("Unable to add card {}", i.get_name()));
                            }
                        }
                        new_sel.add_note(&format!("added{}", need_name));
                    };
                }
            }
        }
        if !acted {
            return Err(
                "AddMissingDependencyGroup called buit nothing seems to be missing".to_string(),
            );
        }
        return self.col.build_selection(&SelectionPtr::from_state(new_sel));
    }
}

struct AddProsperity {
    col: CardCollectionPtr,
}

impl AddProsperity {
    fn make_ptr(col: &CardCollectionPtr) -> ConstraintActionPtr {
        ConstraintActionPtr {
            0: Rc::new(AddProsperity { col: col.clone() }),
        }
    }
}

impl ConstraintAction for AddProsperity {
    fn apply(&self, label: &str, start: &SelectionPtr) -> Result<SelectionPtr, String> {
        // The c++ version only initialised when needed
        // but an Option seemed clunky
        let mut new_sel = start.duplicate_state();

        let col = start.get_collection();
        let platinum = match col.get_pile_for_card(&"Platinum".to_string()) {
            Some(m) => m,
            None => {
                return Err("Can't find prosperity base cards".to_string());
            }
        };
        let colony = match col.get_pile_for_card(&"Colony".to_string()) {
            Some(m) => m,
            None => {
                return Err("Can't find prosperity base cards".to_string());
            }
        };
        if !new_sel.contains(&platinum) {
            if !new_sel.add_pile(&platinum) {
                return Err("Error adding Platinum".to_string());
            }
            new_sel.tag_pile(&platinum, &label.to_string());
        };
        if !new_sel.contains(&colony) {
            if !new_sel.add_pile(&colony) {
                return Err("Error adding Colony".to_string());
            }
            new_sel.tag_pile(&colony, &label.to_string());
        };
        return self.col.build_selection(&SelectionPtr::from_state(new_sel));
    }
}

#[derive(PartialEq, Eq)]
enum ConsResult {
    ConsOK,           // Constraint is neutral/inactive on the selection
    ConsActionReq,    // something needs to be done to satisfy constraint
    ConsMorePossible, // Constraint is satisfied but could accept additional cards
    ConsFail,         // constraint can not be satisfied from current selection
}

struct Constraint {
    property: PropertyPtr,
    precondition: Option<PropertyPtr>,
    action: Option<ConstraintActionPtr>,
    prop_active: Unsigned,    // x
    prop_satisfied: Unsigned, // a
    prop_inactive: Unsigned,  // b
    prop_broken: Unsigned,    // c
    why: String,
}

impl Constraint {
    fn unsatisfiable(label: &String) -> ConstraintPtr {
        Rc::new(Constraint {
            property: FailProperty::make_ptr(),
            precondition: None,
            action: None,
            prop_active: 0,
            prop_satisfied: 0,
            prop_inactive: 0,
            prop_broken: 0,
            why: label.to_string(),
        })
    }

    fn make_ptr(
        label: String,
        prop: &PropertyPtr,
        act: Option<ConstraintActionPtr>,
        min: Unsigned,
        max: Unsigned,
    ) -> ConstraintPtr {
        Rc::new(Constraint {
            property: prop.clone(),
            precondition: None,
            action: act,
            prop_active: 0,
            prop_satisfied: min,
            prop_inactive: min,
            prop_broken: max + 1,
            why: label.to_string(),
        })
    }

    fn make_ptr_full(
        label: String,
        pre: Option<PropertyPtr>,
        prop: &PropertyPtr,
        act: Option<ConstraintActionPtr>,
        x: Unsigned,
        a: Unsigned,
        b: Unsigned,
        c: Unsigned,
    ) -> ConstraintPtr {
        Rc::new(Constraint {
            property: prop.clone(),
            precondition: pre,
            action: act,
            prop_active: x,
            prop_satisfied: a,
            prop_inactive: b,
            prop_broken: c,
            why: label,
        })
    }

    fn get_status(&self, sel: &SelectionPtr) -> ConsResult {
        let piles = &sel.get_piles();
        if let Some(prec) = &self.precondition {
            let mut count = 0;
            if prec.is_selection_property() {
                if prec.selection_meets(sel) {
                    count += 1;
                }
            } else {
                for p in *piles {
                    if prec.pile_meets(p) {
                        count += 1;
                    }
                }
            }
            if count < self.prop_active {
                return ConsResult::ConsOK;
            }
        } // so we need to test property
        let mut count = 0;
        if self.property.is_selection_property() {
            if self.property.selection_meets(sel) {
                count += 1;
            }
        } else {
            for p in *piles {
                if self.property.pile_meets(p) {
                    count += 1;
                }
            }
        }
        if count >= self.prop_broken {
            return ConsResult::ConsFail;
        }
        if count >= self.prop_inactive {
            return ConsResult::ConsOK;
        }
        if count >= self.prop_satisfied {
            return ConsResult::ConsMorePossible;
        }
        return ConsResult::ConsActionReq;
    }

    fn act(&self, start: &SelectionPtr) -> Result<SelectionPtr, String> {
        match &self.action {
            Some(act) => {
                return act.apply(&self.why, start);
            }
            None => Err("".to_string()),
        }
    }
}

type ConstraintPtr = Rc<Constraint>;

struct SelectionState {
    piles: Piles,
    cards: Cards,
    constraints: Rc<RefCell<Vec<ConstraintPtr>>>,
    tags: RefCell<BTreeMap<PilePtr, Vec<String>>>,
    required_cards: UShort,
    current_normal_pile_count: UShort,
    notes: BTreeSet<String>,
    need_items: RefCell<BTreeSet<String>>, // <= required_cards
    costs_in_supply: CostSet,
    // This one needs to be modified after wrapping
    target_check_required: RefCell<bool>,
    target_blame: RefCell<String>, // piles responsible for cost target
    targets: Targets,
    interacts_keywords: BTreeMap<String, Unsigned>,
    keywords: BTreeMap<String, Unsigned>,
    card_coll: CardCollectionPtr,
    begin_general: RefCell<CollectionIterator>,
}

impl SelectionState {
    fn get_collection(&self) -> &CardCollectionPtr {
        &self.card_coll
    }

    fn get_piles(&self) -> &Piles {
        &self.piles
    }

    fn add_constraint(&mut self, cp: ConstraintPtr) {
        let c_vec: &RefCell<Vec<ConstraintPtr>> = self.constraints.borrow();
        c_vec.borrow_mut().push(cp.clone());
    }

    // only use so far is to make space for "bane" card
    fn increase_required_piles(&mut self) {
        self.required_cards += 1
    }

    fn add_pile(&mut self, p: &PilePtr) -> bool {
        for k in &self.piles {
            if k == p {
                return false;
            }
        }
        self.piles.push(p.clone());
        if p.get_supply() && p.get_kingdom() {
            if self.current_normal_pile_count >= self.required_cards {
                return false; // silent failure if no room to add card
            }
            self.current_normal_pile_count += 1;
        }
        for c in p.get_cards() {
            self.cards.push(c.clone());
            if c.get_supply() {
                self.costs_in_supply.insert(c.get_cost().clone());
            }
        }
        if !p.get_targets().is_empty() {
            self.set_need_to_check(true, &p.get_name().to_string());
            *self.target_check_required.borrow_mut() = true;
            for t in p.get_targets() {
                // need to ensure no duplicates
                let mut b = false;
                for v in &self.targets {
                    if v.str_rep() == t.str_rep() {
                        b = true;
                    }
                }
                if !b {
                    self.targets.push(t.clone());
                }
            }
        }
        for kw in p.get_keywords() {
            // replacement for addCount call
            let e = self.keywords.entry(kw.to_string()).or_insert(0);
            *e += 1;
        }
        for ikw in p.get_kw_interactions() {
            let e = self.interacts_keywords.entry(ikw.clone()).or_insert(0);
            *e += 1;
        }
        for react in p.get_other_interactions() {
            if react.find("react(") == Some(0) {
                let len1 = "react(".len();
                let len2 = ")".len();
                let e = self
                    .interacts_keywords
                    .entry(react[len1..react.len() - len2].to_string())
                    .or_insert(0);
                *e += 1;
            }
        }
        return true;
    }

    fn tag_pile(&self, p: &PilePtr, tag: &String) {
        let r = &mut self.tags.borrow_mut();
        let vs = r.entry(p.clone()).or_insert(vec![]);
        vs.push(tag.to_string());
    }

    fn add_note(&mut self, s: &String) {
        self.notes.insert(s.to_string());
    }

    fn add_item(&self, s: &String) {
        self.need_items.borrow_mut().insert(s.to_string());
    }

    fn set_need_to_check(&self, v: bool, s: &String) {
        let old_value: bool = *self.target_check_required.borrow();
        let mut target_blame = self.target_blame.borrow_mut();
        let old_len = target_blame.len();
        if v {
            if !old_value || old_len == 0
            // transition from false to true
            {
                // or no previous string
                *target_blame = s.clone();
                //self.target_blame.swap(s.clone());
            } else {
                *target_blame = format!("{},{}", *target_blame, s);
            }
        }
        *self.target_check_required.borrow_mut() = v;
    }

    fn get_target_string(&self) -> String {
        self.target_blame.borrow().clone()
    }

    fn new2(
        col: &CardCollectionPtr,
        general_begin: CollectionIterator,
        market_cap: UShort,
    ) -> SelectionState {
        SelectionState {
            piles: Piles::new(),
            cards: Cards::new(),
            constraints: Rc::new(RefCell::new(vec![])),
            tags: RefCell::new(BTreeMap::new()),
            required_cards: market_cap,
            current_normal_pile_count: 0,
            notes: BTreeSet::new(),
            need_items: RefCell::new(BTreeSet::new()), // <= required_cards
            costs_in_supply: CostSet::new(),
            target_check_required: RefCell::new(false),
            targets: Targets::new(),
            target_blame: RefCell::new("".to_string()), // piles responsible for cost target

            interacts_keywords: BTreeMap::new(),
            keywords: BTreeMap::new(),
            card_coll: col.clone(),

            begin_general: RefCell::new(general_begin),
        }
    }

    fn new1(
        coll: &CardCollectionPtr,
        general_begin: CollectionIterator,
        market_cap: UShort,
    ) -> SelectionState {
        SelectionState::new2(coll, general_begin, market_cap)
    }

    fn new(coll: &CardCollectionPtr, general_begin: CollectionIterator) -> SelectionState {
        SelectionState::new1(coll, general_begin, 10)
    }

    fn contains(&self, p: &PilePtr) -> bool {
        for t in &self.piles {
            if t == p {
                return true;
            }
        }
        return false;
    }
}

#[derive(Ord, Eq)]
struct SortablePile {
    p: PilePtr,
}

impl PartialEq for SortablePile {
    fn eq(&self, other: &SortablePile) -> bool {
        // can do this because no cards should have
        // the same name but different groups
        return self.p.get_name() == other.p.get_name();
    }
}

impl PartialOrd for SortablePile {
    fn partial_cmp(&self, other: &SortablePile) -> Option<Ordering> {
        let mine = self.p.get_card_group();
        let theirs = other.p.get_card_group();
        if mine < theirs {
            return Some(Less);
        }
        if mine > theirs {
            return Some(Greater);
        }
        // card groups are equal
        let mine = self.p.get_name();
        let theirs = other.p.get_name();
        if mine < theirs {
            return Some(Less);
        }
        if mine > theirs {
            return Some(Greater);
        }
        return Some(Equal);
    }
}

struct SelectionPtr {
    state: Rc<SelectionState>,
}

impl SelectionPtr {
    fn from_state(s: SelectionState) -> SelectionPtr {
        SelectionPtr { state: Rc::new(s) }
    }

    // Makes a copy of the state to modify before
    // wrapping it in a SelectionPtr later
    fn duplicate_state(&self) -> SelectionState {
        let state = &self.state;
        SelectionState {
            piles: state.piles.clone(),
            cards: state.cards.clone(),
            constraints: state.constraints.clone(),
            tags: state.tags.clone(),
            required_cards: state.required_cards,
            current_normal_pile_count: state.current_normal_pile_count,
            notes: state.notes.clone(),
            need_items: state.need_items.clone(),
            costs_in_supply: state.costs_in_supply.clone(),
            target_check_required: RefCell::new(*state.target_check_required.borrow()),
            targets: state.targets.clone(),
            target_blame: state.target_blame.clone(),
            interacts_keywords: state.interacts_keywords.clone(),
            keywords: state.keywords.clone(),
            card_coll: state.card_coll.clone(),
            begin_general: state.begin_general.clone(),
        }
    }

    fn dump(&self, show_all: bool, show_card_info: bool) {
        let mut result: Vec<SortablePile> = vec![];
        result.reserve(self.state.piles.len());
        let mut max_len: usize = 0;
        for p in &self.state.piles {
            result.push(SortablePile { p: p.clone() });
            let l = p.get_name().len();
            if max_len < l {
                max_len = l;
            }
        }
        result.sort();
        let mut group_name = "".to_string();
        let mut items: BTreeSet<String> = BTreeSet::new();

        for pp in &result {
            let p = &pp.p;
            if p.get_card_group() != group_name {
                group_name = p.get_card_group().to_string();
                print!("From {}\n", group_name);
            }
            print!("   {}", p.get_name());
            match self.state.tags.borrow().get(p) {
                Some(e) => {
                    let mut first = true;
                    for s in e {
                        if show_all || !s.contains('<') {
                            print!("{}{}", if first { " (" } else { ", " }, s);
                            first = false;
                        }
                    }
                    if !first {
                        print!(")");
                    }
                }
                None => (),
            };
            if show_card_info {
                for _ in p.get_name().len()..max_len {
                    print!(" ");
                }
                print!(" types=");
                let mut first = true;
                for s in p.get_types() {
                    if !first {
                        print!(", ");
                    }
                    first = false;
                    print!("{}", s)
                }
                print!(" costs={{");
                first = true;
                for c in p.get_costs() {
                    if !first {
                        print!(", ");
                    }
                    first = false;
                    print!("{}", c.get_string());
                }
                print!("}}");
            }
            print!("\n");
            for s in p.get_other_interactions() {
                if s.starts_with("item(") {
                    let l1 = "item(".len();
                    let l2 = s.len() - ')'.len_utf8();
                    items.insert(s[l1..l2].to_string());
                }
            }
        }
        let its: &BTreeSet<String> = &self.state.need_items.borrow();
        for i in its {
            items.insert(i.to_string());
        }
        if items.len() > 0 {
            print!("Need the following items:\n");
            for s in items {
                print!("   {}\n", s);
            }
        };
    }

    fn get_normal_pile_count(&self) -> UShort {
        self.state.current_normal_pile_count
    }

    fn get_required_count(&self) -> UShort {
        self.state.required_cards
    }

    fn contains(&self, p: &PilePtr) -> bool {
        return self.state.contains(p);
    }

    fn get_piles(&self) -> &Piles {
        &self.state.piles
    }

    fn get_cards(&self) -> &Cards {
        &self.state.cards
    }

    fn has_note(&self, s: &String) -> bool {
        self.state.notes.contains(s)
    }

    fn get_general_pile(&self) -> Option<PilePtr> {
        match self.state.begin_general.borrow_mut().next() {
            None => None,
            Some(v) => Some(v.clone()),
        }
    }

    fn get_cost_set(&self) -> &CostSet {
        &self.state.costs_in_supply
    }

    fn need_to_check_costtargets(&self) -> bool {
        *self.state.target_check_required.borrow()
    }

    fn set_need_to_check(&self, v: bool, s: &String) {
        self.state.set_need_to_check(v, s);
    }

    fn get_target_set(&self) -> &Targets {
        &self.state.targets
    }

    fn get_collection(&self) -> &CardCollectionPtr {
        &self.state.card_coll
    }

    fn get_interacts_keywords(&self) -> &BTreeMap<String, Unsigned> {
        &self.state.interacts_keywords
    }

    fn get_keywords(&self) -> &BTreeMap<String, Unsigned> {
        &self.state.keywords
    }
}

type StringMultiMap = std::collections::BTreeMap<String, Vec<String>>;

fn split_once<'a>(s: &'a str, sep: char) -> Option<(&'a str, &'a str)> {
    for (pos, c) in s.char_indices() {
        if c == sep {
            if pos == 0 {
                return Some((&s[0..0], &s[sep.len_utf8()..]));
            }
            return Some((&s[0..pos], &s[pos + sep.len_utf8()..]));
        }
    }
    return None;
}

fn organise_args(
    args: &Vec<String>,
    legal_options: &HashMap<String, String>,
) -> Result<StringMultiMap, String> {
    let mut m = StringMultiMap::new();
    for s in args {
        let temp: &String = s;
        match split_once(s, '=') {
            None => {
                if !legal_options.contains_key(&temp.to_string()) {
                    return Result::Err(format!("Unknown option {}", temp));
                }
                m.insert(s.clone(), vec!["_".to_string()]);
            }
            Some((lhs, rhs)) => {
                let toks = rhs.split(",");
                if !legal_options.contains_key(lhs) {
                    return Err(format!("Unknown option {}", lhs));
                }
                let e = m.entry(lhs.to_string()).or_insert(vec![]);
                for t in toks {
                    e.push(t.to_string());
                }
            }
        }
    }
    Ok(m)
}

fn show_options(legal_options: &HashMap<String, String>) {
    println!("decker [options]");
    for (k, v) in legal_options {
        println!("  {}  {}", k, v);
    }
    println!();
    println!("Arguments which take further input are of the form --opt=a,b,c")
}

fn short_value(s: &str) -> Short {
    match s.parse::<Short>() {
        Ok(v) => v,
        Err(_) => -1,
    }
}

fn ushort_value(s: &str) -> UShort {
    match s.parse::<UShort>() {
        Ok(v) => v,
        Err(_) => 0,
    }
}

fn bool_value(s: &str) -> bool {
    s == "Y" || s == "y"
}

fn unsigned_value(s: &str) -> Unsigned {
    match s.parse::<Unsigned>() {
        Ok(v) => v,
        Err(_) => 0,
    }
}

fn decode_cost(s: &str) -> Option<TargetPtr> {
    let matches_required = 6;
    let unmet_weight = 3;
    let met_weight = 1;
    let upto_matches = 3;
    let cost_bound = 30;
    if s.starts_with("cost<=+") {
        let value = short_value(&s["cost<=+".len()..]);
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
    } else if s.starts_with("cost<=-") {
        let value = short_value(&s["cost<=-".len()..]);
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
    } else if s.starts_with("cost<=") {
        let value = short_value(&s["cost<=".len()..]);
        if value <= 0 || value > cost_bound {
            return None;
        }
        return Some(Rc::new(CostUpto::new(
            upto_matches,
            unmet_weight,
            met_weight,
            value,
        )));
    } else if s.starts_with("cost=+") {
        let value = short_value(&s["cost=+".len()..]);
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
    } else if s.starts_with("cost=-") {
        let value = short_value(&s["cost=-".len()..]);
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
    } else if s.starts_with("cost>=") {
        let value = short_value(&s["cost>=".len()..]);
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
    return None;
}

fn string_split(s: &String, sep: char) -> Vec<String> {
    let mut v = vec![];
    for i in s.split(sep) {
        v.push(i.to_string());
    }
    v
}

fn no_empty_split(s: &String, sep: char) -> Vec<String> {
    if s.len() == 0 {
        return Vec::<String>::new();
    }
    string_split(s, sep)
}

fn make_card(fields: &Vec<String>) -> Option<Card> {
    const NAMECOL: usize = 0;
    const PILECOL: usize = 1;
    const SETCOL: usize = 2;
    const SUPPLYCOL: usize = 3;
    const KINGDOMCOL: usize = 4;
    const TYPECOL: usize = 5;
    const COINCOST: usize = 6;
    //     let SPENDPOW=7;
    const DEBTCOST: usize = 8;
    const POTIONCOST: usize = 9;
    //     let POINTSCOL=10;
    const KEYWORDSCOL: usize = 11;
    const INTERACTKEY: usize = 12;
    const INTERACTOTHER: usize = 13;
    const END_COL: usize = INTERACTOTHER + 1;
    if fields.len() < END_COL {
        return None;
    }
    let coin_cost = short_value(&fields[COINCOST]);
    let has_coin = coin_cost != -1;
    let potion_cost = short_value(&fields[POTIONCOST]);
    let has_potion = potion_cost != -1;
    let debt_cost = short_value(&fields[DEBTCOST]);
    let has_debt = debt_cost != -1;
    let c = Cost::new(
        coin_cost,
        has_coin,
        potion_cost,
        has_potion,
        debt_cost,
        has_debt,
    );
    let in_supply = bool_value(&fields[SUPPLYCOL]);
    let is_kingdom = bool_value(&fields[KINGDOMCOL]);

    let types = no_empty_split(&fields[TYPECOL], ';');
    let keywords = no_empty_split(&fields[KEYWORDSCOL], ';');
    let interacts_kw = no_empty_split(&fields[INTERACTKEY], ';');
    let interacts_other = no_empty_split(&fields[INTERACTOTHER], ';');
    let mut targets: Targets = vec![];

    // Recognise cost constraints and check
    for s in &interacts_other {
        if s.contains("(") && !s.ends_with(")") {
            return None;
        }
        if s.starts_with("cost") {
            match decode_cost(&s) {
                None => {
                    return None;
                }
                Some(c) => {
                    targets.push(c);
                }
            }
        }
    }
    return Some(Card::new(
        &fields[NAMECOL],
        &fields[PILECOL],
        &fields[SETCOL],
        in_supply,
        is_kingdom,
        types,
        &c,
        keywords,
        interacts_kw,
        interacts_other,
        targets,
    ));
}

fn read_boxes(fname: &String) -> Result<StringMultiMap, String> {
    let ifs = match File::open(Path::new(fname)) {
        Err(_) => return Err("Can't open file".to_string()),
        Ok(f) => f,
    };
    let input = BufReader::new(ifs);
    let mut num = 0;
    let mut res = StringMultiMap::new();
    for item in input.lines() {
        let line;
        match item {
            Err(_) => break,
            Ok(l) => line = l,
        }
        num += 1;
        if line.starts_with('#') || line.len() == 0 {
            continue;
        }
        let split_eq = string_split(&line, '=');
        if split_eq.len() != 2 || split_eq[0].len() == 0 || split_eq[1].len() == 0 {
            return Err(format!("Can't parse line {}", num));
        }
        let groups = string_split(&split_eq[1], ';');
        for g in groups {
            let e = res.entry(split_eq[0].to_string()).or_insert(vec![]);
            e.push(g);
        }
    }
    return Ok(res);
}

fn caps(v: usize) -> Short {
    match Short::try_from(v) {
        Ok(res) => res,
        Err(_) => Short::MAX - 1,
    }
}

fn capus(v: usize) -> UShort {
    match UShort::try_from(v) {
        Ok(res) => res,
        Err(_) => UShort::MAX - 1,
    }
}

fn capu(v: usize) -> Unsigned {
    match Unsigned::try_from(v) {
        Ok(res) => res,
        Err(_) => Unsigned::MAX - 1,
    }
}

fn group_name_prefix(group_name: &str) -> String {
    match split_once(group_name, '-') {
        None => group_name.to_string(),
        Some((lhs, _)) => lhs.to_string(),
    }
}

fn load_cards(
    card_filename: &String,
    exclude_names: &mut HashMap<String, bool>,
) -> Result<Vec<PilePtr>, String> {
    let mut card_piles: Vec<Pile> = vec![];

    // piles we need to remove before handing card_piles back
    let mut remove_piles: BTreeSet<usize> = BTreeSet::new();
    // find things in card_piles using their name
    let mut p_map: BTreeMap<String, usize> = BTreeMap::new();

    let ifs = match File::open(Path::new(card_filename)) {
        Err(_) => return Err("Can't open file".to_string()),
        Ok(f) => f,
    };

    let mut line = String::new();
    let mut input = BufReader::new(ifs);

    // just want to get rid of the line
    match input.read_line(&mut line) {
        Ok(_) => {}
        Err(_) => {}
    }

    let mut linecount: u16 = 1;
    let mut error: String = "".to_string();

    for item in input.lines() {
        let line: String;
        match item {
            Err(_) => continue,
            Ok(l) => line = l,
        }
        linecount += 1;
        if line.starts_with(',') {
            continue;
        }
        let comp = string_split(&line, ',');
        let c = match make_card(&comp) {
            None => {
                error = format!("{}Error parsing card line {}\n", error, linecount);
                continue;
            }
            Some(c) => c,
        };

        let pile_name = if c.get_pile_name().len() == 0 {
            c.get_name()
        } else {
            c.get_pile_name()
        };
        // find or make the pile (borrowed)
        let index = match p_map.get(pile_name) {
            Some(&i) => i,
            None => {
                let newpile = Pile::new(pile_name);
                let new_index = card_piles.len();
                card_piles.push(newpile);
                p_map.insert(pile_name.to_string(), new_index);
                new_index
            }
        };
        // need to shift this up here because
        // it used to be after the addCard and you can't borrow
        // after move
        match exclude_names.get_mut(c.get_name()) {
            Some(v) => {
                remove_piles.insert(index);
                // It feels weird to have to do this
                *v = true;
            }
            None => {}
        }
        card_piles[index].add_card(c);

        // this is where we would handle comments but we aren't storing them
    }

    // The efficient way to do this would be to sort
    // the indices and remove them in decreasing order
    // But instead I'll sweep and filter
    // I'll combine this with making PilePtrs

    let mut result_piles: Vec<PilePtr> = vec![];
    let mut index = 0;
    for p in card_piles.into_iter() {
        if !remove_piles.contains(&index) {
            result_piles.push(PilePtr::new(p));
        }
        index += 1;
    }
    if error.len() == 0
    // only check for unknown card names if no error
    {
        for (k, v) in exclude_names {
            if !*v {
                error = format!("Unknown card {}", k);
                break;
            }
        }
    }
    if error.len() != 0 {
        return Err(error);
    }
    return Ok(result_piles);
}

// less need to do reference params to get around multiple ret / error ret
fn load_config(args: Vec<String>, card_file: String, box_file: String) -> Result<Config, String> {
    let mut err: String = "".to_string();
    let legal_options = get_legal_options();
    let mut m = match organise_args(&args, &legal_options) {
        Err(s) => {
            return Err(s);
        }
        Ok(m) => m,
    };
    if m.contains_key("--help") {
        show_options(&legal_options);
        // This should not return a config but it isn't really an error
        // I'll send back an empty string
        return Err("".to_string()); // this is not correct but I need a marker
    };
    // build an exclusion list of cardnames to drop
    let mut exclude_names: HashMap<String, bool> = HashMap::new();
    match m.get(&"--exclude".to_string()) {
        Some(v) => {
            for p in v {
                exclude_names.insert(p.to_string(), false);
            }
        }
        None => {}
    };
    // groups we need
    let mut required_groups: HashMap<String, bool> = HashMap::new();
    let card_filename = match m.get(&"--cardfile".to_string()) {
        Some(v) => {
            if v.len() == 0 || v[0].len() == 0 {
                return Err("No card file specified".to_string());
            }
            &v[0]
        }
        None => &card_file,
    };
    let temp_piles = match load_cards(&card_filename, &mut exclude_names) {
        Err(s) => {
            return Err(s);
        }
        Ok(v) => v,
    };

    match m.get(&"--boxes".to_string()) {
        Some(box_names) => {
            let box_filename = match m.get(&"--boxfile".to_string()) {
                Some(f) => {
                    if f.len() == 0 {
                        "".to_string()
                    } else {
                        f[0].clone()
                    }
                }
                None => box_file,
            };
            if box_filename.len() == 0 {
                return Err("No box file specified.".to_string());
            }
            let box_to_set = match read_boxes(&box_filename) {
                Err(s) => return Err(s),
                Ok(v) => v,
            };
            if box_to_set.len() == 0 {
                return Err("--boxes specified but no boxes known (use --boxfile).".to_string());
            };
            // now we start processing the --boxes param
            for bp in box_names {
                match box_to_set.get(bp) {
                    None => {
                        return Err(format!("Box {} not known in box file {}", bp, box_filename))
                    }
                    Some(e) => {
                        for name in e {
                            required_groups.insert(name.to_string(), false);
                        }
                    }
                };
            }
        }
        None => (),
    };

    let mut p_set = PileSet::new();

    match m.get(&"--groups".to_string()) {
        Some(e) => {
            for v in e {
                required_groups.insert(v.to_string(), false);
            }
        }
        None => (),
    };
    if required_groups.len() > 0 {
        required_groups.insert("base".to_string(), false); // avoid invalid games
        let mut to_drop: Vec<String> = vec![];
        for pile in temp_piles {
            let group_name = pile.get_card_group().to_string();
            match required_groups.get_mut(&group_name) {
                Some(g) => {
                    p_set.insert(pile.clone());
                    *g = true;
                }
                None => to_drop.push(group_name.clone()),
            };
        }
        // now we check to see if all groups are known
        for (n, b) in required_groups {
            if !b {
                if err.len() != 0 {
                    err += "\n";
                }
                err = format!("{}Unknown group {}", err, n);
            };
        }
    } else {
        for p in temp_piles {
            p_set.insert(p); // add all piles
        }
    };
    let mut include_piles: PileSet = PileSet::new();
    match m.get(&"--include".to_string()) {
        Some(e) => {
            for name in e {
                let mut found = false;
                'overset: for pile in &p_set {
                    for c in pile.get_cards() {
                        if c.get_name() == name {
                            include_piles.insert(pile.clone());
                            found = true;
                            break 'overset;
                        }
                    }
                }
                if !found {
                    return Err(format!("Can't find card {}", name));
                };
            }
        }
        None => (),
    };

    let mut min_types: HashMap<String, UShort> = HashMap::new();
    match m.get("--min-type") {
        None => (),
        Some(v) => {
            for s in v {
                match split_once(s, ':') {
                    None => continue,
                    Some((lhs, rhs)) => {
                        if lhs.len() == 0 {
                            continue;
                        }
                        let typecount = ushort_value(rhs);
                        min_types.insert(lhs.to_string(), typecount);
                    }
                }
            }
        }
    };
    let mut max_types: HashMap<String, UShort> = HashMap::new();
    match m.get("--max-type") {
        None => (),
        Some(v) => {
            for s in v {
                match split_once(s, ':') {
                    None => continue,
                    Some((lhs, rhs)) => {
                        if lhs.len() == 0 {
                            continue;
                        }
                        let typecount = ushort_value(rhs);
                        max_types.insert(lhs.to_string(), typecount);
                    }
                }
            }
        }
    };
    let mut use_bad_rand = false;
    match m.get(&"--badrand".to_string()) {
        None => (),
        Some(_) => use_bad_rand = true,
    };
    let mut seed: Unsigned = 0;
    let mut chose_seed = false;
    match m.get(&"--seed".to_string()) {
        None => (),
        Some(v) => {
            if v.len() > 0 {
                seed = unsigned_value(&v[0]);
                chose_seed = true;
            };
        }
    };
    let mut max_cost_repeat = 0;
    match m.get(&"--max-cost-repeat".to_string()) {
        None => (),
        Some(v) => {
            if v.len() > 0 {
                max_cost_repeat = ushort_value(&v[0]);
            }
        }
    };
    let mut validate = true;
    match m.get(&"--no-validate".to_string()) {
        None => (),
        Some(v) => {
            if v.len() > 0 {
                validate = bool_value(&v[0]);
            }
        }
    };
    let mut list_collection = false;
    match m.get(&"--list".to_string()) {
        None => (),
        Some(_) => {
            list_collection = true;
        }
    };
    // now we set up randomiser
    // The cap here is arbitrary (want it to be at least as big
    // as 3 x pile size for shuffling
    let mut rand = get_rand_stream(seed, capu(10 * p_set.len()), use_bad_rand);
    if !chose_seed {
        let seed = rand.init_seed();
        m.insert("--seed".to_string(), vec![seed.to_string()]);
    };

    // Now we limit the groups we can draw from.
    // We'll do it here before anything gets into the
    // CardCollection object.
    // We need to take into account which prefixes any
    // --include cards force in (and also if that exceeds
    // the limit
    match m.get(&"--max-prefixes".to_string()) {
        None => (),
        Some(v) => {
            if v.len() > 0 {
                let mut suggested_max = ushort_value(&v[0]);
                if suggested_max > 0 {
                    suggested_max += 1;
                    let mut chosen_prefixes = HashSet::<String>::new();
                    chosen_prefixes.insert("base".to_string());
                    for ip in &include_piles {
                        chosen_prefixes.insert(group_name_prefix(ip.get_card_group()));
                    }
                    if capus(chosen_prefixes.len()) > suggested_max {
                        // need to -1 from both numbers because of hidden "base" group
                        return Err(format!("Requested at most {} big groups, but included cards are drawn from {}.", suggested_max-1, chosen_prefixes.len()-1));
                    };
                    let mut group_prefixes = BTreeSet::<String>::new();
                    for p in &p_set {
                        group_prefixes.insert(group_name_prefix(p.get_card_group()));
                    }
                    // now shuffle the prefixes
                    let mut shuffle_prefixes: Vec<String> = vec![];
                    for s in group_prefixes {
                        shuffle_prefixes.push(s);
                    }
                    let n_sets = caps(shuffle_prefixes.len());
                    // wow, could have named these better
                    for _i in 0..3 {
                        for i in 0..(n_sets as usize) {
                            let pos: usize = (rand.get() % (n_sets as u64)) as usize;
                            if i != pos {
                                shuffle_prefixes.swap(i, pos);
                            }
                        }
                    }
                    let mut i = 0;
                    while i < n_sets && chosen_prefixes.len() < (suggested_max as usize) {
                        chosen_prefixes.insert(shuffle_prefixes[i as usize].to_string());
                        i += 1;
                    }
                    let mut new_files = PileSet::new();
                    for p2 in p_set {
                        if chosen_prefixes.contains(&group_name_prefix(p2.get_card_group())) {
                            new_files.insert(p2.clone());
                        }
                    }
                    p_set = new_files;
                };
            }
        }
    };
    // Now let's work out how many optional extras we need
    let opt_extra = match m.get(&"--landscape-count".to_string()) {
        None => {
            let x = (rand.get() % 7) as UShort;
            if x < 3 {
                x
            } else {
                0
            }
        }
        Some(v) => {
            if v.len() > 0 {
                ushort_value(&v[0])
            } else {
                0
            }
        }
    };
    let why = match m.get(&"--why".to_string()) {
        None => false,
        Some(_) => true,
    };
    let more_info = match m.get(&"--info".to_string()) {
        None => false,
        Some(_) => true,
    };
    let disable_anti_cursors = match m.get(&"--no-anti-cursor".to_string()) {
        None => false,
        Some(_) => true,
    };
    let disable_attack_react = match m.get(&"--no-attack-react".to_string()) {
        None => false,
        Some(_) => true,
    };

    Ok(Config {
        args: m,
        rand: Box::new(rand),
        why: why,
        more_info: more_info,
        optional_extras: opt_extra,
        validate: validate,
        list_collection: list_collection,
        disable_anti_cursors: disable_anti_cursors,
        disable_attack_react: disable_attack_react,
        max_cost_repeat: max_cost_repeat,
        min_types: min_types,
        max_types: max_types,
        piles: p_set,
        includes: include_piles,
    })
}

#[derive(Clone)]
struct CollectionIterator {
    col: CardCollectionPtr,
    vec_num: usize,
    index: usize,
}

impl CollectionIterator {}

impl Iterator for CollectionIterator {
    type Item = PilePtr;
    fn next(&mut self) -> Option<Self::Item> {
        match self.col.get_pile_item(self.vec_num, self.index) {
            None => None,
            Some(v) => {
                self.index += 1;
                Some(v)
            }
        }
    }
}

// so we only need to do one lock
struct PropLists {
    lists: Vec<Vec<PilePtr>>,
    map: HashMap<PropertyPtr, usize>,
}

struct CollectionState {
    legal_costs: CostSet,
    cards: HashSet<CardPtr>,
    general_property: PropertyPtr,
    group_names: BTreeSet<String>,
    card_names: BTreeSet<String>,
    piles: Vec<PilePtr>,
    lists: RefCell<PropLists>,
}

impl CollectionState {
    fn validate_collection(&self) -> CollectionStatus {
        let mut warnings: Vec<String> = vec![];
        for p in &self.piles {
            for c in p.get_cards() {
                for inter in c.get_other_interactions() {
                    if inter.starts_with("card(") {
                        let target = inter["card(".len()..inter.len() - 1].to_string();
                        if !self.card_names.contains(&target) {
                            warnings.push(format!(
                                "Card {} interacts with {} but it is missing.",
                                c.get_name(),
                                target
                            ));
                        }
                    } else if inter.starts_with("group(") {
                        let target = inter["group(".len()..inter.len() - 1].to_string();
                        if !self.group_names.contains(&target) {
                            warnings.push(format!(
                                "Card {} interacts with group {} but it is missing.",
                                c.get_name(),
                                target
                            ));
                        }
                    }
                }
            }
        }
        if warnings.len() == 0 {
            CollectionStatus::CollOK
        } else {
            CollectionStatus::CollWarning(warnings)
        }
    }

    fn shuffle(&mut self, r: &mut Box<dyn RandStream>) {
        let us = self.piles.len();
        let size: u64 = match us.try_into() {
            Ok(v) => v,
            Err(_) => 10, // arbitrary result
        };
        // go through the pile vector 3 times and swap items
        for _i in 0..3 {
            for j in 0..us {
                let pos: usize = match (r.get() % size).try_into() {
                    Ok(v) => v,
                    Err(_) => 10, // arbitrary result
                };
                self.piles.swap(pos, j);
            }
        }
    }
}

// Rename this once I've got it done
#[derive(Clone)]
struct CardColl {
    state: Rc<CollectionState>,
}

impl CardColl {
    // Wrapper for starting, building and finishing a selection
    fn generate_selection(
        &self,
        market_cap: UShort,
        landscapes: UShort,
        includes: &PileSet,
        cons: &Vec<ConstraintPtr>,
        rand: &mut Box<dyn RandStream>,
    ) -> Result<SelectionPtr, String> {
        let mut sel = match self.start_selection(market_cap, landscapes) {
            Some(s) => s,
            None => return Err("".to_string()),
        };
        for c in cons {
            sel.add_constraint(c.clone())
        }
        for p in includes {
            sel.add_pile(p);
            sel.tag_pile(p, &"<why?--included>".to_string());
        }
        let mut res = match self.build_selection(&SelectionPtr::from_state(sel)) {
            Ok(s) => s,
            Err(m) => return Err(m),
        };
        let state = match Rc::get_mut(&mut res.state) {
            Some(r) => r,
            None => return Err("Unexpected reference count".to_string()),
        };
        self.finish_selection(state, rand);
        return Ok(res);
    }

    fn start_selection(&self, market_cap: UShort, landscapes: UShort) -> Option<SelectionState> {
        let base = CardGroupProperty::make_ptr(&"base".to_string());
        let begin = match self.get_iterators(&base) {
            Some(v) => v,
            None => return None,
        };
        let begin_general = match self.get_iterators(&self.state.general_property) {
            Some(v) => v,
            None => {
                return None; // This is different from c++
            }
        };
        let mut new_sel = if market_cap == 0 {
            SelectionState::new(self, begin_general)
        } else {
            SelectionState::new1(self, begin_general, market_cap)
        };
        for i in begin {
            if !new_sel.add_pile(&i)
            // you should never fail to add base cards
            {
                return None; // different from c++
            }
        }
        if landscapes > 0 {
            let oep = OptionalExtraProperty::make_ptr();
            if let Some(begin) = self.get_iterators(&oep) {
                // in c++ this was a multi-condition for loop
                let mut count = 0;
                for i in begin {
                    if count >= landscapes {
                        break;
                    };
                    if !new_sel.add_pile(&i)
                    // should not be able to fail adding
                    {
                        return None; // different from c++
                    };
                    count += 1;
                }
            };
        };
        return Some(new_sel);
    }

    // Cleanup to check for extra elements like vp tokens which
    // don't really need a constraint to catch
    fn finish_selection(&self, sel: &mut SelectionState, rand: &mut Box<dyn RandStream>) {
        // check to see if we need to add DarkAges-base cards
        // rules say to do it based on randomness eg the last card added
        // but we don't know what order things were drawn
        // do we have any DarkAges cards
        let mut da_count = 0;
        let mut ks_count = 0; // count of kingdom and supply
        for p in sel.get_piles() {
            if p.get_supply() && p.get_kingdom() {
                ks_count += 1;
                if p.get_card_group() == "DarkAges" {
                    da_count += 1;
                }
            }
        }
        if da_count > 0 {
            // if the random is less than the number of number of
            // DarkAges cards, add the DarkAges base cards to replace Estate
            let r = rand.get() % ks_count;
            if r < da_count {
                // need to add all piles from that group
                let ps = CardGroupProperty::make_ptr(&"DarkAges-base".to_string());

                // If we can't add this for some reason do nothing
                if let Some(begin) = sel.get_collection().get_iterators(&ps) {
                    for p in begin {
                        if sel.add_pile(&p) {
                            sel.tag_pile(&p, &"<why?had enough DarkAges cards>".to_string());
                            sel.tag_pile(&p, &"Replaces Estate in starting deck".to_string());
                        }
                    }
                    sel.add_note(&"addedDarkAges-base".to_string());
                }
            }
        }
        for p in sel.get_piles() {
            if p.get_keywords().contains("+point") {
                sel.add_item(&"points(shield) tokens".to_string());
                break;
            }
        }
        for p in sel.get_piles() {
            for c in p.get_costs()
            // yes this loop runs longer than it needs to
            {
                if c.has_debt() {
                    sel.add_item(&"debt tokens".to_string());
                    break;
                }
            }
        }
        for p in sel.get_piles() {
            if p.get_keywords().contains("+coffers") {
                sel.add_item(&"coin tokens".to_string());
                sel.add_item(&"coffers/villagers mat".to_string());
                break;
            }
        }
        for p in sel.get_piles() {
            if p.get_keywords().contains("+villagers") {
                sel.add_item(&"coin tokens".to_string());
                sel.add_item(&"coffers/villagers mat".to_string());
                break;
            }
        }
        for p in sel.get_piles() {
            if p.get_types().contains("Heirloom") {
                sel.tag_pile(&p, &"Replaces one Copper in starting deck".to_string());
                // no break because multiple could be in play
            }
        }
    }

    fn get_pile_item(&self, vec_num: usize, index: usize) -> Option<PilePtr> {
        let contents: &CollectionState = self.state.borrow();
        if vec_num >= contents.lists.borrow().lists.len() {
            None
        } else if index < contents.lists.borrow().lists[vec_num].len() {
            Some(contents.lists.borrow().lists[vec_num][index].clone())
        } else {
            None
        }
    }

    fn new_state(piles: &PileSet) -> CollectionState {
        let mut temp_vector = vec![];
        for v in piles {
            temp_vector.push(SortablePile { p: v.clone() });
        }
        temp_vector.sort();
        let mut piles_vector = vec![];
        for v in temp_vector {
            piles_vector.push(v.p);
        }
        let mut cards = CardSet::new();
        let mut card_names = BTreeSet::<String>::new();
        let mut group_names = BTreeSet::<String>::new();
        let mut legal_costs = HashSet::<Cost>::new();
        // I'll try pulling the details out here rather than passing them in
        for p in piles {
            group_names.insert(p.get_card_group().to_string());
            for c in p.get_cards() {
                card_names.insert(c.get_name().to_string());
                cards.insert(c.clone());
                legal_costs.insert(*c.get_cost());
            }
        }

        CollectionState {
            legal_costs: legal_costs,
            cards: cards,
            general_property: KingdomAndSupplyProperty::make_ptr(),
            group_names: group_names,
            card_names: card_names,
            piles: piles_vector,
            lists: RefCell::new(PropLists {
                lists: vec![],
                map: HashMap::new(),
            }),
        }
    }

    fn from_state(c: CollectionState) -> CardColl {
        CardColl { state: Rc::new(c) }
    }

    fn get_iterators(&self, p: &PropertyPtr) -> Option<CollectionIterator> {
        let mut list_contents = self.state.lists.borrow_mut();
        // do we have this one?
        match list_contents.map.get(p) {
            Some(pos) => Some(CollectionIterator {
                col: self.clone(),
                vec_num: *pos,
                index: 0,
            }),
            None => {
                // populate a new vector
                if p.is_selection_property() {
                    return None;
                };
                let mut newv = vec![];
                for pil in &self.state.piles {
                    if p.pile_meets(&pil) {
                        newv.push(pil.clone());
                    }
                }
                if newv.len() == 0 {
                    return None;
                }

                list_contents.lists.push(newv);
                let lists_count = list_contents.lists.len() - 1;
                list_contents.map.insert(p.clone(), lists_count);
                Some(CollectionIterator {
                    col: self.clone(),
                    vec_num: lists_count,
                    index: 0,
                })
            }
        }
    }

    fn get_piles(&self) -> &Vec<PilePtr> {
        &self.state.piles
    }

    // 1. check all constraints to see if anything needs to be fixed from current cards
    // 2. Make sure we aren't trying to add more (supply) cards than we are allowed
    // 3. If the current selection meets all requirements, set result to that selection and return true
    // 4. If some requirement can not be met (eg a call to act or addPile fails), return false.
    // 5. If we need more cards, pick a new pile to add and recurse
    // This method _may_ modify the "start" it is given (instead of making a new clone
    //  to mod). So you should clone your selection before calling this.
    // ConstraintAction::act is expected to:
    //     1. make whatever changes
    //     2. call buildSelection() with the resulting selection
    //     3. return the result of the build selection
    //
    // Checked are done in the following order:
    // 1. Are any constraints failed?
    // 2. Do any constraints require action?
    // 3. Do any constraints have suggested actions?
    // 4. Anything to add based on cost targets?
    // 5. Try adding general cards (list of all available cards).
    fn build_selection(&self, start: &SelectionPtr) -> Result<SelectionPtr, String> {
        let temp: &RefCell<Vec<ConstraintPtr>> = start.state.constraints.borrow();
        let constraints = temp.borrow();
        //let constraints:&mut Vec<ConstraintPtr> = start.state.constraints.borrow_mut();
        let size = constraints.len();
        let mut status: Vec<ConsResult> = vec![];
        status.reserve(size);
        // see if we are breaking any constraints
        for c in &*constraints {
            let stat = c.get_status(start);
            if stat == ConsFail {
                return Err("Constraint Fail".to_string());
            }
            status.push(stat)
        }
        // we haven't "failed" constraints but do we still need action
        // and would that action put us over pile limit?
        let mut supply_cap = false;
        if start.get_normal_pile_count() == start.get_required_count() {
            supply_cap = true; //. we don't return immediately because we might
                               // need to add non-supply cards to fix something
        }

        for it in 0..size {
            if status[it] == ConsActionReq {
                return constraints[it].act(start);
            }
        }
        if supply_cap {
            return Ok(SelectionPtr {
                state: start.state.clone(),
            });
            //return Ok(start.clone());
        }
        for it in 0..size {
            if status[it] == ConsMorePossible {
                // should take action on this constraint
                break;
            }
        }
        // do we need to consider cost targets?
        if start.need_to_check_costtargets() {
            const HAVE_COST_PENALTY: f32 = -3.0;
            const THRESHOLD: f32 = 0.5;
            const TOLERANCE: f32 = 0.21; // 0.2 was resulting in non-determinism

            let mut need_target_action = false;

            let costs = start.get_cost_set();
            let mut votes = CostVotes::new(self.state.legal_costs.clone());

            for tar in start.get_target_set() {
                need_target_action = tar.add_votes(costs, &mut votes) || need_target_action;
            }
            // Now we need to take into account the costs where we already have a pile
            for c in costs {
                votes.add_vote(c, HAVE_COST_PENALTY);
            }
            // Two possibilities to consider here
            // A) there is an unmet target ... interate through all possibles
            //      to find a card that works
            // B) The flag is still set but all targets are minimally
            //    satisfied. In which case, try the first card which matches
            //    if it works, fine. If not, stop
            let mut max_cost = CostSet::new();
            if votes.get_max_weighted(&mut max_cost, THRESHOLD, TOLERANCE) {
                let cp = CostProperty::make_ptr_set(max_cost, true);
                match self.get_iterators(&cp) {
                    None => {
                        // couldn't find matching costs
                        if need_target_action
                        // need to check if we _needed_ it
                        {
                            return Err("Needed target action".to_string());
                        }
                    }
                    Some(mut begin) => {
                        // The original version won't return empty iterators
                        // so do while and while will be equivalent
                        while let Some(next) = begin.next() {
                            if start.contains(&next) {
                                continue;
                            }
                            let mut new_sel = start.duplicate_state();
                            if !need_target_action {
                                new_sel.set_need_to_check(false, &"".to_string());
                            }
                            let blame = new_sel.get_target_string().to_string();
                            if !new_sel.add_pile(&next) {
                                if !need_target_action {
                                    // We didn't need this card
                                    // we'll try later options
                                    start.set_need_to_check(false, &"".to_string()); // *start* is not a mistake
                                    break; // need to prevent _current_ selection from seeking more, not just the recursive one
                                } else {
                                    return Err("".to_string());
                                }
                            }
                            // need to work out how to give more useful feedback
                            let why = format!("<why?cost-target:{}>", blame);
                            new_sel.tag_pile(&next, &why);
                            match self.build_selection(&SelectionPtr::from_state(new_sel)) {
                                Ok(s) => return Ok(s),
                                Err(_) => (),
                            }
                        }
                    }
                }
            }
        }
        // If we get to this point, this selection (start)
        // can't be looking to costtargets for help so
        start.set_need_to_check(false, &"".to_string());

        // we don't have any constraints to guide us so add a general pile
        // Note: this method of preventing lower levels from considering a pile if
        // an upper level has already tried that pile should be ok _provided_ that
        // Some action by an intermediate level hasn't made a previously invalid card
        // valid.
        while let Some(gen) = start.get_general_pile() {
            if start.contains(&gen) {
                continue;
            }
            let mut new_sel = start.duplicate_state();
            if !new_sel.add_pile(&gen) {
                return Err("".to_string());
            }
            new_sel.tag_pile(&gen, &"<why?general>".to_string());
            if let Ok(v) = self.build_selection(&SelectionPtr::from_state(new_sel)) {
                return Ok(v);
            }
        }
        return Err("".to_string());
    }

    fn get_pile_for_card(&self, s: &String) -> Option<PilePtr> {
        for c in &self.state.cards {
            if c.get_name() == s {
                let pn = if c.get_pile_name().len() == 0 {
                    s
                } else {
                    c.get_pile_name()
                };
                for it2 in &self.state.piles {
                    if it2.get_name() == pn {
                        return Some(it2.clone());
                    }
                }
            }
        }
        return None;
    }
}

enum CollectionStatus {
    CollOK,
    CollWarning(Vec<String>),
    _CollFatal(Vec<String>),
}

type CardCollectionPtr = CardColl;

fn main() {
    let mut args: Vec<String> = env::args().collect();
    args.remove(0);
    let mut conf = match load_config(args, "cards.dat".to_string(), "".to_string()) {
        Ok(v) => v,
        Err(e) => {
            print!("{}\n", e);
            exit(1);
        }
    };

    // Need to create the state separately, list, validate and sort

    let mut col = CardCollectionPtr::new_state(&conf.piles);
    if conf.validate {
        let warnings = match col.validate_collection() {
            CollectionStatus::CollOK => {
                vec![]
            }
            CollectionStatus::CollWarning(v) => v,
            CollectionStatus::_CollFatal(v) => v,
        };
        if warnings.len() != 0 {
            print!("Error validating collection:\n");
            for s in warnings {
                print!("{}\n", s);
            }
            exit(3);
        };
    };
    if conf.list_collection {
        for p in &conf.piles {
            print!("{}\n", p.get_name());
        }
        exit(0);
    };
    col.shuffle(&mut conf.rand);
    let col = CardCollectionPtr::from_state(col);
    let constraints = match conf.build_constraints(&col) {
        Ok(v) => v,
        Err(s) => {
            print!("{}\n", s);
            exit(4);
        }
    };
    let sel = match col.generate_selection(
        10,
        conf.optional_extras,
        &conf.includes,
        &constraints,
        &mut conf.rand,
    ) {
        Ok(s) => s,
        Err(m) => {
            eprint!("Error: empty selection\n");
            eprint!("Possible explanation: {}\n", m);
            exit(2);
        }
    };
    print!("Options:{}\n", conf.get_string());
    sel.dump(conf.why, conf.more_info);
}
