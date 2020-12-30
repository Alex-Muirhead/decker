#include "types.h"

namespace decker
{

/* considers coin only costs from zero coins up to upper limit
*/ 
class CostUpto : public CostTarget
{
public:
    CostUpto(short matchesNeeded, short unmetW, short metW, int upper);
    virtual bool addVotes(const CostSet& currentCosts, CostVotes& votes) const override;
    virtual bool operator==(const CostTarget& other) const override;
private:
    short limit;
};

/* Considers cost in a fixed set
*/ 
class CostInSet : public CostTarget
{
public:
    CostInSet(short matchesNeeded, short unmetW, short metW, const CostSet& s);
    virtual bool addVotes(const CostSet& currentCosts, CostVotes& votes) const override;
    virtual bool operator==(const CostTarget& other) const override;        
private:
    CostSet costs;
};

/* Considers costs relative to existing costs
*/ 
class CostRelative : public CostTarget
{
public:
        // For a card costing x, (+2,false) ->  x+2, x-1, x, x-1, x-2, ...
        //    (+2, true) ->  x+2 only
    CostRelative(short matchesNeeded, short unmetW, short metW, int delta, bool strict);
    virtual bool addVotes(const CostSet& currentCosts, CostVotes& votes) const override;
    virtual bool operator==(const CostTarget& other) const override;        
private:
    int costDelta;
    bool noLess;
};

}
