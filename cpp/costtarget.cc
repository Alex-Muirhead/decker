#include <iostream>
#include "costtarget.h"

namespace decker
{

CostTarget::CostTarget(short matchesNeeded, short unmetW, short metW):cacheHash(0),matchesRequired(matchesNeeded)
{
    unmetWeight=unmetW;
    metWeight=metW;    
        // just in case anyone tries to get "clever"
    if (unmetWeight<metWeight)
    {
        int temp=unmetWeight;
        unmetWeight=metWeight;
        metWeight=temp;
    }
}
    
CostUpto::CostUpto(short matchesNeeded, short unmetW, short metW, int upper):CostTarget(matchesNeeded, unmetW, metW), limit(upper)
{
}

bool CostUpto::addVotes(const CostSet& currentCosts, CostVotes& votes) const
{
    int matchCount=0;       // costs which meet the criterian
    for (const Cost& c : currentCosts)
    {
        if (c.isCoinOnly())
        {
            if (c.getCoin()<=limit)
            {
                matchCount++;
            }
        }
    }
    float weight=(matchCount>=matchesRequired)?(float(metWeight)/limit):(float(unmetWeight)/limit);
    for (short i=1;i<=limit;++i)
    {
        votes.addVote(Cost(i), weight);
    }
    return matchCount<matchesRequired;
}

bool CostUpto::operator==(const CostTarget& other) const
{
    const CostUpto* obj=dynamic_cast<const CostUpto*>(&other);
    return (obj!=0) && (obj->limit==limit);
}


CostInSet::CostInSet(short matchesNeeded, short unmetW, short metW, const CostSet& s):CostTarget(matchesNeeded, unmetW, metW), costs(s){}

bool CostInSet::addVotes(const CostSet& currentCosts, CostVotes& votes) const 
{
    int matchedCount=0;
    for (const Cost& c : currentCosts)  // yes looping over costs would be quicker
    {                                   // but we need to detect multiple instances
        if (costs.find(c)!=costs.end())
        {
            matchedCount++;
        }
    }
    float weight=(matchedCount>=matchesRequired)?((float)metWeight/costs.size()):((float)unmetWeight/costs.size());
    for (const Cost& c : costs)
    {
        votes.addVote(c, weight);
    }
    return matchedCount<matchesRequired;
}

bool CostInSet::operator==(const CostTarget& other) const
{
    const CostInSet* obj=dynamic_cast<const CostInSet*>(&other);
    return (obj!=0) && (obj->costs==costs); 
}

CostRelative::CostRelative(short matchesNeeded, short unmetW, short metW, int delta, bool strict):CostTarget(matchesNeeded, unmetW, metW), costDelta(delta),noLess(strict){}

    // What consitutes matching here. People might like to use this to get a more expensive
    // card but technically any cost in range will do
    // also remember delta could be negative
bool CostRelative::addVotes(const CostSet& currentCosts, CostVotes& votes) const 
{
    int matchedCount=0;
    for (const Cost& c : currentCosts)  // NOTE: this doesn't take less than costs into account
    {                                   
        Cost adjCost=c.getRelCost(costDelta);
        if (currentCosts.find(adjCost)!=currentCosts.end())
        {
            matchedCount++;
        }
    }
        // not much thought went into these particular values
        // other than to give a bit of a boost to costs above the current one
    float boost=float(unmetWeight-metWeight)/costDelta;
    float weight=float(metWeight)/currentCosts.size();
    if (costDelta<0)
    {
        for (const Cost& c : currentCosts)
        {
            if (!c.hasCoin())   // costs without coin components can't do coin relative costs
            {
                continue;
            }
            if (c.getCoin()<-costDelta) // don't let cost drop below zero
            {
                continue;
            }
            Cost target=c.getRelCost(costDelta);
            if (!noLess)
            {
                for (; target.getCoin()>=0; target=target.getRelCost(-1))
                {
                    votes.addVote(target, weight);
                }
            }
        }
    }
    else    // delta >=0
    {
        for (const Cost& c : currentCosts)
        {            
            if (!c.hasCoin())   // costs without coin components can't do coin relative costs
            {
                continue;
            }
            Cost target=c.getRelCost(costDelta);
            if (noLess)
            {
                votes.addVote(target, weight+boost);
            }
            else
            {
                for (;(target!=c);target=target.getRelCost(-1))
                {
                    votes.addVote(target, weight+boost);
                }
                for (unsigned short i=0;(i<costDelta) && (target.getCoin()>0); target=target.getRelCost(-1),++i)
                {
                    votes.addVote(target, weight);
                }
                votes.addVote(target, weight);
            }
        }
    }
    return matchedCount<matchesRequired;
}

bool CostRelative::operator==(const CostTarget& other) const
{
    const CostRelative* obj=dynamic_cast<const CostRelative*>(&other);
    return (obj!=0) && (obj->costDelta==costDelta) && (obj->noLess == noLess);
}


}
