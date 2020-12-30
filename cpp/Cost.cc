#include <sstream>
#include "types.h"

namespace decker
{

// Internally use -1 to indicate no cost of that type
// Why not use 0? because curses and coppers have a coin cost of 0.
#define NOCOST -1
#define COIN 0
#define POTION 1
#define DEBT 2

Cost::Cost()
{
    components[COIN]=NOCOST;
    components[POTION]=NOCOST;
    components[DEBT]=NOCOST;
}
    
Cost::Cost(short coin)
{
    components[COIN]=coin;
    components[POTION]=NOCOST;
    components[DEBT]=NOCOST;
}

Cost::Cost(short coin, bool hasCoin, short potion, bool hasPotion, short debt, bool hasDebt)
{
    components[COIN]=hasCoin?coin:NOCOST;
    components[POTION]=hasPotion?potion:NOCOST;
    components[DEBT]=hasDebt?debt:NOCOST;
}

bool Cost::valid() const
{
    return !((components[COIN]==NOCOST) && (components[POTION]==NOCOST)
        && (components[DEBT]==NOCOST));
}

bool Cost::operator==(const Cost& other) const
{
    return components[COIN]==other.components[COIN] && 
           components[POTION]==other.components[POTION] &&
           components[DEBT]==other.components[DEBT];
}

bool Cost::operator!=(const Cost& other) const
{
    return components[COIN]!=other.components[COIN] || 
           components[POTION]!=other.components[POTION] ||
           components[DEBT]!=other.components[DEBT];
}

bool Cost::hasDebt() const
{
    return components[DEBT]!=NOCOST;
}

bool Cost::hasCoin() const
{
    return components[COIN]!=NOCOST;
}

bool Cost::hasPotion() const
{
    return components[POTION]!=NOCOST;
}

short Cost::getCoin() const
{
    return components[COIN];
}

bool Cost::isCoinOnly() const
{
    return components[POTION]==NOCOST && components[DEBT]==NOCOST;
}

std::string Cost::getString() const
{
    std::ostringstream oss;
    oss << '(';
    if (hasCoin())
    {
        oss << components[COIN];
    }
    oss << ',';
    if (hasPotion())
    {
        oss << components[POTION] << 'P';
    }
    oss << ',';
    if (hasDebt())
    {
        oss << components[DEBT] << 'D';
    }
    oss << ')';
    return oss.str();
}

//assumes nothing ever costs more than 20 coin, 20 debt, 1 potion for a max value of 881
size_t Cost::costHash() const
{
    return ((components[COIN]*21+components[DEBT])*2+components[POTION])%882;    
}

size_t Cost::maxHash()
{
    return 881;
}

Cost Cost::getRelCost(int delta) const
{
    short newCoin=components[COIN]+delta;
    if (newCoin<0)
    {
        newCoin=0;
    }
    Cost res(newCoin);
    res.components[DEBT]=components[DEBT];
    res.components[POTION]=components[POTION];
    return res;
}

// This is a separate class to avoid a declaration loop
// with the CostSet typedef
CostSet Cost::getCostSetUpTo(short coin)
{
    CostSet res;
    for (short i=0;i<=coin;++i)
    {
        res.insert(Cost(i));
    }
    return res;
}

// set of costs which have difference up to and including (or exactly in case of "exact") coin from elements in basis
CostSet Cost::getCostSetdiff(short coin, bool exact, CostSet& basis)
{
    CostSet res;
    for (const Cost c : basis)
    {
        if (c.components[COIN]==NOCOST)    // Can't be x more than no coin cost
        {
            continue;
        } 
        if (exact) 
        {
            short newCoin=c.components[COIN]+coin;
            Cost n(newCoin, true, c.components[POTION], 
                             c.components[POTION]!=NOCOST, c.components[DEBT],
                             c.components[DEBT]!=NOCOST);
            res.insert(n);
            if (c.components[COIN]>=coin)
            {
                newCoin=c.components[COIN]-coin;
                Cost n(newCoin, true, c.components[POTION], 
                             c.components[POTION]!=NOCOST, c.components[DEBT],
                             c.components[DEBT]!=NOCOST);
            }
            res.insert(n);
        }
        else    // Not exact
        {
            for (short delta=0; delta<=coin; ++delta)
            {
                Cost n(c.components[COIN]+delta, true, c.components[POTION], 
                             c.components[POTION]!=NOCOST, c.components[DEBT],
                             c.components[DEBT]!=NOCOST);
                res.insert(n);
            }
            for (short delta=0; delta<=coin; ++delta)
            {
                if (c.components[COIN]>=delta) 
                {
                    Cost n(c.components[COIN]-delta, true, c.components[POTION], 
                                c.components[POTION]!=NOCOST, c.components[DEBT],
                                c.components[DEBT]!=NOCOST);
                    res.insert(n);
                }
                else
                {
                    break;
                }
            }
            
        }
    }
    return res;
}

bool Cost::intersects(const CostSet& cs1, const CostSet& cs2)
{
    static Cost::CostCompare comp;
    auto it1=cs1.begin();
    auto en1=cs1.end();
    auto it2=cs2.begin();
    auto en2=cs2.end();
    while (it1!=en1 && it2!=en2) {
        if (*it1==*it2) 
        {
            return true;
        }    
        if (comp(*it1,*it2)) // if it1 is "smaller" move it forward
        {
            ++it1;
        }
        else
        {
            ++it2;
        }
    }
    return false;
    
}



}
