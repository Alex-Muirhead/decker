#include "types.h"

namespace decker
{

Card::Card(const std::string& cardName, const std::string& cardPile, const std::string& groupName, bool cardInSupply, bool cardisKingdom, const std::vector<std::string>& cardTypes, const Cost& c, const std::vector<std::string>& cardKeywords, const std::vector<std::string>& interactsKeywords, const std::vector<std::string>& interactsOther, std::vector<CostTarget*>& targets)
:name(cardName), cardGroup(groupName), pile(cardPile), supply(cardInSupply), kingdom(cardisKingdom), types(cardTypes), cost(c), keywords(cardKeywords), kwInteractions(interactsKeywords), otherInteractions(interactsOther), costTargets(targets) 
{
}

}
