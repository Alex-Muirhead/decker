#include <map>
#include <set>
#include <string>
#include <map>
#include "rand.h"
#include "types.h"
namespace decker
{

    // Read commandline args
    // was going to have this as just a query store but some 
    // config options are interconnected eg random needs to know how many piles you have
class Config
{
public:
    Config();
    ~Config();
    
    static bool loadConfig(int argc, char** argv, Config& conf, std::string& err, const std::string& cardFile, const std::string& boxFile);

    std::string getString() const;
    
    RandStream* rand;
    std::unique_ptr<PileSet> piles;
    unsigned optionalExtras;    // How many landscape cards to use
    bool why;
    bool moreInfo;
    std::string options;
    bool validate;
    bool listCollection;
    PileSet includes;
    bool disableAntiCursors;
    bool disableAttackReact;
    unsigned maxCostRepeat;
    std::map<std::string, unsigned> minTypes;   // minimum of card types
    std::map<std::string, unsigned> maxTypes;   // maximum of card types
    std::multimap<std::string, std::string> args;

    std::vector<Constraint*> buildConstraints(const CardCollection& coll, std::string& err);
};


}
