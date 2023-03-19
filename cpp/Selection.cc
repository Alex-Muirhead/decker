#include <iostream>
#include <algorithm>
#include "types.h"

using namespace std;

namespace
{
void addCount(std::map<const std::string, unsigned>& m, const std::string& key)
{
    auto it=m.find(key);
    if (it==m.end())    // no record yet
    {
        m.insert(make_pair(key, 1));
    }
    else
    {
        it->second++;
    }
}

struct cleanup
{
    void operator()(std::vector<const decker::Constraint*>* v)
    {
        for (size_t i=0;i<v->size();++i)
        {
            delete (*v)[i];
        }
        delete v;
    }
};
    
}

namespace decker
{
    
// MarketCap is how many kingdom cards should be in the supply
Selection::Selection(const CardCollection* coll, PileIt generalBegin, 
    PileIt generalEnd, short marketCap)
    :constraints(new std::vector<const Constraint*>(), cleanup()),
    requiredCards(marketCap),
    currentNormalPileCount(0),beginGeneral(generalBegin), 
    endGeneral(generalEnd), targetCheckRequired(false), cardColl(coll)
{
}

Selection::Selection(const Selection& old)
:piles(old.piles), cards(old.cards), constraints(old.constraints), tags(old.tags), requiredCards(old.requiredCards),  currentNormalPileCount(old.currentNormalPileCount), notes(old.notes),
beginGeneral(old.beginGeneral), endGeneral(old.endGeneral), costsInSupply(old.costsInSupply), targetCheckRequired(old.targetCheckRequired), targets(old.targets), targetBlame(old.targetBlame),cardColl(old.cardColl),
interactsKeywords(old.interactsKeywords),keywords(old.keywords)
{}

Selection::~Selection()
{
}

void Selection::addConstraint(Constraint* cons)
{
    constraints->push_back(cons);
}

// I was hoping to do this using a constraint
// but it complicated things too much
void Selection::increaseRequiredPiles()
{
    requiredCards++;
}

// Constraints must be checked separately
bool Selection::addPile(const Pile* p)
{
    auto res=piles.insert(p);
    if (!res.second) // it was already there
    {
        return false;
    }
    if (p->getSupply() && p->getKingdom())
    {
        if (currentNormalPileCount>=requiredCards)
        {
            return false; // silent failure if no room to add card
        }
        currentNormalPileCount++;
    }
    for (auto c : p->getCards())
    {
        cards.insert(c);
        if (c->getSupply())
        {
            costsInSupply.insert(c->getCost());
        }
    }
    if (!p->getTargets().empty())
    {
        setNeedToCheck(true, p->getName());
        targetCheckRequired=true;
        auto targs=p->getTargets();
        targets.insert(targs.begin(), targs.end());
    }
    for (auto kw : p->getKeywords())
    {
        addCount(keywords, kw);
    }
    for (auto ikw : p->getKWInteractions())
    {
        addCount(interactsKeywords, ikw);
    }
    for (auto react : p->getOtherInteractions())
    {
        if (react.find("react(")==0)
        {
            addCount(interactsKeywords, react.substr(6,react.size()-7));
        }
    }
    return true;
}

// warning: will allow tagging of piles that aren't currently selected
void Selection::tagPile(const Pile* p, const std::string& s)
{
    auto it=tags.find(p);
    if (it==tags.end())
    {
        vector<string> v;
        v.push_back(s);
        tags.insert(make_pair(p,v));
    }
    else
    {
        it->second.push_back(s);
    }
}

void Selection::dump(bool showAll, bool showCardInfo) const
{
    std::vector<const Pile*> result;
    result.reserve(piles.size());
    for (auto it=piles.begin();it!=piles.end();++it)
    {
        result.push_back(*it);
    }
    sort(result.begin(), result.end(), comparePile);
    std::string groupName;
    std::set<std::string> items;
    size_t maxLen=0;
    for (const Pile* p : result)
    {
        size_t l=p->getName().size();
        maxLen=(maxLen>l)?maxLen:l;
    }
    for (const Pile* p : result)
    {
        if (p->getCardGroup()!=groupName)
        {
            groupName=p->getCardGroup();
            cout << "From " << groupName << endl;
        }
        cout << "   " << p->getName();
        auto it=tags.find(p);
        if (it!=tags.end())
        {
            bool first=true;
            for (const string& s : it->second)
            {
                if (showAll || s.find('<')==std::string::npos)
                {
                    cout << (first?" (":", ") << s;
                    first=false;
                }
            }
            if (!first) 
            {
                cout << ')';
            }
        }
        if (showCardInfo)
        {
            for (size_t pad=p->getName().size(); pad<maxLen;++pad)
            {
                cout << ' ';
            }
            cout << " types=";
            bool first=true;
            for (auto s : p->getTypes())
            {
                if (first)
                {
                    first=false;
                }
                else
                {
                    cout << ", ";
                }
                cout << s;
            }
            cout << " costs={";
            first=true;
            for (auto c : p->getCosts())
            {
                if (first)
                {
                    first=false;
                }
                else
                {
                    cout << ", ";
                }
                cout << c.getString();
            }
            cout << "}";
        }
        cout << endl;
        for (const string& s : p->getOtherInteractions())
        {
            if (s.find("item(")==0)
            {
                items.insert(s.substr(5, s.size()-6));
            }
        }
    }
    for (const string& s : needItems)
    {
        items.insert(s);
    }
    if (!items.empty())
    {
        cout << "Need the following items:" << endl;
        for (string s : items)
        {
            cout << "   " << s << endl;
        }
    }
}

short Selection::getNormalPileCount() const
{
    return currentNormalPileCount;
}

short Selection::getRequiredCount() const
{
    return requiredCards;
}

bool Selection::contains(const Pile* p) const
{
    return piles.find(p)!=piles.end();
}

const PileSet& Selection::getPiles() const
{
    return piles;
}

const CardSet& Selection::getCards() const
{
    return cards;
}

void Selection::addNote(const std::string& n)
{
    notes.insert(n);
}

bool Selection::hasNote(const std::string& n) const
{
    return notes.find(n)!=notes.end();
}

const Pile* Selection::getGeneralPile()
{
    if (beginGeneral==endGeneral)
    {
        return 0;
    }
    const Pile* p=*beginGeneral;
    beginGeneral++;
    return p;
}

void Selection::addItem(const std::string& s)
{
    needItems.insert(s);
}

void Selection::setNeedToCheck(bool v, const std::string& s)
{
    if (!targetCheckRequired && v)  // transition from false to true
    {
        targetBlame="";    
    }
    targetCheckRequired=v;
    if (v)
    {
        if (!targetBlame.empty())
        {
            targetBlame+=',';
        }
        targetBlame+=s;
    }  
}

}
