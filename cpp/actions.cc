#include "actions.h"
#include "property.h"
namespace decker
{

FindBane::FindBane(const CardCollection* col, PileIt& beginIT, PileIt& endIT) : ConstraintAction(col), begin(beginIT), end(endIT){}

bool FindBane::apply(const std::string& why, const SelectionPtr& start, SelectionPtr& result, std::string& message)
{
    for (auto it=begin; it!=end; ++it)
    {
        if (!start->contains(*it))
        {
            Selection* newSel=new Selection(*start);
            newSel->increaseRequiredPiles();
            bool res=newSel->addPile(*it);
            if (res)
            {
                newSel->tagPile(*it, "Bane");
                newSel->tagPile(*it, std::string("<why:")+why+'>');
                newSel->addNote("hasBane");
                SelectionPtr n(newSel);
                if (collection->buildSelection(n, result, message))
                {
                    return true;
                }
            }
            else
            {
                delete newSel;
            }
        }
    }
    return false;
}


AddGroup::AddGroup(const CardCollection* collection, const std::string& group):ConstraintAction(collection), groupName(group){}

bool AddGroup::apply(const std::string& why, const SelectionPtr& start, SelectionPtr& result,
    std::string& message)
{
    Selection* newSel=new Selection(*start);
    PileIt begin, end;
        // ownership of Property is an issue
    Property* p=new CardGroupProperty(groupName);
    if (!collection->getIterators(PropertyPtr(p), begin, end))
    {
        message=std::string("Tried to add group (")+groupName+") but no cards belonging to it found in the collection.";
        return false;
    }
    for (;begin!=end;++begin)
    {
        if (newSel->addPile(*begin))   // Not catching individual fails
        {
            newSel->tagPile(*begin, std::string("<why:")+why+'>');
        }
    }                           // maybe some got added some other way?
    newSel->addNote(std::string("added"+groupName));
    return collection->buildSelection(SelectionPtr(newSel), result, message);
}

FindPile::FindPile(const CardCollection* col, PileIt& beginIT, PileIt& endIT):ConstraintAction(col), begin(beginIT), end(endIT)
{}

bool FindPile::apply(const std::string& why, const SelectionPtr& start, SelectionPtr& result, std::string& message)
{
    for (auto it=begin; it!=end; ++it)
    {
        if (!start->contains(*it))
        {
            Selection* newSel=new Selection(*start);
            bool res=newSel->addPile(*it);
            if (res)
            {
                newSel->tagPile(*it, std::string("<why?")+why+'>');
                SelectionPtr n(newSel);
                if (collection->buildSelection(n, result, message))
                {
                    return true;
                }
            }
            else
            {
                delete newSel;
            }
        }
    }
    return false;    
}

AddMissingDependency::AddMissingDependency(const CardCollection* col):ConstraintAction(col)
{
}

bool AddMissingDependency::apply(const std::string& why, const SelectionPtr& start, SelectionPtr& result, std::string& message)
{
    std::map<std::string, std::string> need; // card needed and by which pile 
    for (auto p : start->getPiles())    // This code is duplicated in Missing...Property
    {                                   // that's not a good thing
        for (auto it : p->getOtherInteractions())
        {
            if (it.find("card(")==0)
            {
                std::string needName=it.substr(5, it.size()-6);
                need.insert(make_pair(needName, p->getName()));
            }
        }
    }
    if (need.empty())
    {
        message="AddMissingDependency applied but no cards have card() OtherInteractions";
        return false;
    }  
    for (auto it : need)
    {
        const Pile* p=collection->getPileForCard(it.first);
        if (!p)
        {
            message=std::string("Unable to find a pile containing ")+it.first;
            return false;
        }
        if (!start->contains(p))
        {
            Selection* newSel=new Selection(*start);
            bool res=newSel->addPile(p);          
            if (res)
            {
                newSel->tagPile(p, std::string("<why?card:")+it.second+" interacts with it>");
                SelectionPtr n(newSel);
                    // we aren;t trying to add more than one pile because any successive
                    // missing cards will get picked up in later actions
                return collection->buildSelection(n, result, message);
            }
        }
    }
    message="AddMissingDependency applied but nothing seemed missing";
    return false;
}

AddMissingDependencyGroup::AddMissingDependencyGroup(const CardCollection* col):ConstraintAction(col)
{
}

bool AddMissingDependencyGroup::apply(const std::string& why, const SelectionPtr& start, SelectionPtr& result, std::string& message)
{
    SelectionPtr newSel;
    for (auto p : start->getPiles())
    {
        for (auto it : p->getOtherInteractions())
        {
            if (it.find("group(")==0)
            {
                std::string needName=it.substr(6, it.size()-7);
                if (!start->hasNote("added"+needName))
                {
                        // need to add all piles from that group
                    CardGroupProperty* ps=new CardGroupProperty(needName);
                    PileIt begin, end;
                    if (!start->getCollection()->getIterators(PropertyPtr(ps), begin, end))
                    {
                        message=std::string("Unable to find required group named ")+needName;
                        return false;
                    }
                    if (!newSel)
                    {
                        newSel=SelectionPtr(new Selection(*start));
                    }
                    for (;begin!=end;++begin)
                    {
                        if (newSel->addPile(*begin))
                        {
                            newSel->tagPile(*begin, std::string("<why?cards:")+p->getName()+" needs it>");
                        }
                        else
                        {
                            message=std::string("Unable to add card "+(*begin)->getName());
                            return false;
                        }
                    }
                    newSel->addNote(std::string("added"+needName));
                }
            }
        }
    }
    if (!newSel)
    {
        message="AddMissingDependencyGroup called buit nothing seems to be missing";
        return false;
    }
    return collection->buildSelection(newSel, result, message);
}


AddProsperity::AddProsperity(const CardCollection* col):ConstraintAction(col){}
    
bool AddProsperity::apply(const std::string& why, const SelectionPtr& start, SelectionPtr& result, std::string& message)
{
    const CardCollection* col = start->getCollection();
    auto plat = col->getPileForCard("Platinum");
    auto colony= col->getPileForCard("Colony");
    if (!plat || !colony) 
    {
        message="Can't find prosperity base cards";
        return false;
    }
    SelectionPtr res(start);
    if (!res->contains(plat))
    {
        if (!res->addPile(plat))
        {
            message="Error adding Platinum";
            return false;
        }
        res->tagPile(plat, why);
    }
    if (!res->contains(colony))
    {
        if (!res->addPile(colony))
        {
            message="Error adding Colony";
            return false;
        }
        res->tagPile(colony, why);        
    }
    return collection->buildSelection(res, result, message);
}

}
