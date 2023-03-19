#include "types.h"

namespace decker
{

class FindBane : public ConstraintAction
{
public:
    FindBane(const CardCollection* col, PileIt& beginIT, PileIt& endIT);
    ~FindBane(){}
    bool apply(const std::string& why, const SelectionPtr& start, SelectionPtr& result, std::string& message) override;
private:
    PileIt begin, end;
};

class AddGroup : public ConstraintAction
{
public:
    AddGroup(const CardCollection* collection, const std::string& group);
    ~AddGroup(){};
    bool apply(const std::string& why, const SelectionPtr& start, SelectionPtr& result, std::string& message) override;
private:
    std::string groupName; 
};

class FindPile : public ConstraintAction
{
public:
    FindPile(const CardCollection* col, PileIt& beginIT, PileIt& endIT);
    ~FindPile(){}
    bool apply(const std::string& why, const SelectionPtr& start, SelectionPtr& result, std::string& message) override;
private:
    PileIt begin, end;    
};

class AddMissingDependency : public ConstraintAction
{
public:
    AddMissingDependency(const CardCollection* col);
    ~AddMissingDependency(){}
    bool apply(const std::string& why, const SelectionPtr& start, SelectionPtr& result, std::string& message) override;
};

class AddMissingDependencyGroup : public ConstraintAction
{
public:
    AddMissingDependencyGroup(const CardCollection* col);
    ~AddMissingDependencyGroup(){}
    bool apply(const std::string& why, const SelectionPtr& start, SelectionPtr& result, std::string& message) override;
};

class AddProsperity : public ConstraintAction
{
public:    
    AddProsperity(const CardCollection* col);
    ~AddProsperity(){}
    bool apply(const std::string& why, const SelectionPtr& start, SelectionPtr& result, std::string& message) override;
};


}
