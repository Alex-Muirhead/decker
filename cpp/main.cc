#include <iostream>
#include <cstdlib>  // strtol
#include "types.h"
#include "property.h"
#include "Config.h"

using namespace std;
using namespace decker;

int main(int argc, char** argv) 
{
    Config c;
    string err;
    if (!Config::loadConfig(argc, argv, c, err, "cards.dat", ""))
    {
        cerr << err << endl;
        return 1;
    }
    std::vector<std::string> empty;
    CardCollection col(c.piles);
    std::set<std::string> warnings;
    if (c.validate)
    {
        if (col.validateCollection(warnings)!=Coll_OK)
        {
            cerr << "Error validating collection:\n";
            for (auto s : warnings)
            {
                cerr << s << endl;
            }
            return 3;
        }
    }
    if (c.listCollection)
    {
        col.dump();
        return 0;
    }
    col.shuffle(*c.rand);
    std::string message;
    std::vector<Constraint*> cons=c.buildConstraints(col, message);
    if (!message.empty())
    {
        cerr << message << endl;
        return 4;
    }
    SelectionPtr sel=col.generateSelection(10, c.optionalExtras, c.includes, message, cons, c.rand);
    if (sel.get()==0)
    {
        cerr << "Error: empty selection\n";
        if (!message.empty())
        {
            cerr << "Possible explanation: " << message << std::endl;
        }
        return 2;
    }  
    cout << "Options: " << c.options << endl;
    sel->dump(c.why, c.moreInfo);
    return 0;
}
