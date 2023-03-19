#include <algorithm>
#include <fstream>
#include <string>
#include <sstream>
#include "actions.h"
#include "types.h"
#include "property.h"
#include "util.h"

using namespace std;

namespace
{






}   // end annonymous namespace


namespace decker
{

void splitString(const std::string& line, char sep, std::vector<std::string>& result)
{
    if (line.empty()) {
        return;
    }
    size_t start=0, end;
    while (end=line.find(sep, start), end!=string::npos)
    {
        result.push_back(line.substr(start, end-start));
        start=end+1;
    }
    result.push_back(line.substr(start));
}    
    
    
CostSet collectPrices(const PileSet& piles)
{
    CostSet res;
    for (auto p : piles)
    {
        res.insert(p->getCosts().begin(), p->getCosts().end());
    }
    return res;
}
    
    
Constraint* baneConstraint(const CardCollection& coll)
{
    PropertyPtr hasYW=PropertyPtr(new NameProperty("Young Witch"));
    CostSet cs;
    cs.insert(Cost(2));
    cs.insert(Cost(3));
    PropertyPtr baneCost=PropertyPtr(new CostAndTypeProperty("Action", cs));
    PileIt begin, end;
    // TODO: Need to deal with this call failing
    coll.getIterators(PropertyPtr(baneCost), begin, end);
    ConstraintAction* fix=new FindBane(&coll, begin, end);
    PropertyPtr hasBane=PropertyPtr(new NoteProperty("hasBane"));
        // if we have less than 1 YoungWitch do nothing
        // if we have less than 1 hasBane note actionRequired   (only ever have 1 note)
        //.can accept more is empty (1,1)
        // from 1 .. MANY hasBane go inactive
        // more than MANY -> fail
    return new Constraint("bane", hasYW, hasBane, fix, 1, 1, 1, decker::MANY);
}

Constraint* prospConstraint(const CardCollection& coll)
{
    PropertyPtr groupPros=PropertyPtr(new CardGroupProperty("Prosperity"));
    PropertyPtr hasProsBase=PropertyPtr(new NoteProperty("addedProsperity-base"));
    ConstraintAction* fix=new AddGroup(&coll, "Prosperity-base");
        // if we have less than 5 Prosperity cards do nothing
        // if we have less than 1 note, action required
    return new Constraint("prospBasics", groupPros, hasProsBase, fix, 5, 1, 1, decker::MANY);
    
}


Constraint* curserConstraint(const CardCollection& coll, unsigned threshold)
{
    PropertyPtr curser=PropertyPtr(new KeywordProperty("curser", false));
    PropertyPtr trash=PropertyPtr(new KeywordProperty("trash_any", true));
    PileIt begin, end;
    if (!coll.getIterators(PropertyPtr(trash), begin, end))
    {
        return 0;
    }
    ConstraintAction* fix=new FindPile(&coll, begin, end);
    return new Constraint("counterCurser", curser, trash, fix, threshold, 1, 1, decker::MANY);
}


Constraint* attackReactConstraint(const CardCollection& coll, unsigned threshold)
{
    PropertyPtr attack=PropertyPtr(new TypeProperty("Attack"));    
        // only want kingdom and supply piles
    PropertyPtr react=PropertyPtr(new OtherInteractionProperty("react(Attack)", true));
    PileIt begin, end;
    if (!coll.getIterators(PropertyPtr(react), begin, end))
    {
        return 0;
    }
    ConstraintAction* fix=new FindPile(&coll, begin, end);
    return new Constraint("counterAttack", attack, react, fix, threshold, 1, 1, decker::MANY);
}


std::string groupNamePrefix(const std::string& groupName)
{
    auto pos=groupName.find('-');
    if (pos==std::string::npos)
    {
        return groupName;
    }
    return groupName.substr(0, pos);
}

}
