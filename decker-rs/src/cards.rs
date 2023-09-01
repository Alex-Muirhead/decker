use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::rc::Rc;

use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde::Deserialize;

use crate::costs::{decode_cost, Cost, CostTarget};
use crate::piles::{Pile, PilePtr};
use crate::{bool_value, no_empty_split, string_split};

// What to do about vectors?
#[derive(Debug)]
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
    pub cost_targets: Vec<Box<dyn CostTarget>>,
}

pub type CardPtr = Rc<Card>;
pub type Cards = Vec<CardPtr>;

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
        targets: Vec<Box<dyn CostTarget>>,
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

// #[derive(Debug, Deserialize)]
// pub struct CSVCard {
//     name: String,
//     pile: String,
//     card_group: String,
//     supply: bool,
//     kingdom: bool,
//     types: Vec<String>,
//     cost: Cost,
//     keywords: Vec<String>,
//     kw_interactions: Vec<String>,
//     other_interactions: Vec<String>,
//     cost_targets: Targets,
// }

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
    let coin_cost = fields[COINCOST].parse::<i8>().ok();
    let potion_cost = fields[POTIONCOST].parse::<i8>().ok();
    let debt_cost = fields[DEBTCOST].parse::<i8>().ok();
    let c = Cost::new(coin_cost, potion_cost, debt_cost);

    let in_supply = bool_value(&fields[SUPPLYCOL]);
    let is_kingdom = bool_value(&fields[KINGDOMCOL]);

    let types = no_empty_split(&fields[TYPECOL], ';');
    let keywords = no_empty_split(&fields[KEYWORDSCOL], ';');
    let interacts_kw = no_empty_split(&fields[INTERACTKEY], ';');
    let interacts_other = no_empty_split(&fields[INTERACTOTHER], ';');
    let mut targets: Vec<Box<dyn CostTarget>> = vec![];

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

pub fn load_cards(
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

    let mut linecount: u16 = 1;
    let mut error: String = "".to_string();

    for item in input.lines().skip(1) {
        let line: String = match item {
            Err(_) => continue,
            Ok(l) => l,
        };
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
    if error.is_empty()
    // only check for unknown card names if no error
    {
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
