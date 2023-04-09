use std::collections::{BTreeSet, HashMap, HashSet};

use crate::actions::{
    AddGroup, AddMissingDependency, AddMissingDependencyGroup, AddProsperity, FindPile,
};
use crate::bad_rand::{get_rand_stream, RandStream};
use crate::cards::load_cards;
use crate::collections::CardCollectionPtr;
use crate::constraints::{
    attack_react_constraint, bane_constraint, curser_constraint, prosp_constraint, Constraint,
    ConstraintPtr,
};
use crate::piles::PileSet;
use crate::properties::prelude::*;

use crate::{
    bool_value, caps, capu, capus, group_name_prefix, organise_args, read_boxes, show_options,
    split_once, unsigned_value, ushort_value, StringMultiMap, MANY,
};

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

pub struct Config {
    pub(crate) args: StringMultiMap,
    pub(crate) rand: Box<dyn RandStream>,
    pub(crate) why: bool,
    pub(crate) more_info: bool,
    pub(crate) optional_extras: u8,
    pub(crate) validate: bool,
    pub(crate) list_collection: bool,
    pub(crate) disable_anti_cursors: bool,
    pub(crate) disable_attack_react: bool,
    pub(crate) max_cost_repeat: u8,
    pub(crate) min_types: HashMap<String, u8>,
    pub(crate) max_types: HashMap<String, u8>,
    pub(crate) piles: PileSet,
    pub(crate) includes: PileSet,
}

impl Config {
    pub fn get_string(&self) -> String {
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

    pub fn build_constraints(
        &mut self,
        col: &CardCollectionPtr,
    ) -> Result<Vec<ConstraintPtr>, String> {
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

// less need to do reference params to get around multiple ret / error ret
pub fn load_config(
    args: Vec<String>,
    card_file: String,
    box_file: String,
) -> Result<Config, String> {
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
    let temp_piles = match load_cards(card_filename, &mut exclude_names) {
        Err(s) => {
            return Err(s);
        }
        Ok(v) => v,
    };

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
        let box_to_set = match read_boxes(&box_filename) {
            Err(s) => return Err(s),
            Ok(v) => v,
        };
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
