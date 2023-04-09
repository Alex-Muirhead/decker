use std::collections::BTreeMap;
use std::rc::Rc;

use crate::collections::{CardCollectionPtr, CollectionIterator};
use crate::properties::CardGroupProperty;
use crate::selections::SelectionPtr;

trait ConstraintAction {
    // send back a selection or an error message
    fn apply(&self, label: &str, start: &SelectionPtr) -> Result<SelectionPtr, String>;
}

pub struct ConstraintActionPtr(Rc<dyn ConstraintAction>);

impl ConstraintActionPtr {
    pub fn apply(&self, label: &str, start: &SelectionPtr) -> Result<SelectionPtr, String> {
        self.0.apply(label, start)
    }
}

pub struct FindBane {
    begin: CollectionIterator,
    col: CardCollectionPtr,
}

impl FindBane {
    pub fn make_ptr(col: &CardCollectionPtr, begin_it: &CollectionIterator) -> ConstraintActionPtr {
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
