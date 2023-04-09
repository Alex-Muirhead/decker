use std::rc::Rc;

use crate::actions::{AddGroup, ConstraintActionPtr, FindBane, FindPile};
use crate::collections::CardCollectionPtr;
use crate::costs::{Cost, CostSet};
use crate::properties::PropertyPtr;
use crate::selections::SelectionPtr;
use crate::MANY;

use crate::properties::{
    CardGroupProperty, CostAndTypeProperty, FailProperty, KeywordProperty, NameProperty,
    NoteProperty, OtherInteractionProperty, TypeProperty,
};

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
    pub(crate) fn act(&self, start: &SelectionPtr) -> Result<SelectionPtr, String> {
        match &self.action {
            Some(act) => act.apply(&self.why, start),
            None => Err("".to_string()),
        }
    }

    pub(crate) fn get_status(&self, sel: &SelectionPtr) -> ConsResult {
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

    pub(crate) fn make_ptr(
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

    pub(crate) fn make_ptr_full(
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
}

pub type ConstraintPtr = Rc<Constraint>;
