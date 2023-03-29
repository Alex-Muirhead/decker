use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::cost::*;

// What to do about vectors?
pub struct Card {
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

pub type CardPtr = Rc<Card>;
pub type Cards = Vec<CardPtr>;

impl Card {
    // need to work out how many of these need to be copies and how many can be moved
    // Look at uses for Card constructor and work this out
    // A move constructor would have been fine for what I used it for
    pub fn new(
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
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_card_group(&self) -> &str {
        &self.card_group
    }
    pub fn get_pile_name(&self) -> &str {
        &self.pile
    }
    pub fn get_supply(&self) -> bool {
        self.supply
    }
    pub fn get_kingdom(&self) -> bool {
        self.kingdom
    }
    pub fn get_types(&self) -> &[String] {
        &self.types
    }
    pub fn get_cost(&self) -> &Cost {
        &self.cost
    }
    pub fn get_keywords(&self) -> &[String] {
        &self.keywords
    }
    pub fn get_kw_interactions(&self) -> &[String] {
        &self.kw_interactions
    }
    pub fn get_other_interactions(&self) -> &[String] {
        &self.other_interactions
    }
    pub fn get_cost_targets(&self) -> &Targets {
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
pub type CardSet = HashSet<CardPtr>;
