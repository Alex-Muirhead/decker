use std::collections::BTreeMap;
use std::rc::Rc;

use crate::MANY;
use crate::cost::*;
use crate::collection::*;
use crate::property::*;
use crate::selection::SelectionPtr;

pub fn bane_constraint(col: &CardCollectionPtr) -> ConstraintPtr {
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
    let fix = FindBane::make_ptr(col, &begin);
    let has_bane = NoteProperty::make_ptr(&"hasBane".to_string());
    // if we have less than 1 YoungWitch do nothing
    // if we have less than 1 hasBane note actionRequired   (only ever have 1 note)
    //.can accept more is empty (1,1)
    // from 1 .. MANY hasBane go inactive
    // more than MANY -> fail
    Constraint::make_ptr_full(
        "bane".to_string(),
        Some(has_yw),
        &has_bane,
        Some(fix),
        1,
        1,
        1,
        MANY,
    )
}

pub fn prosp_constraint(col: &CardCollectionPtr) -> ConstraintPtr {
    let group_pros = CardGroupProperty::make_ptr("Prosperity");
    let has_pros_base = NoteProperty::make_ptr(&"addedProsperity-base".to_string());
    let fix = AddGroup::make_ptr(col, &"Prosperity-base".to_string());
    // if we have less than 5 Prosperity cards do nothing
    // if we have less than 1 note, action required
    Constraint::make_ptr_full(
        "prospBasics".to_string(),
        Some(group_pros),
        &has_pros_base,
        Some(fix),
        5,
        1,
        1,
        MANY,
    )
}

pub fn curser_constraint(col: &CardCollectionPtr, threshold: u64) -> Option<ConstraintPtr> {
    let curser = KeywordProperty::make_ptr("curser", false);
    let trash = KeywordProperty::make_ptr("trash_any", true);
    let begin = match col.get_iterators(&trash) {
        Some(v) => v,
        None => return None,
    };
    let fix = FindPile::make_ptr(col, &begin);
    Some(Constraint::make_ptr_full(
        "counterCurser".to_string(),
        Some(curser),
        &trash,
        Some(fix),
        threshold,
        1,
        1,
        MANY,
    ))
}

pub fn attack_react_constraint(col: &CardCollectionPtr, threshold: u64) -> Option<ConstraintPtr> {
    let attack = TypeProperty::make_ptr("Attack", true);
    // only want kingdom and supply piles
    let react = OtherInteractionProperty::make_ptr("react(Attack)", true);
    let begin = match col.get_iterators(&react) {
        Some(v) => v,
        None => return None,
    };
    let fix = FindPile::make_ptr(col, &begin);
    Some(Constraint::make_ptr_full(
        "counterAttack".to_string(),
        Some(attack),
        &react,
        Some(fix),
        threshold,
        1,
        1,
        MANY,
    ))
}


trait ConstraintAction {
    // send back a selection or an error message
    fn apply(&self, label: &str, start: &SelectionPtr) -> Result<SelectionPtr, String>;
}

pub struct ConstraintActionPtr(Rc<dyn ConstraintAction>);

impl ConstraintActionPtr {
    fn apply(&self, label: &str, start: &SelectionPtr) -> Result<SelectionPtr, String> {
        self.0.apply(label, start)
    }
}

struct FindBane {
    begin: CollectionIterator,
    col: CardCollectionPtr,
}

impl FindBane {
    fn make_ptr(col: &CardCollectionPtr, begin_it: &CollectionIterator) -> ConstraintActionPtr {
        ConstraintActionPtr(Rc::new(FindBane {
            begin: begin_it.clone(),
            col: col.clone(),
        }))
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
                    if res.is_ok() {
                        return res;
                    }
                }
            }
        }
        Err("".to_string())
    }
}

pub struct AddGroup {
    group: String,
    coll: CardCollectionPtr,
}

impl AddGroup {
    pub fn make_ptr(coll: &CardCollectionPtr, group: &String) -> ConstraintActionPtr {
        ConstraintActionPtr(Rc::new(AddGroup {
            group: group.to_string(),
            coll: coll.clone(),
        }))
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
        self.coll
            .build_selection(&SelectionPtr::from_state(new_sel))
    }
}

pub struct FindPile {
    begin: CollectionIterator,
    col: CardCollectionPtr,
}

impl FindPile {
    pub fn make_ptr(col: &CardCollectionPtr, begin_it: &CollectionIterator) -> ConstraintActionPtr {
        ConstraintActionPtr(Rc::new(FindPile {
            begin: begin_it.clone(),
            col: col.clone(),
        }))
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
        Err("".to_string())
    }
}

pub struct AddMissingDependency {
    col: CardCollectionPtr,
}

impl AddMissingDependency {
    pub fn make_ptr(col: &CardCollectionPtr) -> ConstraintActionPtr {
        ConstraintActionPtr(Rc::new(AddMissingDependency { col: col.clone() }))
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
        if need.is_empty() {
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
        Err("AddMissingDependency applied but nothing seemed missing".to_string())
    }
}

pub struct AddMissingDependencyGroup {
    col: CardCollectionPtr,
}

impl AddMissingDependencyGroup {
    pub fn make_ptr(col: &CardCollectionPtr) -> ConstraintActionPtr {
        ConstraintActionPtr(Rc::new(AddMissingDependencyGroup { col: col.clone() }))
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
                        let ps = CardGroupProperty::make_ptr(need_name);
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
        self.col.build_selection(&SelectionPtr::from_state(new_sel))
    }
}

pub struct AddProsperity {
    col: CardCollectionPtr,
}

impl AddProsperity {
    pub fn make_ptr(col: &CardCollectionPtr) -> ConstraintActionPtr {
        ConstraintActionPtr(Rc::new(AddProsperity { col: col.clone() }))
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
        self.col.build_selection(&SelectionPtr::from_state(new_sel))
    }
}

#[derive(PartialEq, Eq)]
pub enum ConsResult {
    ConsOK,           // Constraint is neutral/inactive on the selection
    ConsActionReq,    // something needs to be done to satisfy constraint
    ConsMorePossible, // Constraint is satisfied but could accept additional cards
    ConsFail,         // constraint can not be satisfied from current selection
}

pub struct Constraint {
    property: PropertyPtr,
    precondition: Option<PropertyPtr>,
    action: Option<ConstraintActionPtr>,
    prop_active: u64,    // x
    prop_satisfied: u64, // a
    prop_inactive: u64,  // b
    prop_broken: u64,    // c
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

    pub fn make_ptr(
        label: String,
        prop: &PropertyPtr,
        act: Option<ConstraintActionPtr>,
        min: u64,
        max: u64,
    ) -> ConstraintPtr {
        Rc::new(Constraint {
            property: prop.clone(),
            precondition: None,
            action: act,
            prop_active: 0,
            prop_satisfied: min,
            prop_inactive: min,
            prop_broken: max + 1,
            why: label,
        })
    }

    pub fn make_ptr_full(
        label: String,
        pre: Option<PropertyPtr>,
        prop: &PropertyPtr,
        act: Option<ConstraintActionPtr>,
        x: u64,
        a: u64,
        b: u64,
        c: u64,
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

    pub fn get_status(&self, sel: &SelectionPtr) -> ConsResult {
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
        ConsResult::ConsActionReq
    }

    pub fn act(&self, start: &SelectionPtr) -> Result<SelectionPtr, String> {
        match &self.action {
            Some(act) => act.apply(&self.why, start),
            None => Err("".to_string()),
        }
    }
}

pub type ConstraintPtr = Rc<Constraint>;
