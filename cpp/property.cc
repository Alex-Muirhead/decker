#include <algorithm>
#include "property.h"

/*
 * The least significant digits of the hash are determined by type
TypeProperty = 0
KeywordProperty = 5
KeywordInteractionProperty = 10
CostProperty = 15
CostAndTypeProperty = 20
KingdomAndSupplyProperty = 25
OptionalExtraProperty = 30
CardGroupProperty = 35
NameProperty = 40
NoteProperty = 45
OtherInteractionProperty = 50
MissingPotionProperty = 55
MissingInteractingCardProperty = 60
FailProperty = 65
MissingInteractingCardGroupProperty = 70
RepeatedCostProperty = 75
HangingInteractsWith = 80
EitherProperty = 85
MissingGroupForKeywordProperty = 90
NeedProsperity = 95

The gaps are to allow the range of values to include whether 
there are supply and kingdom limitations
*/

namespace
{
const unsigned TYPEWIDTH=100;   
}

using namespace std;
namespace decker
{

TypeProperty::TypeProperty(const std::string& hasType, bool restrictToKingdomAndSupply):type(hasType), kingdomAndSupply(restrictToKingdomAndSupply)
{}
    
bool TypeProperty::meets(const Pile* p) const
{
    if (kingdomAndSupply && (!p->getKingdom() || !p->getSupply()))
    {
        return false;
    }
    return find(p->getTypes().begin(), p->getTypes().end(), type)!=p->getTypes().end();
}

bool TypeProperty::operator==(const Property& other) const
{
    const TypeProperty* tp=dynamic_cast<const TypeProperty*>(&other);
    return typeid(*this)==typeid(other) and tp!=0 and tp->type==type and tp->kingdomAndSupply==kingdomAndSupply;
}

size_t TypeProperty::calcHash() const
{
    return hash<std::string>()(type)/TYPEWIDTH*TYPEWIDTH+0+kingdomAndSupply;   
}
    
bool TypeProperty::typesEqual(const TypeProperty& other) const
{
    return type==other.type and other.kingdomAndSupply==kingdomAndSupply;
}    
    
KeywordProperty::KeywordProperty(const std::string& hasKeyword, bool onlyKingdomAndSupply):keyword(hasKeyword), kingdomAndSupply(onlyKingdomAndSupply){}    
    
bool KeywordProperty::meets(const Pile* p) const
{
    if (kingdomAndSupply && (!p->getKingdom() || !p->getSupply()))
    {
        return false;
    }
    return find(p->getKeywords().begin(), p->getKeywords().end(), keyword)!=p->getKeywords().end();
}

bool KeywordProperty::operator==(const Property& other) const
{
    const KeywordProperty& kwp=dynamic_cast<const KeywordProperty&>(other);
    return typeid(*this)==typeid(other) and kwp.kingdomAndSupply==kingdomAndSupply and kwp.keyword==keyword;
}

size_t KeywordProperty::calcHash() const
{
    return (hash<std::string>()(keyword)/TYPEWIDTH)*TYPEWIDTH+5+kingdomAndSupply;
}

bool KeywordInteractionProperty::meets(const Pile* p) const
{
    return find(p->getKWInteractions().begin(), p->getKWInteractions().end(), keyword)!=p->getKWInteractions().end();
}

bool KeywordInteractionProperty::operator==(const Property& other) const
{
    return typeid(*this)==typeid(other) and dynamic_cast<const KeywordInteractionProperty&>(other).keyword==keyword;
}

size_t KeywordInteractionProperty::calcHash() const
{
    return (hash<std::string>()(keyword)/TYPEWIDTH)*TYPEWIDTH+10;   
}

CostProperty::CostProperty(Cost cost, bool inSupply):singleCost(cost), supplyOnly(inSupply)
{
}

CostProperty::CostProperty(const CostSet& costSet, bool inSupply):costs(costSet), supplyOnly(inSupply)
{
}

bool CostProperty::meets(const Pile* p) const
{
    if (supplyOnly && !p->getSupply())
    {
        return false;
    }
    if (singleCost.valid())
    {
        return p->getCosts().find(singleCost)!=p->getCosts().end();
    }
        // we need to find if there is a non-empty intersection
        // between the cost sets. I'm not using std::set_intersection
        // because I don't need to construct the intersection
    return Cost::intersects(p->getCosts(), costs);
}

bool CostProperty::operator==(const Property& other) const
{
    if (typeid(*this)!=typeid(other))
    {
        return false;
    }
    const CostProperty* cp=dynamic_cast<const CostProperty*>(&other);
    return (singleCost==cp->singleCost) && (cp->costs==costs) && (cp->supplyOnly==supplyOnly);
}

size_t CostProperty::calcHash() const
{
    size_t res=1;
    size_t max=Cost::maxHash();
    size_t cap=max*max/2;
    for (auto i : costs)
    {
        res=(3*res+i.costHash()*17)%cap;    // these are not significant primes
    }
    return res*TYPEWIDTH+15+supplyOnly;   
}

bool CostProperty::costsEqual(const CostProperty& other) const
{
    return (singleCost==other.singleCost) && (costs==other.costs);   
}

bool KingdomAndSupplyProperty::meets(const Pile* p) const 
{
    return p->getSupply() && p->getKingdom();
}

size_t KingdomAndSupplyProperty::calcHash() const
{
    return 25;
}

bool KingdomAndSupplyProperty::operator==(const Property& other) const
{
    return typeid(*this)==typeid(other);
}

// For Events, projects etc
bool OptionalExtraProperty::meets(const Pile* p) const
{
    auto end=p->getTypes().end();
    return !p->getSupply() && !p->getKingdom() && ((p->getTypes().find("Event")!=end) 
        || (p->getTypes().find("Project")!=end) || (p->getTypes().find("Landmark")!=end) 
        || (p->getTypes().find("Way")!=end));
}

bool OptionalExtraProperty::operator==(const Property& other) const
{
    return typeid(*this)!=typeid(other);
}

size_t OptionalExtraProperty::calcHash() const
{
    return 30;
}

CardGroupProperty::CardGroupProperty(const std::string& s):groupName(s)
{}

bool CardGroupProperty::meets(const Pile* p) const
{
    return p->getCardGroup()==groupName; 
}

bool CardGroupProperty::operator==(const Property& other) const
{
    if (typeid(*this)!=typeid(other))
    {
        return false;
    }
    const CardGroupProperty* sp=dynamic_cast<const CardGroupProperty*>(&other);
    return sp->groupName == groupName;
}

size_t CardGroupProperty::calcHash() const
{
    hash<std::string> hs;
    return (hs(groupName)/TYPEWIDTH)*TYPEWIDTH+35;
}

NameProperty::NameProperty(const std::string& name):pileName(name)
{}

bool NameProperty::meets(const Pile* p) const
{
    return p->getName()==pileName;
}

bool NameProperty::operator==(const Property& other) const 
{
    if (typeid(*this)!=typeid(other))
    {
        return false;
    }
    const NameProperty* np=dynamic_cast<const NameProperty*>(&other);
    return np->pileName == pileName;
}

size_t NameProperty::calcHash() const
{
    hash<std::string> hs;
    return (hs(pileName)/TYPEWIDTH)*TYPEWIDTH+40;    
}

NoteProperty::NoteProperty(const std::string& wantNote):note(wantNote){}

bool NoteProperty::meets(const Selection* s) const
{
    return s->hasNote(note);
}

bool NoteProperty::operator==(const Property& other) const
{
    if (typeid(*this)!=typeid(other))
    {
        return false;
    }
    const NoteProperty* np=dynamic_cast<const NoteProperty*>(&other);
    return np->note == note;
}

size_t NoteProperty::calcHash() const
{
    hash<std::string> hs;
    return (hs(note)/TYPEWIDTH)*TYPEWIDTH+45;
}

CostAndTypeProperty::CostAndTypeProperty(const std::string& type, Cost cost)
:CostProperty(cost),TypeProperty(type)
{}

CostAndTypeProperty::CostAndTypeProperty(const std::string& type, const CostSet& costs)
:CostProperty(costs),TypeProperty(type)
{}

bool CostAndTypeProperty::meets(const Pile* p) const
{
    return TypeProperty::meets(p) && CostProperty::meets(p);
}

bool CostAndTypeProperty::operator==(const Property& other) const
{
    if (typeid(*this)!=typeid(other))
    {
        return false;
    }
    const CostAndTypeProperty& cat=dynamic_cast<const CostAndTypeProperty&>(other);
    return costsEqual(cat) && typesEqual(cat);
}

// Not a great hash function
size_t CostAndTypeProperty::calcHash() const
{
    size_t res=CostProperty::calcHash()%(50*TYPEWIDTH)+TypeProperty::calcHash()%(TYPEWIDTH*TYPEWIDTH);
    return res*TYPEWIDTH+20;
}

OtherInteractionProperty::OtherInteractionProperty(const std::string& otherInteracts, bool onlyKingdomAndSupply):inter(otherInteracts), kingdomAndSupply(onlyKingdomAndSupply){}


bool OtherInteractionProperty::meets(const Pile* p) const
{
    if (kingdomAndSupply && (!p->getSupply() || !p->getKingdom()))
    {
        return false;
    }
    return find(p->getOtherInteractions().begin(), p->getOtherInteractions().end(), inter)!=p->getOtherInteractions().end();
}

bool OtherInteractionProperty::operator==(const Property& other) const
{
    const OtherInteractionProperty& oip=dynamic_cast<const OtherInteractionProperty&>(other);
    return typeid(*this)==typeid(other) and oip.inter==inter and oip.kingdomAndSupply==kingdomAndSupply;
}

size_t OtherInteractionProperty::calcHash() const
{
    return (hash<std::string>()(inter)/TYPEWIDTH)*TYPEWIDTH+50+kingdomAndSupply;  
}


bool MissingPotionProperty::meets(const Selection* c) const
{
    bool found=false;
    bool havePotion=false;
    for (auto p : c->getPiles())
    {
        if (p->getName()=="Potion")
        {
            havePotion=true;
            continue;
        }
        for (auto c : p->getCosts())
        {
            if (c.hasPotion())
            {
                found=true;
                break;
            }
        }
    }
    return found && !havePotion;
}

bool MissingPotionProperty::operator==(const Property& other) const
{
    return (dynamic_cast<const MissingPotionProperty*>(&other)!=0);
}

size_t MissingPotionProperty::calcHash() const
{
    return 55;
}

bool FailProperty::operator==(const Property& other) const
{
    return (dynamic_cast<const FailProperty*>(&other)!=0);
}

size_t FailProperty::calcHash() const
{
    return 65;
}


bool MissingInteractingCardProperty::meets(const Selection* s) const
{
    std::set<std::string> need;
    for (auto p : s->getPiles())
    {
        for (auto it : p->getOtherInteractions())
        {
            if (it.find("card(")==0)
            {
                std::string needName=it.substr(5, it.size()-6);
                need.insert(needName);
            }
        }
    }
    if (need.empty())
    {
        return false;
    }
    for (auto name : need)
    {
        bool found=false;
        for (auto c : s->getCards())
        {
            if (c->getName()==name)
            {
                found=true;
                break;
            }
        }
        if (!found)
        {
            return true;
        }
    }
    return false;
}

bool MissingInteractingCardProperty::operator==(const Property& other) const
{
    return (dynamic_cast<const MissingPotionProperty*>(&other)!=0);
}

size_t MissingInteractingCardProperty::calcHash() const
{
    return 60;
}


bool MissingInteractingCardGroupProperty::meets(const Selection* s) const
{
    for (auto p : s->getPiles())
    {
        for (auto it : p->getOtherInteractions())
        {
            if (it.find("group(")==0)
            {
                std::string needName=it.substr(6, it.size()-7);
                if (!s->hasNote("added"+needName))
                {
                    return true;
                }
            }
        }
    }
    return false;
}

bool MissingInteractingCardGroupProperty::operator==(const Property& other) const
{
    return (dynamic_cast<const MissingInteractingCardGroupProperty*>(&other)!=0);
}

size_t MissingInteractingCardGroupProperty::calcHash() const
{
    return 70;
}


bool RepeatedCostProperty::meets(const Selection* s) const
{
    std::map<Cost, unsigned, Cost::CostCompare> counts;
    for (const Cost& c : s->getCostSet())
    {
        counts[c]=0;
    }
    for (const Pile* p : s->getPiles())
    {
        for (const Cost& c : p->getCosts())
        {
            counts[c]++;
        }
    }
    for (auto it=counts.begin(); it!=counts.end(); ++it)
    {
        if (it->second>maxRepeats)
        {
            return true;
        }
    }
    return false;
}

bool RepeatedCostProperty::operator==(const Property& other) const
{
    const RepeatedCostProperty* rcp=dynamic_cast<const RepeatedCostProperty*>(&other);
    return (rcp!=0 && rcp->maxRepeats==maxRepeats);
}

size_t RepeatedCostProperty::calcHash() const
{
    return 75;
}

bool HangingInteractsWith::meets(const Selection* s) const
{
    if (s->getInteractsKeywords().find(interactsWith)==s->getInteractsKeywords().end())
    {
        return false; // no card has that interacts with
    }
    if (s->getKeywords().find(kw)!=s->getKeywords().end())
    {
        return false;   // we have both interaction and keyword
    }
    if (s->getKeywords().find(altKw)!=s->getKeywords().end())
    {
        return false;   // we have both interaction and keyword
    }
    return true;
}

bool HangingInteractsWith::operator==(const Property& other) const
{
    const HangingInteractsWith* rcp=dynamic_cast<const HangingInteractsWith*>(&other);
    return (rcp!=0 && rcp->interactsWith==interactsWith && rcp->kw==kw && rcp->altKw==altKw);
}

size_t HangingInteractsWith::calcHash() const
{
    return 80;  // a bit lazy
}

EitherProperty::EitherProperty(Property* p1, Property* p2)
{
    if (!p1 || !p2)
    {
        prop1=nullptr;
        prop2=nullptr;
        delete p1;
        delete p2;
    }
    else if (p1->isSelectionProperty()!=p2->isSelectionProperty())
    {
        prop1=nullptr;
        prop2=nullptr;
        delete p1;
        delete p2;        
    }
    else
    {
        prop1=p1;
        prop2=p2;
    }
}

bool EitherProperty::meets(const Pile* c) const
{
    return prop1!=0 && (prop1->meets(c) || prop2->meets(c));
}

bool EitherProperty::meets(const Selection* p) const
{
    return prop1!=0 && (prop1->meets(p) || prop2->meets(p));
}

bool EitherProperty::operator==(const Property& other) const 
{
    const EitherProperty* ep=dynamic_cast<const EitherProperty*>(&other);
    return ep!=0 && ((prop1==nullptr && ep->prop1==nullptr) || (*prop1==*(ep->prop1) && *prop2==*(ep->prop2)));
}

bool EitherProperty::isSelectionProperty() const
{
    return (prop1!=0) && (prop1->isSelectionProperty());
}

size_t EitherProperty::calcHash() const
{
    return 85;  // yes this is a bit lazy
}



bool MissingGroupForKeywordProperty::meets(const Selection* s) const
{
    for (auto p : s->getPiles())
    {
        for (auto it : p->getTypes())
        {
            if (it.find(type)==0)
            {
                if (!s->hasNote(note))
                {
                    return true;
                }
            }
        }
    }
    return false;
}

bool MissingGroupForKeywordProperty::operator==(const Property& other) const
{
    const MissingGroupForKeywordProperty* msf=dynamic_cast<const MissingGroupForKeywordProperty*>(&other);
    return (msf!=0) 
        && (type==msf->type) && (note==msf->note);
}

size_t MissingGroupForKeywordProperty::calcHash() const
{
    return 90;
}



bool NeedProsperity::meets(const Selection* p) const
{
    // If we didn't find the piles at all something
    // went wrong
    if (!col || !plat)
    {
        return false;
    }
    bool hasCol = p->contains(col);
    bool hasPlat = p->contains(plat);
    if (hasCol && hasPlat) 
    {
        return false;
    }
    if (hasCol != hasPlat)
    {
        return true;
    }
    // count how many prosperity cards we have
    ushort total = 0;
    for (auto pp : p->getPiles())
    {
        if (pp->getCardGroup().find("Prosperity") == 0)
        {
            total++;
        }
    }
    return (threshold>0) && (threshold <= total);
}


bool NeedProsperity::operator==(const Property& other) const
{
    auto po = dynamic_cast<const NeedProsperity*>(&other);
    if (po)
    {
        return po->threshold==threshold;
    }
    return false;
}

size_t NeedProsperity::calcHash() const
{
    return 95;
}


}
