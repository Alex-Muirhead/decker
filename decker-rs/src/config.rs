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

use crate::{caps, capu, capus, group_name_prefix, read_boxes, Cli, MANY};

pub struct Config {
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
pub fn load_config(cli: Cli, card_file: String, box_file: String) -> Result<Config, String> {
    let mut err: String = "".to_string();

    let mut exclude_names =
        HashMap::from_iter(cli.exclude.iter().map(|name| (name.clone(), false)));

    let card_filename = &cli.cardfile.unwrap_or(card_file);

    let temp_piles = load_cards(card_filename, &mut exclude_names)?;
    let mut required_groups: HashMap<String, bool> = HashMap::new();

    if !cli.boxes.is_empty() {
        let box_filename = cli.boxfile.unwrap_or(box_file);
        if box_filename.is_empty() {
            return Err("No box file specified.".to_string());
        }

        let box_to_set = read_boxes(&box_filename)?;
        if box_to_set.is_empty() {
            return Err("--boxes specified but no boxes known (use --boxfile).".to_string());
        };

        // now we start processing the --boxes param
        for bp in cli.boxes.iter() {
            match box_to_set.get(bp) {
                None => return Err(format!("Box {} not known in box file {}", bp, box_filename)),
                Some(e) => {
                    for name in e {
                        required_groups.insert(name.to_string(), false);
                    }
                }
            }
        }
    }

    for group_name in cli.groups {
        required_groups.insert(group_name.to_string(), false);
    }

    let mut p_set = PileSet::new();
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
    for name in cli.include {
        let mut found = false;
        'overset: for pile in &p_set {
            for card in pile.get_cards() {
                if card.get_name() == name {
                    include_piles.insert(pile.clone());
                    found = true;
                    break 'overset;
                }
            }
        }
        if !found {
            return Err(format!("Can't find card {}", name));
        }
    }

    let mut min_types: HashMap<String, u8> = HashMap::new();
    for s in cli.min_type.iter() {
        // Checking for valid inputs of form "Type:Int"
        match s.split_once(':') {
            Some((lhs, rhs)) if !lhs.is_empty() => {
                let typecount = rhs.parse::<u8>().unwrap_or(0);
                min_types.insert(lhs.to_string(), typecount);
            }
            _ => continue,
        }
    }

    let mut max_types: HashMap<String, u8> = HashMap::new();
    for s in cli.max_type.iter() {
        match s.split_once(':') {
            Some((lhs, rhs)) if !lhs.is_empty() => {
                let typecount = rhs.parse::<u8>().unwrap_or(0);
                max_types.insert(lhs.to_string(), typecount);
            }
            _ => continue,
        }
    }

    let use_bad_rand = cli.badrand;
    // TODO: Let the seed be picked from the randomiser if not provided
    // TODO: Find a way to update the CLI for seed value instead of using args
    let seed = cli.seed.unwrap_or(0);
    let max_cost_repeat = cli.max_cost_repeat;
    let validate = !cli.no_validate;
    let list_collection = cli.list;

    // now we set up randomiser
    // The cap here is arbitrary (want it to be at least as big
    // as 3 x pile size for shuffling
    let mut rand = get_rand_stream(seed, capu(10 * p_set.len()), use_bad_rand);

    // Now we limit the groups we can draw from.
    // We'll do it here before anything gets into the
    // CardCollection object.
    // We need to take into account which prefixes any
    // --include cards force in (and also if that exceeds
    // the limit

    let mut suggested_max = cli.max_prefixes;
    if suggested_max > 0 {
        suggested_max += 1;
        let mut chosen_prefixes = HashSet::<String>::new();
        chosen_prefixes.insert("base".to_string());
        for ip in &include_piles {
            chosen_prefixes.insert(group_name_prefix(ip.get_card_group()));
        }
        if capus(chosen_prefixes.len()) > suggested_max {
            // need to -1 from both numbers because of hidden "base" group
            return Err(format!(
                "Requested at most {} big groups, but included cards are drawn from {}.",
                suggested_max - 1,
                chosen_prefixes.len() - 1
            ));
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
    }

    // Now let's work out how many optional extras we need
    let opt_extra = cli.landscape_count.unwrap_or_else(|| {
        let x = (rand.get() % 7) as u8;
        if x < 3 {
            x
        } else {
            0
        }
    });

    let why = cli.why;
    let more_info = cli.info;
    let disable_anti_cursors = cli.no_anti_cursor;
    let disable_attack_react = cli.no_attack_react;

    Ok(Config {
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
