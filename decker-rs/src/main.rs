use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;

use std::convert::TryInto;

use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;


use std::convert::TryFrom;

use std::process::exit;
use std::rc::Rc;

mod card;
mod collection;
mod cost;
mod pile;
mod property;
mod selection;
mod constraint;

use card::*;
use cost::*;
use pile::*;
use collection::*;
use property::*;
use constraint::*;

static MAXCOINCOST: i8 = 11;

static MANY: u64 = 5000;

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

// To do random stream, I need traits
// This should go in a namespace eventually
pub trait RandStream {
    fn get(&mut self) -> u64;
    fn init_seed(&self) -> u64;
}

// This was hidden as an implementation detail in c++
// Can I do something like that here?
struct BadRand {
    seed: u64,
    cap: u64,
    step: u64,
    init: u64,
}

fn make_bad_rand(s: u64, bound: u64) -> BadRand {
    let cap = bound;
    let mut setstep: u64 = 1; // Not convinced this init is necessary
    let mut f = bound / 2 + 1;
    while f < cap {
        let mut i: u64 = 2;
        while i < f {
            if f % i == 0 {
                break;
            };
            i += 1;
        }
        if i == f {
            setstep = f;
            break;
        }
        f += 1;
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
    fn get(&mut self) -> u64 {
        if self.cap == 0 {
            return 0;
        }
        let newseed = (self.seed + self.step) % self.cap;
        self.seed = newseed;
        newseed
    }
    fn init_seed(&self) -> u64 {
        self.init
    }
}

fn get_rand_stream(s: u64, cap: u64, _use_bad_random: bool) -> impl RandStream {
    // eventually want to make this conditional on use_bad_random
    make_bad_rand(s, cap)
}

struct Config {
    args: StringMultiMap,
    rand: Box<dyn RandStream>,
    why: bool,
    more_info: bool,
    optional_extras: u8,
    validate: bool,
    list_collection: bool,
    disable_anti_cursors: bool,
    disable_attack_react: bool,
    max_cost_repeat: u8,
    min_types: HashMap<String, u8>,
    max_types: HashMap<String, u8>,
    piles: PileSet,
    includes: PileSet,
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
            &fail_prop,
            Some(AddGroup::make_ptr(col, &"Alchemy-base".to_string())),
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
            &fail_prop,
            Some(AddProsperity::make_ptr(col)),
            1,
            MANY,
            MANY,
            MANY,
        );
        cons.push(c);

        let c = Constraint::make_ptr_full(
            "AddInteractingGroup".to_string(),
            Some(MissingInteractingCardGroupProperty::make_ptr()),
            &fail_prop,
            Some(AddMissingDependencyGroup::make_ptr(col)),
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
            Some(AddMissingDependency::make_ptr(col)),
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
            Some(AddGroup::make_ptr(col, &"Nocturne-Hexes".to_string())),
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
            Some(AddGroup::make_ptr(col, &"Nocturne-Boons".to_string())),
            1,
            MANY,
            MANY,
            MANY,
        );
        cons.push(c);

        if !self.disable_anti_cursors {
            if let Some(c) = curser_constraint(col, 1) {
                cons.push(c);
            }
        }
        if !self.disable_attack_react {
            if let Some(c) = attack_react_constraint(col, 2) {
                cons.push(c);
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
                Some(FindPile::make_ptr(col, &t_begin)),
                (*type_count).into(),
                MANY,
            );
            cons.push(c);
        }
        for (k, v) in &self.max_types {
            let type_name = k;
            let type_count = v;

            let searcher = TypeProperty::make_ptr(type_name, false);
            if col.get_iterators(&searcher).is_some() {
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
                let p1 = KeywordProperty::make_ptr("gain", true);
                let p2 = KeywordProperty::make_ptr("+buy", true);
                let prop = EitherProperty::make_ptr(&p1, &p2);
                let name = "Provide interacted keyword (gain/+buy)";
                if let Some(begin) = col.get_iterators(&prop) {
                    let c = Constraint::make_ptr_full(
                        name.to_string(),
                        Some(HangingInteractsWith::make_ptr3(
                            &"gain".to_string(),
                            &"gain".to_string(),
                            &"+buy".to_string(),
                        )),
                        &fail_prop,
                        Some(FindPile::make_ptr(col, &begin)),
                        1,
                        MANY,
                        MANY,
                        MANY,
                    );
                    cons.push(c);
                }
            } else if s == "trash" {
                let p1 = KeywordProperty::make_ptr("trash_any", true);
                let p2 = KeywordProperty::make_ptr("trash_limited", true);
                let prop = EitherProperty::make_ptr(&p1, &p2);
                let name = "Provide interacted keyword (trash_any/trash_limited)";
                if let Some(begin) = col.get_iterators(&prop) {
                    let prop = HangingInteractsWith::make_ptr3(
                        &"trash".to_string(),
                        &"trash_limited".to_string(),
                        &"trash_any".to_string(),
                    );
                    let c = Constraint::make_ptr_full(
                        name.to_string(),
                        Some(prop),
                        &fail_prop,
                        Some(FindPile::make_ptr(col, &begin)),
                        1,
                        MANY,
                        MANY,
                        MANY,
                    );
                    cons.push(c);
                };
            } else {
                let prop = KeywordProperty::make_ptr(s, true);
                if let Some(begin) = col.get_iterators(&prop) {
                    let name = format!("Provide interacted keyword {}", s);
                    let c = Constraint::make_ptr_full(
                        name,
                        Some(HangingInteractsWith::make_ptr2(s, s)),
                        &fail_prop,
                        Some(FindPile::make_ptr(col, &begin)),
                        1,
                        MANY,
                        MANY,
                        MANY,
                    );
                    cons.push(c);
                };
            };
        }
        Ok(cons)
    }
}

type StringMultiMap = std::collections::BTreeMap<String, Vec<String>>;

fn split_once(s: &str, sep: char) -> Option<(&str, &str)> {
    for (pos, c) in s.char_indices() {
        if c == sep {
            if pos == 0 {
                return Some((&s[0..0], &s[sep.len_utf8()..]));
            }
            return Some((&s[0..pos], &s[pos + sep.len_utf8()..]));
        }
    }
    None
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
                let toks = rhs.split(',');
                if !legal_options.contains_key(lhs) {
                    return Err(format!("Unknown option {}", lhs));
                }
                let e = m.entry(lhs.to_string()).or_default();
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

fn short_value(s: &str) -> i8 {
    s.parse::<i8>().unwrap_or(-1)
}

fn ushort_value(s: &str) -> u8 {
    s.parse::<u8>().unwrap_or(0)
}

fn bool_value(s: &str) -> bool {
    s == "Y" || s == "y"
}

fn unsigned_value(s: &str) -> u64 {
    s.parse::<u64>().unwrap_or(0)
}

fn decode_cost(s: &str) -> Option<TargetPtr> {
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

fn string_split(s: &str, sep: char) -> Vec<String> {
    s.split(sep).map(|i| i.to_string()).collect()
}

fn no_empty_split(s: &String, sep: char) -> Vec<String> {
    if s.is_empty() {
        return Vec::<String>::new();
    }
    string_split(s, sep)
}

fn make_card(fields: &Vec<String>) -> Option<Card> {
    static NAMECOL: usize = 0;
    static PILECOL: usize = 1;
    static SETCOL: usize = 2;
    static SUPPLYCOL: usize = 3;
    static KINGDOMCOL: usize = 4;
    static TYPECOL: usize = 5;
    static COINCOST: usize = 6;
    //     let SPENDPOW=7;
    static DEBTCOST: usize = 8;
    static POTIONCOST: usize = 9;
    //     let POINTSCOL=10;
    static KEYWORDSCOL: usize = 11;
    static INTERACTKEY: usize = 12;
    static INTERACTOTHER: usize = 13;
    static END_COL: usize = INTERACTOTHER + 1;
    if fields.len() < END_COL {
        return None;
    }
    let c = Cost {
        coin: fields[COINCOST].parse().ok(),
        potion: fields[POTIONCOST].parse().ok(),
        debt: fields[DEBTCOST].parse().ok(),
    };

    let in_supply = bool_value(&fields[SUPPLYCOL]);
    let is_kingdom = bool_value(&fields[KINGDOMCOL]);

    let types = no_empty_split(&fields[TYPECOL], ';');
    let keywords = no_empty_split(&fields[KEYWORDSCOL], ';');
    let interacts_kw = no_empty_split(&fields[INTERACTKEY], ';');
    let interacts_other = no_empty_split(&fields[INTERACTOTHER], ';');
    let mut targets: Targets = vec![];

    // Recognise cost constraints and check
    for s in &interacts_other {
        if s.contains('(') && !s.ends_with(')') {
            return None;
        }
        if s.starts_with("cost") {
            match decode_cost(s) {
                None => {
                    return None;
                }
                Some(c) => {
                    targets.push(c);
                }
            }
        }
    }
    Some(Card::new(
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
    ))
}

fn read_boxes(fname: &String) -> Result<StringMultiMap, String> {
    let ifs = match File::open(Path::new(fname)) {
        Err(_) => return Err("Can't open file".to_string()),
        Ok(f) => f,
    };
    let input = BufReader::new(ifs);
    let mut res = StringMultiMap::new();
    for (num, item) in input.lines().enumerate() {
        let line = match item {
            Err(_) => break,
            Ok(l) => l,
        };
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let split_eq = string_split(&line, '=');
        if split_eq.len() != 2 || split_eq[0].is_empty() || split_eq[1].is_empty() {
            return Err(format!("Can't parse line {}", num));
        }
        let groups = string_split(&split_eq[1], ';');
        for g in groups {
            let e = res.entry(split_eq[0].to_string()).or_default();
            e.push(g);
        }
    }
    Ok(res)
}

fn caps(v: usize) -> i8 {
    match i8::try_from(v) {
        Ok(res) => res,
        Err(_) => i8::MAX - 1,
    }
}

fn capus(v: usize) -> u8 {
    match u8::try_from(v) {
        Ok(res) => res,
        Err(_) => u8::MAX - 1,
    }
}

fn capu(v: usize) -> u64 {
    match u64::try_from(v) {
        Ok(res) => res,
        Err(_) => u64::MAX - 1,
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

    let input = BufReader::new(ifs);

    let mut error: String = "".to_string();

    for (line_count, line) in input.lines().skip(1).filter_map(|l| l.ok()).enumerate() {
        if line.starts_with(',') {
            continue;
        }
        let comp = string_split(&line, ',');
        let c = match make_card(&comp) {
            None => {
                error = format!("{}Error parsing card line {}\n", error, line_count + 1);
                continue;
            }
            Some(c) => c,
        };

        let pile_name = if c.get_pile_name().is_empty() {
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
        if let Some(v) = exclude_names.get_mut(c.get_name()) {
            remove_piles.insert(index);
            // It feels weird to have to do this
            *v = true;
        }
        card_piles[index].add_card(c);

        // this is where we would handle comments but we aren't storing them
    }

    // The efficient way to do this would be to sort
    // the indices and remove them in decreasing order
    // But instead I'll sweep and filter
    // I'll combine this with making PilePtrs

    let mut result_piles: Vec<PilePtr> = vec![];
    for (index, p) in card_piles.into_iter().enumerate() {
        if !remove_piles.contains(&index) {
            result_piles.push(PilePtr::new(p));
        }
    }
    // only check for unknown card names if no error
    if error.is_empty() {
        for (k, v) in exclude_names {
            if !*v {
                error = format!("Unknown card {}", k);
                break;
            }
        }
    }
    if !error.is_empty() {
        return Err(error);
    }
    Ok(result_piles)
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
    if let Some(v) = m.get(&"--exclude".to_string()) {
        for p in v {
            exclude_names.insert(p.to_string(), false);
        }
    };
    // groups we need
    let mut required_groups: HashMap<String, bool> = HashMap::new();
    let card_filename = match m.get(&"--cardfile".to_string()) {
        Some(v) => {
            if v.is_empty() || v[0].is_empty() {
                return Err("No card file specified".to_string());
            }
            &v[0]
        }
        None => &card_file,
    };
    let temp_piles = load_cards(card_filename, &mut exclude_names)?;

    if let Some(box_names) = m.get(&"--boxes".to_string()) {
        let box_filename = match m.get(&"--boxfile".to_string()) {
            Some(f) => {
                if f.is_empty() {
                    "".to_string()
                } else {
                    f[0].clone()
                }
            }
            None => box_file,
        };
        if box_filename.is_empty() {
            return Err("No box file specified.".to_string());
        }
        let box_to_set = read_boxes(&box_filename)?;
        if box_to_set.is_empty() {
            return Err("--boxes specified but no boxes known (use --boxfile).".to_string());
        };
        // now we start processing the --boxes param
        for bp in box_names {
            match box_to_set.get(bp) {
                None => return Err(format!("Box {} not known in box file {}", bp, box_filename)),
                Some(e) => {
                    for name in e {
                        required_groups.insert(name.to_string(), false);
                    }
                }
            };
        }
    };

    let mut p_set = PileSet::new();

    if let Some(e) = m.get(&"--groups".to_string()) {
        for v in e {
            required_groups.insert(v.to_string(), false);
        }
    };
    if !required_groups.is_empty() {
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
                if !err.is_empty() {
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
    if let Some(e) = m.get(&"--include".to_string()) {
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
    };

    let mut min_types: HashMap<String, u8> = HashMap::new();
    if let Some(v) = m.get("--min-type") {
        for s in v {
            match split_once(s, ':') {
                None => continue,
                Some((lhs, rhs)) => {
                    if lhs.is_empty() {
                        continue;
                    }
                    let typecount = ushort_value(rhs);
                    min_types.insert(lhs.to_string(), typecount);
                }
            }
        }
    };
    let mut max_types: HashMap<String, u8> = HashMap::new();
    if let Some(v) = m.get("--max-type") {
        for s in v {
            match split_once(s, ':') {
                None => continue,
                Some((lhs, rhs)) => {
                    if lhs.is_empty() {
                        continue;
                    }
                    let typecount = ushort_value(rhs);
                    max_types.insert(lhs.to_string(), typecount);
                }
            }
        }
    };
    let use_bad_rand = m.get(&"--badrand".to_string()).is_some();
    let mut seed: u64 = 0;
    let mut chose_seed = false;
    if let Some(v) = m.get(&"--seed".to_string()) {
        if !v.is_empty() {
            seed = unsigned_value(&v[0]);
            chose_seed = true;
        };
    };
    let mut max_cost_repeat = 0;
    if let Some(v) = m.get(&"--max-cost-repeat".to_string()) {
        if !v.is_empty() {
            max_cost_repeat = ushort_value(&v[0]);
        }
    };
    let mut validate = true;
    if let Some(v) = m.get(&"--no-validate".to_string()) {
        if !v.is_empty() {
            validate = bool_value(&v[0]);
        }
    };
    let list_collection = m.get(&"--list".to_string()).is_some();
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
            if !v.is_empty() {
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
            let x = (rand.get() % 7) as u8;
            if x < 3 {
                x
            } else {
                0
            }
        }
        Some(v) => {
            if !v.is_empty() {
                ushort_value(&v[0])
            } else {
                0
            }
        }
    };
    let why = m.get(&"--why".to_string()).is_some();
    let more_info = m.get(&"--info".to_string()).is_some();
    let disable_anti_cursors = m.get(&"--no-anti-cursor".to_string()).is_some();
    let disable_attack_react = m.get(&"--no-attack-react".to_string()).is_some();

    Ok(Config {
        args: m,
        rand: Box::new(rand),
        why,
        more_info,
        optional_extras: opt_extra,
        validate,
        list_collection,
        disable_anti_cursors,
        disable_attack_react,
        max_cost_repeat,
        min_types,
        max_types,
        piles: p_set,
        includes: include_piles,
    })
}

fn main() {
    let mut args: Vec<String> = env::args().collect();
    args.remove(0);
    let mut conf = match load_config(args, "cards.dat".to_string(), "".to_string()) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
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
        if !warnings.is_empty() {
            println!("Error validating collection:");
            for s in warnings {
                println!("{}", s);
            }
            exit(3);
        };
    };
    if conf.list_collection {
        for p in &conf.piles {
            println!("{}", p.get_name());
        }
        exit(0);
    };
    col.shuffle(&mut conf.rand);
    let col = CardCollectionPtr::from_state(col);
    let constraints = match conf.build_constraints(&col) {
        Ok(v) => v,
        Err(s) => {
            println!("{}", s);
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
            eprintln!("Error: empty selection");
            eprintln!("Possible explanation: {}", m);
            exit(2);
        }
    };
    println!("Options:{}", conf.get_string());
    sel.dump(conf.why, conf.more_info);
}
