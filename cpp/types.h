#ifndef DECKER_H
#define DECKER_H

#include <vector>
#include <string>
#include <map>
#include <set>
#include <unordered_map>
#include <unordered_set>
#include <memory>
#include "rand.h"
namespace decker
{

const unsigned MANY=5000;      // used to indicate the "end point" of an open range 
    
class Selection;
class Constraint;
class CardCollection;
class ConstraintAction;

/* Represents a single cost
 * Note that there is no total order for costs under Dominion rules
*/
class Cost
{
public:
    Cost();
    Cost(short coin);
    Cost(short coin, bool hasCoin, short potion, bool hasPotion, short debt, bool hasDebt);
    bool valid() const;
    bool operator==(const Cost& other) const;
    bool operator!=(const Cost& other) const;

    std::string getString() const;
    size_t costHash() const;
    static size_t maxHash();
    bool hasDebt() const;
    bool hasCoin() const;
    bool hasPotion() const;
    bool isCoinOnly() const;
    short getCoin() const;  // to make some of the CostTarget stuff simpler
    Cost getRelCost(int delta) const;
    
    // strict total order on cost
    // WARNING: Do not use this for game semantic cost comparison
    //  under dominion rules not all costs are comparable
    class CostCompare
    {
    public:
    bool operator()(const Cost& c1, const Cost& c2) const
    {
        return ((c1.components[1]<c2.components[1]) || 
            (c1.components[2]<c2.components[2]) || 
            (c1.components[0]<c2.components[0]));    
    }
    };       
    
    using CostSet=std::set<Cost, Cost::CostCompare>;
    
    static CostSet getCostSetUpTo(short coin);    // set of costs up to and including coin
    static CostSet getCostSetdiff(short coin, bool exact, CostSet& basis); // set of costs which have difference up to and including coin from elements in basis
    static bool intersects(const CostSet& cs1, const CostSet& cs2);    
private:
    short int components[3];
};

using CostSet=Cost::CostSet;


/*
 * map costs to votes (floats) - only costs which can actually occur are mapped
 * The aim is to merge multiple cost influences to work out which should
 * be added.
 * eg: If we have costs 2,3,4 and there is a cost exactly 2 more, 
 *   then 6 would be a possible cost to add.
*/
class CostVotes
{
public:
    CostVotes(const std::shared_ptr<CostSet>& legalCosts);
    void addVote(const Cost& c, float diff);
        // votes need to be above threshold to count, tolerance is error margin around max
    bool getMaxWeighted(CostSet& maxCost, float threshold, float tolerance) const;
private:
    const std::shared_ptr<const CostSet> availableCosts;    // may not actually need this
    std::map<Cost, float, Cost::CostCompare> votes;
};

/* Describes what costs particular cards may want to be present in the selection.
 * eg: cards exactly 2 more
*/
class CostTarget
{
public:
    CostTarget(short matchesNeeded, short unmetW, short metW);
    virtual ~CostTarget(){};
        // true if this cost target has no matching costs in currentCosts
    virtual bool addVotes(const CostSet& currentCosts, CostVotes& votes) const=0;
    virtual bool operator==(const CostTarget& other) const =0; // to use functions as keys
    size_t hashCode() const {return cacheHash;};        
protected:
    mutable size_t cacheHash;    
    short matchesRequired;
    short unmetWeight;
    short metWeight;
};

/* helper class for TargetSet*/
class TargetHasher
{
public:
    size_t operator()(const CostTarget* ct) const
    {
        return ct->hashCode();
    }
};

/* helper class for TargetSet*/
class TargetEq
{
public:
bool operator()(const CostTarget* c1, const CostTarget* c2) const
{
    return *c1==*c2; 
}
};    

// TargetHasher and TargetEq are named classes here instead of lambdas
// because you can't put lamdas in templates
using TargetSet=std::unordered_set<const CostTarget*, TargetHasher, TargetEq>;

class Card 
{
public:
    Card(const std::string& cardName, const std::string& cardPile, const std::string& groupName, bool cardInSupply, bool cardisKingdom, const std::vector<std::string>& cardTypes, const Cost& c, const std::vector<std::string>& cardKeywords, const std::vector<std::string>& interactsKeywords, const std::vector<std::string>& interactsOther, std::vector<CostTarget*>& targets); 
    
    ~Card() {for (auto c : costTargets){delete c;}}
    
    const std::string& getName() const {return name;}    
    const std::string& getCardGroup() const {return cardGroup;}
    const std::string& getPileName() const {return pile;}
    bool getSupply() const {return supply;}
    bool getKingdom() const {return kingdom;}
    const std::vector<std::string>& getTypes() const {return types;}
    const Cost& getCost() const {return cost;}
    const std::vector<std::string>& getKeywords() const{return keywords;}
    const std::vector<std::string>& getKWInteractions() const{return kwInteractions;}  
    const std::vector<std::string>& getOtherInteractions() const{return otherInteractions;}
    const std::vector<CostTarget*>& getCostTargets() const{return costTargets;}
private:
    std::string name;    
    std::string cardGroup;
    std::string pile;
    bool supply;
    bool kingdom;
    std::vector<std::string> types;
    Cost cost;
    std::vector<std::string> keywords;
    std::vector<std::string> kwInteractions;  
    std::vector<std::string> otherInteractions;
    std::vector<CostTarget*> costTargets;
};

using CardSet=std::unordered_set<const Card*>;

class Pile
{
public:
    Pile(const std::string& label);
    ~Pile();
    void addCard(const Card* c);
    bool singleType() const;
    const std::string& getCardGroup() const {return cardGroup;}
    bool getSupply() const {return supply;}
    bool getKingdom() const {return kingdom;}
    const std::set<std::string>& getTypes() const {return types;}
    const CostSet& getCosts() const {return costs;}
    const std::set<std::string>& getKeywords() const {return keywords;}
    const std::set<std::string>& getKWInteractions() const {return kwInteractions;}
    const std::set<std::string>& getOtherInteractions() const {return otherInteractions;}
    const std::string& getName() const {return name;}
    const std::unordered_set<const Card*>& getCards() const;
    
    const TargetSet& getTargets() const {return targets;}
private:
    const std::string name;
    std::string cardGroup;
    bool supply;
    bool kingdom;
    std::set<std::string> types;
    CostSet costs;
    std::set<std::string> keywords;
    std::set<std::string> kwInteractions; 
    std::set<std::string> otherInteractions;    
    std::unordered_set<const Card*> cards;
    TargetSet targets;
};

using PileIt=std::vector<const Pile*>::iterator;    

// order by pile-name, then card name
// Used for output to user
bool comparePile(const Pile* p1, const Pile* p2);


class PileSetOrder
{
public:
    bool operator()(const Pile* p1, const Pile* p2) const {
        return p1->getName() < p2->getName();
    }
};

using PileSet=std::set<const Pile*, PileSetOrder>;


class Property
{
public:
    Property():cacheHash(0){}
    virtual bool isSelectionProperty() const {return false;}
    virtual bool meets(const Pile* c) const=0;
    virtual bool meets(const Selection* p) const {return false;}
    virtual ~Property(){};
    virtual bool operator==(const Property& other) const =0; // to use functions as keys
    size_t hashCode() const {return cacheHash?cacheHash:(cacheHash=calcHash());};
protected:
    virtual size_t calcHash() const =0;
    mutable size_t cacheHash;
};

using PropertyPtr=std::shared_ptr<const Property>;


class Selection
{
public:
    Selection(const CardCollection* coll, PileIt generalBegin, PileIt generalEnd, short marketCap=10, short uncheckedCount=0);
    explicit Selection(const Selection& old);
    ~Selection();
    void addConstraint(Constraint* cons);
    void increaseRequiredPiles();   // only use so far is to make space for "bane" card
    bool addPile(const Pile* p);
    void dump(bool showAll, bool showCardInfo) const;
    void tagPile(const Pile* p, const std::string& s);
    short getNormalPileCount() const;
    short getRequiredCount() const;
    bool contains(const Pile* p) const;
    
    const PileSet& getPiles() const;
    const CardSet& getCards() const;
    void addNote(const std::string& n);
    bool hasNote(const std::string& n) const;
    void addItem(const std::string& s);
    const Pile* getGeneralPile();
    
    const CostSet& getCostSet() const {return costsInSupply;}

    bool needToCheckCostTargets() const {return targetCheckRequired;}
    void setNeedToCheck(bool v, const std::string& s);
    std::string getTargetString() const {return targetBlame;}
    const TargetSet& getTargetSet() const {return targets;}
    const CardCollection* getCollection() const {return cardColl;}
    const std::map<const std::string, unsigned>& getInteractsKeywords() const {return interactsKeywords;}
    const std::map<const std::string, unsigned>& getKeywords() const {return keywords;}
private:
    PileSet piles;
    CardSet cards;
    std::shared_ptr<std::vector<const Constraint*> > constraints;
    std::map<const Pile*, std::vector<std::string>> tags;
    short requiredCards;
    short currentNormalPileCount;   // <= requiredCards
    std::set<std::string> notes;
    std::set<std::string> needItems;
    PileIt beginGeneral, endGeneral;    // Where to draw cards from when no other constraints
    friend class CardCollection;
    friend class Constraint;
    
    CostSet costsInSupply;  // costs of cards _in the supply_
    
    bool targetCheckRequired;
    TargetSet targets;
    std::string targetBlame;    // piles responsible for cost target
    
    const CardCollection* cardColl; // need this to able to query for cards with specified cost
    
    std::map<const std::string, unsigned> interactsKeywords;
    std::map<const std::string, unsigned> keywords;
};

using SelectionPtr=std::shared_ptr<Selection>;


// Indicates relationship between the constraint and the current selection
enum ConsResult
{
    C_OK, // Constraint is neutral/inactive on the selection
    C_ActionReq, // something needs to be done to satisfy constraint
    C_MorePossible,   // Constraint is satisfied but could accept additional cards
    C_Fail          // constraint can not be satisfied from current selection
};


// Do not attempt to use this as a key for anything
// the underlying property or bounds may change
// For this reason, copy Constraint objects before changing
// Thresholds:
//    If precondition matches (0..x) : constraint is inactive
//    prop matches < a : action required
//    prop matches (a, b) : More possible (if you want to allow this constraint to make suggestions after immediate satisfaction)
//    prop matches (b, c) : inactive additional matching is ok but not suggested
//    prop matches > c : fail
//
//    a =< b =< c
class Constraint
{
public:
    Constraint(const std::string& label, const PropertyPtr&  prop, ConstraintAction* act, unsigned min, unsigned max);
    Constraint(const std::string& label, const PropertyPtr& pre, const PropertyPtr& prop, ConstraintAction* act, unsigned x, unsigned a, unsigned b, unsigned c);
    ~Constraint();    
    ConsResult getStatus(const SelectionPtr& piles) const;
    unsigned getCount(const PileSet& piles) const;
    bool act(const SelectionPtr& start, SelectionPtr& result, std::string& message) const;
    const std::string& getWhy() const{return why;}
private:
    const PropertyPtr property;
    const PropertyPtr precondition;
    ConstraintAction* action;
    unsigned propActive;  // x      
    unsigned propSatisfied;    // a 
    unsigned propInactive;    // b  
    unsigned propBroken;   // c  
    std::string why;
};

class ConstraintAction
{
public:    
    ConstraintAction(const CardCollection* col):collection(col){};
    virtual ~ConstraintAction(){};
        // true if found a selection solution (result)
        // false, no solution
    virtual bool apply(const std::string& label, const SelectionPtr& start, SelectionPtr& result, std::string& message)=0;
protected:
    const CardCollection* collection;
};

enum CollectionStatus
{
    Coll_OK=0,
    Coll_Warning=1,
    Coll_Fatal=2
};


// the idea is that this object will hold the actual Card* and everything else will 
// be borrowing pointers
class CardCollection
{
public:
    CardCollection(std::unique_ptr<PileSet>& p);     
    ~CardCollection();
    bool getIterators(const PropertyPtr& p, PileIt& begin, PileIt& end) const;
    void shuffle(RandStream& r);
    SelectionPtr generateSelection(short marketCap, short landscapes, PileSet& includes, std::string& message, std::vector<Constraint*> cons, RandStream* rand) const;
    SelectionPtr startSelection(short marketCap, short landscapes) const; 
    bool buildSelection(const SelectionPtr& start, SelectionPtr& result, std::string& message) const;
    void finishSelection(SelectionPtr& sel, RandStream* rand) const;
    CollectionStatus validateCollection(std::set<std::string>& warnings);
    void dump() const;
    
        // find pile containing cards called s
    const Pile* getPileForCard(const std::string& s) const;
    const std::vector<const Pile*>& getPiles() const {return piles;}
private:
    struct PropertyHasher 
    {
        inline size_t operator()(const PropertyPtr& p) const 
        {
            return p->hashCode();
        }
    };

    struct PropertyEqual 
    {
        inline bool operator()(const PropertyPtr& p1, const PropertyPtr& p2) const   
        {
            return *p1==*p2;
        }
    };        
    
    std::map<std::string, std::vector<std::string>> boxContents;
    std::set<const Card*> cards;   // need this for validating collection 
                                //(borrowed pointers from Pile* held in "piles"
    std::set<std::string> groupNames; // need this for validating collection
    std::set<std::string> cardNames;    // need this for validating collection
    mutable std::unordered_map<const PropertyPtr, std::vector<const Pile*>, PropertyHasher, PropertyEqual> propLists;
    std::vector<const Pile*> piles; // other Pile* are borrowed from here
    PropertyPtr generalProperty;
    std::shared_ptr<CostSet> legalCosts;
    

};

}   // end of namespace


#endif
