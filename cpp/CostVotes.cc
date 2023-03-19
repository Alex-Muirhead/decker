#include <iostream>
#include "types.h"

namespace decker
{

CostVotes::CostVotes(const std::shared_ptr<CostSet>& legalCosts):availableCosts(legalCosts)
{
}

void CostVotes::addVote(const Cost& c, float diff)
{
    if (availableCosts->find(c)!=availableCosts->end())
    {
        auto it=votes.insert(std::make_pair(c, diff));
        if (!it.second)    // key was already present
        {
            it.first->second+=diff;
        }
    }
}

bool CostVotes::getMaxWeighted(CostSet& maxCost, float threshold, float tolerance) const
{
    float max=0;    // we'll reject any votes below zero
    for (auto v : votes)
    {
        if (v.second > max)    // yes this check is not completely well defined
        {                       // but "close to max" will do here
            max=v.second;
        }
    }
    if (max < threshold)
    {
        return false;
    }
        // ok now we sweep again and find all votes close to this    
    for (auto v : votes)
    {
        if ((max - v.second) <= tolerance)    // abs not needed since dealing with max
        {
            maxCost.insert(v.first);
        }
    }
    return max>0;
}

}

