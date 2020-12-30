#include "types.h"

namespace decker
{
void splitString(const std::string& line, char sep, std::vector<std::string>& result);    
    
    // make a set of all prices currently in a collection
CostSet collectPrices(const PileSet& piles);

Constraint* baneConstraint(const CardCollection& coll);
Constraint* prospConstraint(const CardCollection& coll);
    // if there are >= threshold cursers, add a general trasher
Constraint* curserConstraint(const CardCollection& coll, unsigned threshold);
    // if there are >= threshold attacks, add a Reaction(attack) 
Constraint* attackReactConstraint(const CardCollection& coll, unsigned threshold);

std::string groupNamePrefix(const std::string& groupName);

}
