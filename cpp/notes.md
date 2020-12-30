
# Type overview:

## Card
Holds the info for a single card (all dumped via the constructor call).

### Major fields:
* name --- Name of this card
* cardGroup --- Each card belongs to exactly one group. 
Eg: 
..* Copper and Estates belong to the "base" group. 
..* Adventurer belongs to "Dominion-v1".
..* Artisan belongs to "Dominion-v2".
* supply --- is this card in the supply? Eg: The prizes from Cornucopia's Tournament are not in the supply.
* kingdom --- is this card a "kingdom card"? Eg: base cards and Event cards are not kingdom cards.
* types --- types listed on the card. Eg: Action, Treasure, Victory ...
* cost --- cost of card; does not record any modifications (indicated with * on physical cards).
Note: at time of writing, costs in dominion have 3 possible components (coin, potion and debt).
Not all components are necessarily present and an absent component is different from a 0 value for that component.
* keywords --- common items from the card's text box (so not types). Eg: drawing additional cards => +card.
..* plurals are all described as singular.   + action, +2 actions ... are all described as +action
..* some keywords do not appear explictly in their text boxes but need some interpretation
...* topdeck --- the card allows the player to influence what the top card of their deck is. Eg: Bureaucrat
...* trash_any --- player can trash a card 
...* trash_limited --- player can trash a card but there are some limits on which card

* keywordInteractions --- list of keywords this card interacts with. Eg: Catacombs (from Dark Ages) does something when a card is trashed.
Note: Reaction cards record their reactions as otherInteractions
* otherInteractions --- non-keyword interactions.
Eg: 
..* Moat has "react(Attack)"
..* Extra game items like item(tavernmat)
..* Other cards and groups --- Page (from Adventures) needs the "group(Adventures-Traveller-page)"
* costTargets --- Describes cost relationships with other cards. Eg: Remodel refers to cards costing 5 or less.
In the data file these are listed in the other interactions column but here they have objects to represent 
them (instead of just strings) and so are stored separately.

## Pile
Every card belongs to exactly one pile.
If the name of the pile is not given in some other way, the pile should have the same name as the card.
Has many of the same properties as cards but as sets. Eg: instead of a single cost there will be a cost set.

* The kingdom property is true if any card in the pile has that property as true. --- It is done this way to allow Artefacts like Renaissance's Flag to be gathered with the card which uses it ("Flag Bearer").
* The supply property is derived in the same way.

## Config
Class to do the following:

* Reads and validates commandline arguments.
* Extracts cards from data file.
* Determine the cards to select from.
* Determine constraints which will apply to the selection.

The results are passed off to the CardCollection instance to generate the selection.


## CardCollection
After it has been set up (and shuffled) this object should be immutable.
Card selections are generated from piles stored in the Collection.
The main calls here are:

* shuffle()
* generateSelection()

The others are helper routines that are exposed in case someone wants more control.

## Selection
There is a fairly tight integration between this class and CardCollection.
The flow is roughly:

1. Get a selection from CardCollection::generateSelection()
2. Call dump() on that selection.

## Property
A predicate which checks either individual cards or a whole Selection.


## Constraint
Checks if changes need to be made to the selection based on the cards already in it.
Constraints evaluate to one of the following:

* OK --- the constraint has no suggested or required modifications. (Ignore it for now).
* ActionRequired --- Action must be taken in order to satisfy the constraint. 
Selection will be invalid until this is done.
This takes priority even over checking other constraints.
* MorePossible --- The selection is currently valid (as far as this constraint is concerned), 
but if there is no other action required, further action on this constraint would be ok.
* Fail --- The current selection can never (not even by adding more cards) satisfy this constraint.
Backtracking required.

Properties are generally used in one of two ways:

1. To count how many piles satisfy a test. Eg: Need to have at most 3 attack cards OR need one or more duration cards.
In this situation the number of satisfying cards will be monotonically increasing.
2. Check if there are unmatched things. Eg: Are there any piles which depend on a group but that group hasn't been added yet.
This will not be monotonic.

Constraints can be conditionally activated by having a precondition property with the main property 
only being checked when the precondition matches.

## ConstraintAction
Describes what action to take if a constraint is not satisfied.
Eg: add a card group if there is a card depending on it in the selection.