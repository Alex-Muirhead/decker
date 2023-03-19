
#include "types.h"
namespace decker
{
/*
 * Note: Properties are evaluated either on a single pile 
 * (eg is this an Action) or over the selection as a whole 
 * (eg: do we have repeated costs).
 * The isSelectionProperty() member indicates which type of property it is 
 * (defaults to false).
 * Because piles have all the types and costs of the cards making up the pile.
 * An extreme example is the castles pile from Empires has a set of 
 * types {Action, Castle, Treasure, Victory} and 
 * all costs between 3 and 10 coins.
*/    

/* Does a pile contain a given type. 
 * By default excludes cards which are not kingdom cards or not in supply.
*/
class TypeProperty : public virtual Property
{
public:
    TypeProperty(const std::string& hasType, bool restrictToKingdomAndSupply=true);
    ~TypeProperty(){};    
    bool meets(const Pile* c) const override;
    bool operator==(const Property& other) const override; 
protected:
    size_t calcHash() const override;
    bool typesEqual(const TypeProperty& other) const;
private:
    const std::string type;
    bool kingdomAndSupply;
};
    
/* Does a pile contain a given keyword. Can restrict to only 
 * kingdom cards in the supply
*/    
class KeywordProperty : public Property
{
public:
    KeywordProperty(const std::string& hasKeyword, bool onlyKingdomAndSupply);
    bool meets(const Pile* c) const override;
    ~KeywordProperty(){};
    bool operator==(const Property& other) const override;
protected:
    size_t calcHash() const override;
private:
    const std::string keyword;
    bool kingdomAndSupply;
};

/* Does a pile contain an interaction with a keyword
*/
class KeywordInteractionProperty : public Property
{
public:
    KeywordInteractionProperty(const std::string& interactsKeyword);
    virtual ~KeywordInteractionProperty(){};
    virtual bool meets(const Pile* c) const override;    
    virtual bool operator==(const Property& other) const override =0; // to use functions as keys    
protected:
    size_t calcHash() const override;
private:
    const std::string keyword;
};

/* Does the pile contain the specified cost or any cost in the specified group.
 * It is configurable whether it only considers cards in the supply
 * Note: This is different to other properties which 
 *       restrict to kingdom AND supply!
 * It's coded this way because we want to consider cards such as the default
 *  treasures for possible cost relationships.
 * virtual inheritance due to its use by CostAndTypeProperty later
*/
class CostProperty : public virtual Property
{
public:
    CostProperty(Cost cost, bool supplyOnly=true);
    CostProperty(const CostSet& costs, bool supplyOnly=true);
    ~CostProperty(){}
    bool meets(const Pile* c) const override;
    
    bool operator==(const Property& other) const  override;
protected:    
    size_t calcHash() const override;    
    bool costsEqual(const CostProperty& other) const;
private:
    Cost singleCost;
    CostSet costs;
    bool supplyOnly;    
};

/* Does the pile have specied type and (one of the) specied cost(s)
 * Note: inherits the default supply only parameter from CostProperty
*/ 
class CostAndTypeProperty : public CostProperty, TypeProperty
{
public:    
    CostAndTypeProperty(const std::string& type, Cost cost);
    CostAndTypeProperty(const std::string& type, const CostSet& costs);
    ~CostAndTypeProperty(){}
    bool meets(const Pile* c) const override;      
    virtual bool operator==(const Property& other) const override;   
protected:    
    size_t calcHash() const override;    
};

// For normal purchasable cards
class KingdomAndSupplyProperty : public Property
{
public:
    KingdomAndSupplyProperty(){};
    ~KingdomAndSupplyProperty(){};
    bool meets(const Pile* c) const override;
    bool operator==(const Property& other) const override;
protected:
    size_t calcHash() const override;       
};

/* Is the pile solely Way, Event, Landmark, Project 
 * (and not kingdom or supply)
 * Note: This does not apply to all "Landscape" cards since 
 * Artefacts are grouped with their supply piles
*/
class OptionalExtraProperty : public Property
{
public:
    OptionalExtraProperty(){};
    ~OptionalExtraProperty(){};
    bool meets(const Pile* c) const override;
    bool operator==(const Property& other) const override; 
protected:
    size_t calcHash() const override;       
};

/* Does this pile belong to the specified set
*/
class CardGroupProperty : public Property
{
public:    
    CardGroupProperty(const std::string& s);
    ~CardGroupProperty(){};
    bool meets(const Pile* c) const override;
    bool operator==(const Property& other) const override; 
protected:
    size_t calcHash() const override;    
private:
    std::string groupName;
};

/* Does the name of the _pile_ match the specified name
 * Note: This does not check names of individual cards within the pile
*/
class NameProperty : public Property
{
public:
     NameProperty(const std::string& name);
     ~NameProperty(){};
    bool meets(const Pile* p) const override;
    bool operator==(const Property& other) const override; 
protected:
    size_t calcHash() const override;    
private:
    const std::string pileName;
};

/* Does the selection have the specified note attached.
 * This is used to check whether particular fixes have been applied.
 * eg: have the prosperity base cards been added yet.
*/ 
class NoteProperty : public Property
{
public:
    NoteProperty(const std::string& wantNote);
    ~NoteProperty(){};
    bool meets(const Pile* p) const override {return false;}   
    bool meets(const Selection* s) const override;
    bool isSelectionProperty() const override {return true;}
    bool operator==(const Property& other) const override; 
protected:
    size_t calcHash() const override;    
private:
    const std::string note;
};

/* Does the pile have the specified "other interaction"
*/
class OtherInteractionProperty : public Property
{
public:
    OtherInteractionProperty(const std::string& otherInteracts, bool onlyKingdomAndSupply);
    virtual ~OtherInteractionProperty(){};
    virtual bool meets(const Pile* c) const override;    
    virtual bool operator==(const Property& other) const override; // to use functions as keys    
protected:
    size_t calcHash() const override;
private:
    const std::string inter;
    bool kingdomAndSupply;
};

/* Does the selection include costs with Potions in them but the Potion pile
 * has not been added yet.
*/ 
class MissingPotionProperty : public Property
{
public:
    MissingPotionProperty(){};
    virtual ~MissingPotionProperty(){}
    virtual bool meets(const Pile* c) const override {return false;};  
    virtual bool meets(const Selection* p) const override;    
    virtual bool operator==(const Property& other) const override;
    virtual bool isSelectionProperty() const override {return true;}    
protected:
    size_t calcHash() const override;
};

/* Do any cards in the selection reference a card which is not in the selection
*/ 
class MissingInteractingCardProperty : public Property
{
public:
    MissingInteractingCardProperty(){};
    virtual ~MissingInteractingCardProperty(){}
    virtual bool meets(const Pile* c) const override {return false;};  
    virtual bool meets(const Selection* p) const override;    
    virtual bool operator==(const Property& other) const override;
    virtual bool isSelectionProperty() const override {return true;}    
protected:
    size_t calcHash() const override;
};

/* Always fails
 * This is used to make constructing single run constraint actions
*/ 
class FailProperty : public Property
{
public:
    FailProperty(){};
    virtual ~FailProperty(){}
    virtual bool meets(const Pile* c) const override {return false;}
    virtual bool meets(const Selection* p) const override {return false;}    
    virtual bool operator==(const Property& other) const override;
    virtual bool isSelectionProperty() const override {return true;}    
protected:
    size_t calcHash() const override;
};

/* Checks for cards which depend on a group which has not been added.
 * Eg: Cornucopia Tournament depends on the Cornucopia-prizes group
 * When the group(X) is added a note "addedX" which we can test for
*/ 
class MissingInteractingCardGroupProperty : public Property
{
public:
    MissingInteractingCardGroupProperty(){}
    virtual ~MissingInteractingCardGroupProperty(){}
    virtual bool meets(const Pile* c) const override {return false;}
    virtual bool meets(const Selection* p) const override;
    virtual bool operator==(const Property& other) const override;
    virtual bool isSelectionProperty() const override {return true;}
protected:
    size_t calcHash() const override;
};

/* Does the selection have too many piles with the same cost
*/ 
class RepeatedCostProperty : public Property
{
public:
    RepeatedCostProperty(unsigned repeatsAllowed):maxRepeats(repeatsAllowed){}
    virtual ~RepeatedCostProperty(){}
    virtual bool meets(const Pile* c) const override {return false;}
    virtual bool meets(const Selection* p) const override;
    virtual bool operator==(const Property& other) const override;
    virtual bool isSelectionProperty() const override {return true;}
protected:
    size_t calcHash() const override;
private:
    unsigned maxRepeats;
};

/* Does the selection have interactswithKeyword(X) or react(X)
 * but no cards with keyword(X)
*/
class HangingInteractsWith : public Property
{
public:
    HangingInteractsWith(const std::string& interactsKeyword, const std::string& keyword="", const std::string& altKeyword=""):interactsWith(interactsKeyword),kw(keyword), altKw(altKeyword){}
    virtual ~HangingInteractsWith(){}
    virtual bool meets(const Pile* c) const override {return false;}
    virtual bool meets(const Selection* p) const override;
    virtual bool operator==(const Property& other) const override;
    virtual bool isSelectionProperty() const override {return true;}    
protected:
    size_t calcHash() const override;
private:
    const std::string interactsWith;
    const std::string kw;
    const std::string altKw;    
};

/* True if either of two Properties (of the same Pile vs Selection type) holds
*/
class EitherProperty : public Property
{
public:
    EitherProperty(Property* p1, Property* p2);
    virtual ~EitherProperty(){delete prop1; delete prop2;}
    virtual bool meets(const Pile* c) const override;
    virtual bool meets(const Selection* p) const override;
    virtual bool operator==(const Property& other) const override;
    virtual bool isSelectionProperty() const override;
protected:
    size_t calcHash() const override;
private:    
    Property* prop1;
    Property* prop2;
};

/* Checks for cards which have a keyword but a related group has not been added
 * Eg: Nocturne Fate cards without the Boon pile
 * Uses note checking for group addition
*/ 
class MissingGroupForKeywordProperty : public Property
{
public:
    MissingGroupForKeywordProperty(const std::string& t, const std::string& groupNeeded):type(t),note("added"+groupNeeded){}
    virtual ~MissingGroupForKeywordProperty(){}
    virtual bool meets(const Pile* c) const override {return false;}
    virtual bool meets(const Selection* p) const override;
    virtual bool operator==(const Property& other) const override;
    virtual bool isSelectionProperty() const override {return true;}
protected:
    size_t calcHash() const override;
private:
    const std::string type;
    const std::string note;
};

/* Decide if prosperity cards needs to be added.
 * Triggers if:   (Colony and Province are not both present)
 *              && (there are >= "threshold" prosperity cards
 *                   or only one of Colony or Province is present).
*/ 
class NeedProsperity : public Property
{
public:
    NeedProsperity(const CardCollection& col, ushort thr):threshold(thr),
plat(col.getPileForCard("Platinum")),
col(col.getPileForCard("Colony")) {}

    virtual ~NeedProsperity(){}
    virtual bool meets(const Pile* c) const override {return false;}
    virtual bool meets(const Selection* p) const override;
    virtual bool operator==(const Property& other) const override;
    virtual bool isSelectionProperty() const override {return true;}
protected:
    size_t calcHash() const override;
private:
    ushort threshold;
    const Pile* plat;    
    const Pile* col;
};


}
