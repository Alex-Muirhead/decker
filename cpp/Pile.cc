#include <iostream>
#include "types.h"
using namespace std;
namespace decker
{
Pile::Pile(const std::string& label):name(label),supply(false),kingdom(false)
{
}

Pile::~Pile()
{
    for (auto c : cards){delete c;}    
}

void Pile::addCard(const Card* c)
{
    if (!cards.insert(c).second) {
        return;
    }
    // so we haven't seen this card before
    types.insert(c->getTypes().begin(), c->getTypes().end());
    costs.insert(c->getCost());
    keywords.insert(c->getKeywords().begin(), c->getKeywords().end());
    kwInteractions.insert(c->getKWInteractions().begin(), c->getKWInteractions().end());
    otherInteractions.insert(c->getOtherInteractions().begin(), c->getOtherInteractions().end());
    cardGroup=c->getCardGroup();
    supply=c->getSupply() || supply;
    kingdom=c->getKingdom() || kingdom;
    targets.insert(c->getCostTargets().begin(), c->getCostTargets().end());
}

bool Pile::singleType() const
{
    return cards.size()==1;
}

const std::unordered_set<const Card*>& Pile::getCards() const
{
    return cards;
}

// order by pile-name, then card name
bool comparePile(const Pile* p1, const Pile* p2) {
    int pCmp=p1->getCardGroup().compare(p2->getCardGroup());
    if (pCmp<0)
    {
        return true;
    }
    else if (pCmp==0)
    {
        return p1->getName() < p2->getName();
    }
    else
    {
        return false;
    }
    return p1->getName() < p2->getName();
}


}
