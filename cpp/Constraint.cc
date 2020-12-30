#include <iostream>
#include "types.h"
#include "property.h"

using namespace std;
namespace decker
{
Constraint::Constraint(const std::string& label, const PropertyPtr& prop, ConstraintAction* act,  unsigned min, unsigned max)
:property(prop), precondition(0), action(act),  propActive(0), propSatisfied(min),    propInactive(min),
propBroken(max+1), why(label)
{}

Constraint::Constraint(const std::string& label, const PropertyPtr& pre, const PropertyPtr& prop, ConstraintAction* act,  unsigned x, unsigned a, unsigned b, unsigned c)
:property(prop),precondition(pre), action(act), propActive(x),  propSatisfied(a),    propInactive(b),
propBroken(c),
why(label)
{}

Constraint::~Constraint()
{
    if (action)
    {
        delete action;
    }
}

// maybe want to cache these calculations?
ConsResult Constraint::getStatus(const SelectionPtr& sel) const
{
    const PileSet& piles=sel->getPiles();
    if (precondition!=0)
    {
        unsigned count=0;        
        if (precondition->isSelectionProperty())
        {
            if (precondition->meets(sel.get()))
            {
                count++;
            }
        }
        else 
        {
            for (auto p : piles)
            {
                if (precondition->meets(p))
                {
                    count++;
                }
            }
        }
        if (count<propActive)
        {
            return C_OK;
        }
    }   // so we need to test property
    unsigned count=0;
    if (property->isSelectionProperty())
    {
        if (property->meets(sel.get()))
        {
            count++;
        }
    }
    else
    {
        for (auto p : piles)
        {
            if (property->meets(p))
            {
                count++;
            }
        }
    }
    if (count>=propBroken)
    {
        return C_Fail;
    }
    if (count>=propInactive)
    {
        return C_OK;
    }
    if (count>=propSatisfied)
    {
        return C_MorePossible;
    }
    return C_ActionReq;
}


unsigned Constraint::getCount(const PileSet& piles) const
{
    unsigned count=0;
    for (auto p : piles) 
    {
        if (property->meets(p)) 
        {
            count++;
        }
    }
    return count;
}

bool Constraint::act(const SelectionPtr& start, SelectionPtr& result, std::string& message) const
{
    if (action)
    {
        return action->apply(why, start, result, message);
    }
    else
    {
cerr << "   " << this << " has a null action\n";        
        return false;   // fail if we have null action
    }
}

}
