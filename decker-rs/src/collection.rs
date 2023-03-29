use std::borrow::Borrow;
use std::collections::{BTreeSet, HashMap, HashSet};

use std::cell::RefCell;
use std::rc::Rc;

use crate::{ConsResult::*, ConstraintPtr, PropertyPtr, RandStream};

use crate::card::*;
use crate::constraint::*;
use crate::cost::*;
use crate::pile::*;
use crate::property::*;
use crate::selection::*;

#[derive(Clone)]
pub struct CollectionIterator {
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

pub struct CollectionState {
    legal_costs: CostSet,
    cards: HashSet<CardPtr>,
    general_property: PropertyPtr,
    group_names: BTreeSet<String>,
    card_names: BTreeSet<String>,
    piles: Vec<PilePtr>,
    lists: RefCell<PropLists>,
}

impl CollectionState {
    pub fn validate_collection(&self) -> CollectionStatus {
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
        if warnings.is_empty() {
            CollectionStatus::CollOK
        } else {
            CollectionStatus::CollWarning(warnings)
        }
    }

    pub fn shuffle(&mut self, r: &mut Box<dyn RandStream>) {
        let us = self.piles.len();
        let size: u64 = us.try_into().unwrap_or(10);
        // go through the pile vector 3 times and swap items
        for _i in 0..3 {
            for j in 0..us {
                let pos: usize = (r.get() % size).try_into().unwrap_or(10);
                self.piles.swap(pos, j);
            }
        }
    }
}

// Rename this once I've got it done
#[derive(Clone)]
pub struct CardColl {
    state: Rc<CollectionState>,
}

impl CardColl {
    // Wrapper for starting, building and finishing a selection
    pub fn generate_selection(
        &self,
        market_cap: u8,
        landscapes: u8,
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
        Ok(res)
    }

    fn start_selection(&self, market_cap: u8, landscapes: u8) -> Option<SelectionState> {
        let base = CardGroupProperty::make_ptr("base");
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
                for (count, i) in begin.enumerate() {
                    if count as u8 >= landscapes {
                        break;
                    };
                    // should not be able to fail adding
                    if !new_sel.add_pile(&i) {
                        return None; // different from c++
                    };
                }
            };
        };
        Some(new_sel)
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
                let ps = CardGroupProperty::make_ptr("DarkAges-base");

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
                if c.debt.is_some() {
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
                sel.tag_pile(p, &"Replaces one Copper in starting deck".to_string());
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

    pub fn new_state(piles: &PileSet) -> CollectionState {
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
            legal_costs,
            cards,
            general_property: KingdomAndSupplyProperty::make_ptr(),
            group_names,
            card_names,
            piles: piles_vector,
            lists: RefCell::new(PropLists {
                lists: vec![],
                map: HashMap::new(),
            }),
        }
    }

    pub fn from_state(c: CollectionState) -> CardColl {
        CardColl { state: Rc::new(c) }
    }

    pub fn get_iterators(&self, p: &PropertyPtr) -> Option<CollectionIterator> {
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
                    if p.pile_meets(pil) {
                        newv.push(pil.clone());
                    }
                }
                if newv.is_empty() {
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

    pub fn get_piles(&self) -> &Vec<PilePtr> {
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
    pub fn build_selection(&self, start: &SelectionPtr) -> Result<SelectionPtr, String> {
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
        for s in status.iter().take(size) {
            if s == &ConsMorePossible {
                // should take action on this constraint
                break;
            }
        }
        // do we need to consider cost targets?
        if start.need_to_check_costtargets() {
            static HAVE_COST_PENALTY: f32 = -3.0;
            static THRESHOLD: f32 = 0.5;
            static TOLERANCE: f32 = 0.21; // 0.2 was resulting in non-determinism

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
                    Some(begin) => {
                        // The original version won't return empty iterators
                        // so do while and while will be equivalent
                        for next in begin {
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
                            if let Ok(s) = self.build_selection(&SelectionPtr::from_state(new_sel))
                            {
                                return Ok(s);
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
        Err("".to_string())
    }

    pub fn get_pile_for_card(&self, s: &String) -> Option<PilePtr> {
        for c in &self.state.cards {
            if c.get_name() == s {
                let pn = if c.get_pile_name().is_empty() {
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
        None
    }
}

pub enum CollectionStatus {
    CollOK,
    CollWarning(Vec<String>),
    _CollFatal(Vec<String>),
}

pub type CardCollectionPtr = CardColl;
