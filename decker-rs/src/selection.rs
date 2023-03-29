use std::borrow::Borrow;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::card::*;
use crate::collection::*;
use crate::constraint::*;
use crate::cost::*;
use crate::pile::*;

pub struct SelectionState {
    piles: Piles,
    cards: Cards,
    pub constraints: Rc<RefCell<Vec<ConstraintPtr>>>,
    tags: RefCell<BTreeMap<PilePtr, Vec<String>>>,
    required_cards: u8,
    current_normal_pile_count: u8,
    notes: BTreeSet<String>,
    need_items: RefCell<BTreeSet<String>>, // <= required_cards
    costs_in_supply: CostSet,
    // This one needs to be modified after wrapping
    target_check_required: RefCell<bool>,
    target_blame: RefCell<String>, // piles responsible for cost target
    targets: Targets,
    interacts_keywords: BTreeMap<String, u64>,
    keywords: BTreeMap<String, u64>,
    card_coll: CardCollectionPtr,
    begin_general: RefCell<CollectionIterator>,
}

impl SelectionState {
    pub fn get_collection(&self) -> &CardCollectionPtr {
        &self.card_coll
    }

    pub fn get_piles(&self) -> &Piles {
        &self.piles
    }

    pub fn add_constraint(&mut self, cp: ConstraintPtr) {
        let c_vec: &RefCell<Vec<ConstraintPtr>> = self.constraints.borrow();
        c_vec.borrow_mut().push(cp);
    }

    // only use so far is to make space for "bane" card
    pub fn increase_required_piles(&mut self) {
        self.required_cards += 1
    }

    pub fn add_pile(&mut self, p: &PilePtr) -> bool {
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
                self.costs_in_supply.insert(*c.get_cost());
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
        true
    }

    pub fn tag_pile(&self, p: &PilePtr, tag: &String) {
        let r = &mut self.tags.borrow_mut();
        let vs = r.entry(p.clone()).or_default();
        vs.push(tag.to_string());
    }

    pub fn add_note(&mut self, s: &String) {
        self.notes.insert(s.to_string());
    }

    pub fn add_item(&self, s: &String) {
        self.need_items.borrow_mut().insert(s.to_string());
    }

    pub fn set_need_to_check(&self, v: bool, s: &String) {
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

    pub fn get_target_string(&self) -> String {
        self.target_blame.borrow().clone()
    }

    fn new2(
        col: &CardCollectionPtr,
        general_begin: CollectionIterator,
        market_cap: u8,
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

    pub fn new1(
        coll: &CardCollectionPtr,
        general_begin: CollectionIterator,
        market_cap: u8,
    ) -> SelectionState {
        SelectionState::new2(coll, general_begin, market_cap)
    }

    pub fn new(coll: &CardCollectionPtr, general_begin: CollectionIterator) -> SelectionState {
        SelectionState::new1(coll, general_begin, 10)
    }

    pub fn contains(&self, p: &PilePtr) -> bool {
        for t in &self.piles {
            if t == p {
                return true;
            }
        }
        false
    }
}

pub struct SelectionPtr {
    pub state: Rc<SelectionState>,
}

impl SelectionPtr {
    pub fn from_state(s: SelectionState) -> SelectionPtr {
        SelectionPtr { state: Rc::new(s) }
    }

    // Makes a copy of the state to modify before
    // wrapping it in a SelectionPtr later
    pub fn duplicate_state(&self) -> SelectionState {
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

    pub fn dump(&self, show_all: bool, show_card_info: bool) {
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
                println!("From {}", group_name);
            }
            print!("   {}", p.get_name());
            if let Some(e) = self.state.tags.borrow().get(p) {
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
            println!();
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
        if !items.is_empty() {
            println!("Need the following items:");
            for s in items {
                println!("   {}", s);
            }
        };
    }

    pub fn get_normal_pile_count(&self) -> u8 {
        self.state.current_normal_pile_count
    }

    pub fn get_required_count(&self) -> u8 {
        self.state.required_cards
    }

    pub fn contains(&self, p: &PilePtr) -> bool {
        self.state.contains(p)
    }

    pub fn get_piles(&self) -> &Piles {
        &self.state.piles
    }

    pub fn get_cards(&self) -> &Cards {
        &self.state.cards
    }

    pub fn has_note(&self, s: &String) -> bool {
        self.state.notes.contains(s)
    }

    pub fn get_general_pile(&self) -> Option<PilePtr> {
        self.state.begin_general.borrow_mut().next()
    }

    pub fn get_cost_set(&self) -> &CostSet {
        &self.state.costs_in_supply
    }

    pub fn need_to_check_costtargets(&self) -> bool {
        *self.state.target_check_required.borrow()
    }

    pub fn set_need_to_check(&self, v: bool, s: &String) {
        self.state.set_need_to_check(v, s);
    }

    pub fn get_target_set(&self) -> &Targets {
        &self.state.targets
    }

    pub fn get_collection(&self) -> &CardCollectionPtr {
        &self.state.card_coll
    }

    pub fn get_interacts_keywords(&self) -> &BTreeMap<String, u64> {
        &self.state.interacts_keywords
    }

    pub fn get_keywords(&self) -> &BTreeMap<String, u64> {
        &self.state.keywords
    }
}
