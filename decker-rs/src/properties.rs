use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::costs::{Cost, CostSet};
use crate::piles::PilePtr;
use crate::selections::SelectionPtr;
use std::collections::hash_map::Entry::Occupied;

// Re-export all these, so I don't have to use them individually
pub mod prelude {
    pub use super::{
        CardGroupProperty, CostAndTypeProperty, CostProperty, EitherProperty, FailProperty,
        HangingInteractsWith, KeywordInteractionProperty, KeywordProperty,
        KingdomAndSupplyProperty, MissingGroupForKeywordProperty,
        MissingInteractingCardGroupProperty, MissingInteractingCardProperty, MissingPotionProperty,
        NameProperty, NeedProsperity, NoteProperty, OptionalExtraProperty,
        OtherInteractionProperty, RepeatedCostProperty, TypeProperty,
    };
}

trait Property {
    fn is_selection_property(&self) -> bool;

    // no method overloading :-(

    fn pile_meets(&self, p: &PilePtr) -> bool;
    fn selection_meets(&self, s: &SelectionPtr) -> bool;
}

#[derive(Clone)]
pub struct PropertyPtr {
    state: Rc<dyn Property>,
}

impl PartialEq for PropertyPtr {
    fn eq(&self, other: &Self) -> bool {
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
    pub fn is_selection_property(&self) -> bool {
        self.state.is_selection_property()
    }

    pub(crate) fn pile_meets(&self, p: &PilePtr) -> bool {
        self.state.pile_meets(p)
    }

    pub(crate) fn selection_meets(&self, s: &SelectionPtr) -> bool {
        self.state.selection_meets(s)
    }
}

pub struct KingdomAndSupplyProperty {}

impl KingdomAndSupplyProperty {
    pub(crate) fn make_ptr() -> PropertyPtr {
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

pub struct TypeProperty {
    type_name: String,
    kingdom_and_supply: bool,
}

impl TypeProperty {
    pub(crate) fn make_ptr(has_type: &str, restrict_to_kingdom_and_supply: bool) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(TypeProperty {
                type_name: has_type.to_owned(),
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

pub struct NameProperty {
    name: String,
}

impl NameProperty {
    pub(crate) fn make_ptr(name: &String) -> PropertyPtr {
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

pub struct CostAndTypeProperty {
    cost_prop: PropertyPtr,
    type_prop: TypeProperty,
}

impl CostAndTypeProperty {
    pub(crate) fn make_ptr_set(type_name: String, cost: CostSet) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(CostAndTypeProperty {
                cost_prop: CostProperty::make_ptr_set(cost, true),
                type_prop: TypeProperty {
                    type_name,
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

pub struct NoteProperty {
    text: String,
}

impl NoteProperty {
    pub(crate) fn make_ptr(text: &String) -> PropertyPtr {
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

pub struct EitherProperty {
    prop1: PropertyPtr,
    prop2: PropertyPtr,
}

impl EitherProperty {
    pub(crate) fn make_ptr(prop1: &PropertyPtr, prop2: &PropertyPtr) -> PropertyPtr {
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

pub struct CardGroupProperty {
    group_name: String,
}

impl CardGroupProperty {
    pub(crate) fn make_ptr(group_name: &str) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(CardGroupProperty {
                group_name: group_name.to_owned(),
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

pub struct OptionalExtraProperty {}

impl OptionalExtraProperty {
    pub(crate) fn make_ptr() -> PropertyPtr {
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

pub struct OtherInteractionProperty {
    other_interact: String,
    kingdom_and_supply: bool,
}

impl OtherInteractionProperty {
    pub(crate) fn make_ptr(other_interact: &str, kingdom_and_supply: bool) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(OtherInteractionProperty {
                other_interact: other_interact.to_owned(),
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

pub struct MissingPotionProperty {}

impl MissingPotionProperty {
    pub(crate) fn make_ptr() -> PropertyPtr {
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
        found && !have_potion
    }
}

pub struct MissingGroupForKeywordProperty {
    type_needed: String,
    note: String,
}

impl MissingGroupForKeywordProperty {
    pub(crate) fn make_ptr(type_needed: &String, group_needed: &String) -> PropertyPtr {
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
                if it.starts_with(&self.type_needed) && !s.has_note(&self.note) {
                    return true;
                }
            }
        }
        false
    }
}

pub struct MissingInteractingCardGroupProperty {}

impl MissingInteractingCardGroupProperty {
    pub(crate) fn make_ptr() -> PropertyPtr {
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
        false
    }
}

pub struct MissingInteractingCardProperty {}

impl MissingInteractingCardProperty {
    pub(crate) fn make_ptr() -> PropertyPtr {
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
        if need.is_empty() {
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
        false
    }
}

pub struct FailProperty {}

impl FailProperty {
    pub(crate) fn make_ptr() -> PropertyPtr {
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

pub struct RepeatedCostProperty {
    max_repeats: u64,
}

impl RepeatedCostProperty {
    pub(crate) fn make_ptr(max_repeats: u64) -> PropertyPtr {
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
        let mut counts: HashMap<Cost, u64> = HashMap::new();
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
        false
    }
}

pub struct CostProperty {
    single_cost: Option<Cost>,
    costs: CostSet,
    supply_only: bool,
}

impl CostProperty {
    pub(crate) fn make_ptr_set(costs: CostSet, supply_only: bool) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(CostProperty {
                single_cost: None,
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
        if let Some(cost) = self.single_cost {
            return p.get_costs().contains(&cost);
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

pub struct HangingInteractsWith {
    interacts_with: String,
    kw: String,
    alt_kw: String,
}

impl HangingInteractsWith {
    pub(crate) fn make_ptr2(interacts_with: &String, kw: &String) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(HangingInteractsWith {
                interacts_with: interacts_with.to_string(),
                kw: kw.to_string(),
                alt_kw: "".to_string(),
            }),
        }
    }

    pub(crate) fn make_ptr3(interacts_with: &String, kw: &String, alt_kw: &String) -> PropertyPtr {
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
        true
    }
}

pub struct KeywordProperty {
    keyword: String,
    kingdom_and_supply: bool,
}

impl KeywordProperty {
    pub(crate) fn make_ptr(keyword: &str, restrict_to_kingdom_and_supply: bool) -> PropertyPtr {
        PropertyPtr {
            state: Rc::new(KeywordProperty {
                keyword: keyword.to_owned(),
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

pub struct KeywordInteractionProperty {
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

pub struct NeedProsperity {
    threshold: u8,
}

impl NeedProsperity {
    pub(crate) fn make_ptr(threshold: u8) -> PropertyPtr {
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
        (self.threshold > 0) && (self.threshold <= total)
    }
}

// so we only need to do one lock
pub struct PropLists {
    pub(crate) lists: Vec<Vec<PilePtr>>,
    pub(crate) map: HashMap<PropertyPtr, usize>,
}
