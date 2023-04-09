use std::borrow::Borrow;
use std::cmp::Ordering;
use std::cmp::Ordering::*;
use std::collections::{BTreeSet, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::cards::{Card, Cards};
use crate::costs::{CostSet, TargetPtr, Targets};

pub struct Pile {
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

pub type PilePtr = Rc<Pile>;

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
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for Pile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Pile {
    pub fn new(name: &str) -> Pile {
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
    pub fn add_card(&mut self, c: Card) {
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

    pub fn get_card_group(&self) -> &str {
        &self.card_group
    }
    pub fn get_supply(&self) -> bool {
        self.supply
    }
    pub fn get_kingdom(&self) -> bool {
        self.kingdom
    }
    // returning const references is not alien to c++
    pub fn get_types(&self) -> &HashSet<String> {
        &self.types
    }
    pub fn get_costs(&self) -> &CostSet {
        &self.costs
    }
    pub fn get_keywords(&self) -> &HashSet<String> {
        &self.keywords
    }
    pub fn get_kw_interactions(&self) -> &HashSet<String> {
        &self.kw_interactions
    }
    // In list of things I use, note a lot of string sets
    pub fn get_other_interactions(&self) -> &HashSet<String> {
        &self.other_interactions
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_cards(&self) -> &Cards {
        &self.cards
    }
    pub(crate) fn get_targets(&self) -> &Targets {
        &self.targets
    }
}

pub type PileSet = BTreeSet<PilePtr>;
pub type Piles = Vec<PilePtr>;

#[derive(Ord, Eq)]
pub struct SortablePile {
    pub(crate) p: PilePtr,
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
        Some(Equal)
    }
}
