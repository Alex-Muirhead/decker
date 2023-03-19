#include <fstream>
#include <iostream>
#include <set>
#include <cstdlib>
#include <algorithm>
#include <sstream>
#include <iostream>

#include "types.h"
#include "property.h"
#include "util.h"
#include "actions.h"

using namespace std;

namespace decker
{

CardCollection::CardCollection(unique_ptr<PileSet>& ps):generalProperty(new KingdomAndSupplyProperty()),legalCosts(std::shared_ptr<CostSet>(new CostSet))
{
    for (auto p : *ps)
    {
        piles.push_back(p);
        groupNames.insert(p->getCardGroup());
            // Now add cards from the pile
        for (auto it=p->getCards().begin(); it!=p->getCards().end(); ++it)
        {
            cards.insert(*it);
            cardNames.insert((*it)->getName());
        }
        for (auto it=p->getCosts().begin();it!=p->getCosts().end(); ++it)
        {
            legalCosts->insert(*it);
        }
    }
        // sort piles by name
    sort(piles.begin(), piles.end(), comparePile);
    ps.reset();     // Since we are taking ownership of the Pile*
                // destroy the original PileSet
}

CardCollection::~CardCollection()
{
    for (size_t i=0; i<piles.size(); ++i)
    {
        delete piles[i];
    }
}

void CardCollection::shuffle(RandStream& r)
{
    unsigned size=piles.size();
    // go through the pile vector 3 times and swap items 
    for (int i=0;i<3;++i)
    {
        for (size_t j=0;j<size;++j)
        {
            unsigned pos=r.get()%size;
            const Pile* temp=piles[pos];
            piles[pos]=piles[j];
            piles[j]=temp;
        }
    }
}

bool CardCollection::getIterators(const PropertyPtr& p, PileIt& begin, PileIt& end) const {
    // do we have this one?
    auto it=propLists.find(p);
    if (it!=propLists.end())
    {
        begin=it->second.begin();
        end=it->second.end();
        return true;
    }
    if (p->isSelectionProperty())   // can't find piles which would satisfy an overall
    {
        return false;
    }
    auto result=make_pair(p, vector<const Pile*>());
    for (size_t i=0;i<piles.size();++i)
    {
        if (p->meets(piles[i]))
        {
            result.second.push_back(piles[i]);
        }
    }
    if (result.second.empty())
    {
        return false;
    }
    auto resIt=propLists.insert(result);
    if (!resIt.second)
    {    
        return false;     // this should never happen since we've checked earlier
    }
    begin=resIt.first->second.begin();
    end=resIt.first->second.end();
    return true;
}

SelectionPtr CardCollection::startSelection(short marketCap, short landscapes) const
{
    CardGroupProperty* base=new CardGroupProperty("base");
    PileIt begin, end;
    if (!getIterators(PropertyPtr(base), begin, end))
    {
        return SelectionPtr();
    }
    PileIt beginGeneral, endGeneral;
    getIterators(generalProperty, beginGeneral, endGeneral);
    Selection* s=0;
    if (marketCap==0)
    {
        s=new Selection(this, beginGeneral, endGeneral);
    } 
    else
    {
        s=new Selection(this, beginGeneral, endGeneral, marketCap);
    }
    for (;begin!=end; ++begin)
    {
        if (!s->addPile(*begin))    // you should never fail to add base cards
        {
            return SelectionPtr(s);
        }
    }
    if (landscapes>0)
    {
        OptionalExtraProperty* oep=new OptionalExtraProperty();
        if (getIterators(PropertyPtr(oep), begin, end))
        {
            for (int count=0;count<landscapes && begin!=end;++count, ++begin)
            {
                if (!s->addPile(*begin))    // should not be able to fail adding landscapes
                {
                    return SelectionPtr(s);
                }
            }
        }
    }
    return SelectionPtr(s);
}

// Cleanup to check for extra elements like vp tokens which
// don't really need a constraint to catch
void CardCollection::finishSelection(SelectionPtr& sel, RandStream* rand) const
{
    if (sel.get()==0)
    {
        return;
    }
        // check to see if we need to add DarkAges-base cards
        // rules say to do it based on randomness eg the last card added
        // but we don't know what order things were drawn
        // do we have any DarkAges cards
    unsigned daCount=0;
    unsigned ksCount=0; // count of kingdom and supply
    for (auto p : sel->getPiles())
    {
        if (p->getSupply() && p->getKingdom())
        {    
            ksCount++;
            if (p->getCardGroup()=="DarkAges")
            {
                daCount++;
            }
        }
    }
    if (daCount>0)
    {
            // if the random is less than the number of number of 
            // DarkAges cards, add the DarkAges base cards to replace Estate
        unsigned short r=rand->get()%ksCount;
        if (r<daCount)
        {
                // need to add all piles from that group
            CardGroupProperty* ps=new CardGroupProperty("DarkAges-base");
            PileIt begin, end;
                // If we can't add this for some reason do nothing
            if (sel->getCollection()->getIterators(PropertyPtr(ps), begin, end))
            {
                for (;begin!=end;++begin)
                {
                    if (sel->addPile(*begin))
                    {
                        sel->tagPile(*begin, "<why?had enough DarkAges cards>");
                        sel->tagPile(*begin, "Replaces Estate in starting deck");
                    }
                }
                sel->addNote("addedDarkAges-base");                
            }
        }
    }
    
    for (const Pile* p : sel->getPiles())
    {
        if (p->getKeywords().find("+point")!=p->getKeywords().end())
        {
            sel->addItem("points(shield) tokens");
            break;
        }
    }
    for (const Pile* p : sel->getPiles())
    {
        for (const Cost& c : p->getCosts())  // yes this loop runs longer than it needs to
        {
            if (c.hasDebt())
            {
                sel->addItem("debt tokens");
                break;
            }
        }
    }
    for (const Pile* p : sel->getPiles())
    {
        if (p->getKeywords().find("+coffers")!=p->getKeywords().end())
        {
            sel->addItem("coin tokens");
            sel->addItem("coffers/villagers mat");
            break;
        }
    }
    for (const Pile* p : sel->getPiles())
    {
        if (p->getKeywords().find("+villagers")!=p->getKeywords().end())
        {
            sel->addItem("coin tokens");
            sel->addItem("coffers/villagers mat");
            break;
        }
    }
    for (const Pile* p : sel->getPiles())
    {    
        if (p->getTypes().find("Heirloom")!=p->getTypes().end())
        {
            sel->tagPile(p, "Replaces one Copper in starting deck");
            // no break because multiple could be in play
        }    
    }
}

// Wrapper for starting, building and finishing a selection
SelectionPtr CardCollection::generateSelection(short marketCap, short landscapes, PileSet& includes, std::string& message, std::vector<Constraint*> cons, RandStream* rand) const
{
    SelectionPtr sel=startSelection(marketCap, landscapes);
    if (sel.get()==0)
    {
        return sel;
    }
    SelectionPtr res;
    for (auto c : cons)
    {
        sel->addConstraint(c);
    }
    for (auto p : includes)
    {
        sel->addPile(p);
        sel->tagPile(p, "<why?--included>");
    }
    if (!buildSelection(sel, res, message))
    {
        return res;
    }
    finishSelection(res, rand);
    return res;
}

// 1. check all constraints to see if anything needs to be fixed from current cards
// 2. Make sure we aren't trying to add more (supply) cards than we are allowed
// 3. If the current selection meets all requirements, set result to that selection and return true
// 4. If some requirement can not be met (eg a call to act or addPile fails), return false.
// 5. If we need more cards, pick a new pile to add and recurse
// This method _may_ modify the "start" it is given (instead of making a new clone
//  to mod). So you should clone your selection before calling this.
// ConstraintAction::act is expected to:
//     1. make whatever changes
//     2. call buildSelection() with the resulting selection
//     3. return the result of the build selection
//
// Checked are done in the following order:
// 1. Are any constraints failed?
// 2. Do any constraints require action?
// 3. Do any constraints have suggested actions?
// 4. Anything to add based on cost targets?
// 5. Try adding general cards (list of all available cards).
bool CardCollection::buildSelection(const SelectionPtr& start, SelectionPtr& result, std::string&  message) const
{
    auto size=start->constraints->size();
    // see if we are breaking any constraints
    unique_ptr<ConsResult[]> status(new ConsResult[size]);
    for (size_t it=0;it<size;++it)
    {
        status[it]=(*start->constraints)[it]->getStatus(start);
        if (status[it]==C_Fail)
        {
            message="Constraint Fail";
            return false;
        }
    }       // we haven't "failed" constraints but do we still need action
        // and would that action put us over pile limit?
    bool supplyCap=false;
    if (start->getNormalPileCount()==start->getRequiredCount())
    {
        supplyCap=true; //. we don't return immediately because we might
                        // need to add non-supply cards to fix something
    }    
    for (size_t it=0;it<size;++it)
    {
        if (status[it]==C_ActionReq)
        {
            return (*start->constraints)[it]->act(start, result, message);
        }
    }
    if (supplyCap)
    {
        result=start;
        return true;
    }
    for (size_t it=0;it<size;++it)
    {
        if (status[it]==C_MorePossible)
        {   
            // should take action on this constraint
            break;
        }
    }
        // do we need to consider cost targets?
    if (start->needToCheckCostTargets())
    {
        const float haveCostPenalty=-3;
        
        const float threshold=0.5;
        const float tolerance=0.21; // 0.2 was showing non-determinism
        
        bool needTargetAction=false;
        const CostSet& costs=start->getCostSet();
        CostVotes votes(legalCosts);
        for (auto tar : start->getTargetSet())
        {
            needTargetAction=tar->addVotes(costs, votes) || needTargetAction;
        }
            // Now we need to take into account the costs where we already have a pile
        for (const Cost c : costs)
        {
            votes.addVote(c, haveCostPenalty);
        }
            // Two possibilities to consider here
            // A) there is an unmet target ... interate through all possibles
            //      to find a card that works
            // B) The flag is still set but all targets are minimally 
            //    satisfied. In which case, try the first card which matches
            //    if it works, fine. If not, stop
        CostSet maxCost;
        if (votes.getMaxWeighted(maxCost, threshold, tolerance))
        {
            PileIt begin, end;
            CostProperty* cp=new CostProperty(maxCost);
            if (this->getIterators(PropertyPtr(cp), begin, end))
            {
                do
                {
                    const Pile* next=*begin;
                    if (start->contains(next))
                    {
                        continue;
                    }
                    Selection* newSel=new Selection(*start);
                    if (!needTargetAction)
                    {
                        newSel->setNeedToCheck(false, string());  
                    }
                    // in case this gets overwritten in add
                    string blame=newSel->getTargetString();
                    if (!newSel->addPile(next))
                    {
                        if (!needTargetAction)
                        {           // We didn't need this card // we'll try later options
                            start->setNeedToCheck(false, string());  // *start* is not a mistake
                            break;                            // need to prevent _current_ selection from seeking more, not just the recursive one
                        }
                        else
                        {          // We needed this to work
                            return false;
                        }
                    }
                        // need to work out how to give more useful feedback
                    ostringstream oss;
                    oss << "<why?cost-target:";
                    oss << blame;
                    oss << '>';
                    newSel->tagPile(next, oss.str());
                    SelectionPtr sp(newSel);                   
                    if (buildSelection(sp, result, message))
                    {
                        return true;
                    }                       
                }
                while(++begin, begin!=end);
            }
            else    // couldn't find matching costs
            {       // need to check if we _needed_ it
                if (needTargetAction)
                {
                    return false;
                }
            }
        }
    }
    // If we get to this point, this selection (start)
    // can't be looking to costtargets for help so
    start->setNeedToCheck(false, string());
    
        // we don't have any constraints to guide us so add a general pile
        // Note: this method of preventing lower levels from considering a pile if
        // an upper level has already tried that pile should be ok _provided_ that
        // Some action by an intermediate level hasn't made a previously invalid card
        // valid.
    const Pile* gen=start->getGeneralPile();
    do
    {
        if (!gen)
        {
            return false;
        }
        if (start->contains(gen))
        {
            continue;
        }
        Selection* newSel=new Selection(*start);
        if (!newSel->addPile(gen))
        {
            return false;
        }
        newSel->tagPile(gen, "<why?general>");
        SelectionPtr sp(newSel);
        if (buildSelection(sp, result, message))
        {
            return true;
        }    
    } while (gen=start->getGeneralPile(), gen);
    return false;
}


CollectionStatus CardCollection::validateCollection(std::set<std::string>& warnings)
{
    for (auto it=piles.begin();it!=piles.end();++it)
    {
        for (auto card:(*it)->getCards())
        {
            for (const std::string& inter : card->getOtherInteractions())
            {
                if (inter.find("card(")==0)
                {
                    string target=inter.substr(5, inter.size()-6);
                    if (cardNames.find(target)==cardNames.end())
                    {
                        ostringstream oss;
                        oss << "Card " << card->getName() << " interacts with " << target << " but it is missing.";
                        warnings.insert(oss.str());
                    }
                }
                else if (inter.find("group(")==0)
                {
                    string target=inter.substr(6, inter.size()-7);
                    if (groupNames.find(target)==groupNames.end())
                    {
                        ostringstream oss;
                        oss << "Card " << card->getName() << " interacts with group " << target << " but it is missing.";
                        warnings.insert(oss.str());
                    }
                }
            }
        }
        
    }
    return warnings.empty()?Coll_OK:Coll_Warning;
}

void CardCollection::dump() const
{
    for (auto p : piles)
    {
        cout << p->getName() << endl;
    }
}


const Pile* CardCollection::getPileForCard(const std::string& s) const
{
    string pn;
    for (auto c : cards)
    {
        if (c->getName()==s)
        {
            pn=(c->getPileName().empty())?s:c->getPileName();
            break;
        }
    }
    if (pn.empty())
    {
        return 0;
    }
    for (auto it2 : piles)
    {
        if (it2->getName()==pn)
        {
            return it2;
        }
    }
    return 0;
}

}
