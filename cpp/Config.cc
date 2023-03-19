#include <fstream>
#include <sstream>
#include <algorithm>
#include <iostream>
#include <list>
#include "Config.h"
#include "types.h"
#include "util.h"
#include "costtarget.h"
#include "property.h"
#include "actions.h"

using namespace decker;
using namespace std;



#define NAMECOL 0
#define PILECOL 1
#define SETCOL 2    
#define SUPPLYCOL 3
#define KINGDOMCOL 4
#define TYPECOL 5
#define COINCOST 6
#define SPENDPOW 7
#define DEBTCOST 8
#define POTIONCOST 9
#define POINTSCOL 10
#define KEYWORDSCOL 11
#define INTERACTKEY 12
#define INTERACTOTHER 13
#define ENDCOL (INTERACTOTHER+1)

// Would be nice if we had some way to extract this from the cards file
// Maybe we could convert the cost targets in a second pass.
#define MAXCOINCOST 11

namespace
{
    
// This should be a map including explanations
std::map<std::string, std::string> legalOptions={
    {"--seed", "Seed for random number generator. 0 will use a default value."},
    {"--badrand", "Use bad (but cross platform) random number generator"}, 
    {"--boxes", "Which boxes to include in the collection."}, 
    {"--groups", "Which groups to include in the collection."}, 
    {"--boxfile", "Filename listing boxes and which groups they contain"},
    {"--cardfile", "Filename listing all cards."},
    {"--help", "List command options."},
    {"--list", "Dump contents of collection and exit."},
    {"--landscape-count", "How many landscape cards to include (does not include artefacts etc)."},
    {"--why", "Explain why cards were added."},
    {"--no-validate", "Do not validate collection."},
    {"--exclude", "Do not allow any of these cards."},
    {"--include", "This card must be in the selection."},
    {"--info", "Show info about selected cards."},
    {"--no-attack-react", "Disable automatic adding reacts to attacks."},
    {"--no-anti-cursor", "Disable automatic adding of trash cards if cards give curses."},
    {"--max-cost-repeat", "Set the maximum number of times a cost can occur."},
    {"--min-type", "eg --min-type=Treasure:5 means that the selection will can contain at least 5 treasures."},
    {"--max-type", "eg --max-type=Treasure:5 means that the selection will can contain at most 5 treasures."},    
    {"--max-prefixes", "Most prefixes (groups and related groups) which can be included. Eg: Cornucopia would also allow Cornucopia-prizes."}
};    
    
    

inline bool strchr(const char* s, char c)
{
    while (*s!=c && *s!='\0') {s++;}    
    return *s==c;
}
    
inline bool boolValue(const std::string& s)
{
    return s.length()==1 && (s[0]=='Y' || s[0]=='y'); 
}

inline short shortValue(const std::string& s, int offset=0)
{
    char* err=0;    
    short res=strtol(s.c_str()+offset, &err, 10);
    if (s.empty() || res<0 || *err!='\0') {
        return -1;
    }
    return res;
}

inline unsigned unsignedValue(const std::string& s)
{
    char* err=0;    
    unsigned res=strtoul(s.c_str(), &err, 10);
    if (s.empty() || *err!='\0') {
        return 0;
    }
    return res;
}

    
bool parensOK(const std::vector<std::string>& l)
{
    for (const string& s : l)
    {
        if ((s.find('(')!=string::npos) && (s[s.length()-1]!=')'))
        {
            return false;
        }
    }
    return true;    
}    
   
   
namespace
{
CostTarget* decodeCost(const std::string& s)
{
    const short matchesRequired=6;
    const short unmetWeight=3;
    const short metWeight=1;    
    
    const short uptoMatches=3;
    if (!s.compare(0, 7, "cost<=+"))
    {
        int value=shortValue(s, 7);
        if (value==0)
        {
            return 0;
        }
        return new CostRelative(matchesRequired, unmetWeight, metWeight, value, false);
    }
    else if (!s.compare(0, 7, "cost<=-"))
    {
        int value=shortValue(s, 6);
        if (value==0)
        {
            return 0;
        }
        return new CostRelative(matchesRequired, unmetWeight, metWeight, value, false);
    }
    else if (!s.compare(0, 6, "cost<="))
    {
        int value=shortValue(s, 6);
        if (value==0)
        {
            return 0;
        }
        return new CostUpto(uptoMatches, unmetWeight, metWeight, value);
    }
    else if (!s.compare(0, 6, "cost=+"))
    {
        int value=shortValue(s, 6);
        if (value==0)
        {
            return 0;
        }
        return new CostRelative(matchesRequired, unmetWeight, metWeight, value, true);    
    }
    else if (!s.compare(0, 6, "cost=-"))
    {     
        int value=shortValue(s, 6);
        if (value==0)
        {
            return 0;
        }
        return new CostRelative(matchesRequired, unmetWeight, metWeight, -value, true);    
    }   
    else if (!s.compare(0, 6, "cost>="))
    {
        int value=shortValue(s, 6);
        if (value==0)
        {
            return 0;
        }
        CostSet cs;
        for (short v=value; v<=MAXCOINCOST; ++v)
        {
            cs.insert(Cost(v));
        }
        return new CostInSet(uptoMatches, unmetWeight, metWeight, cs);    
    }
    else if (!s.compare(0, 8, "cost_in("))
    {
        auto sep=s.find('.');
        if (sep==string::npos)
        {
            return 0;
        }
        short lower=shortValue(s.substr(8, sep-8));
        short upper=shortValue(s.substr(sep+1, s.size()-sep-2));
        if ((lower<=0) || (upper<=0))
        {
            return 0;
        }
        CostSet cs;
        for (;lower<=upper;++lower)
        {
            cs.insert(Cost(lower));
        }
        return new CostInSet(uptoMatches, unmetWeight, metWeight, cs);
    }
    return 0;
}
    
}

Card* makeCard(const std::vector<std::string>& fields)
{
    if (fields.size()<ENDCOL) 
    {
        return 0;
    }
    short coincost=shortValue(fields[COINCOST]);
    bool hascoin=(coincost!=-1);
    short potioncost=shortValue(fields[POTIONCOST]);
    bool haspotion=(potioncost!=-1);
    short debtcost=shortValue(fields[DEBTCOST]);
    bool hasdebt=(debtcost!=-1);
    Cost c(coincost, hascoin, potioncost, haspotion, debtcost, hasdebt);
    bool inSupply=boolValue(fields[SUPPLYCOL]);
    bool isKingdom=boolValue(fields[KINGDOMCOL]);
    vector<string> types;
    splitString(fields[TYPECOL], ';', types);
    vector<string> keywords;
    splitString(fields[KEYWORDSCOL], ';', keywords);
    vector<string> interactsKW;
    splitString(fields[INTERACTKEY], ';', interactsKW);
    vector<string> interactsOther;
    splitString(fields[INTERACTOTHER], ';', interactsOther);
        // want to impose some constraints
    if (!parensOK(interactsOther))
    {
        return 0;
    }
    vector<CostTarget*> targets;
        // Recognise cost constraints
    for (string& s : interactsOther)
    {
        if (!s.compare(0, 4, "cost"))
        {
            CostTarget* res=decodeCost(s);
            if (res!=0)
            {
                targets.push_back(res);
            }
            else
            {          
                return 0;   // parse error of cost interaction
            }               // need a way to get more detailed error info
        }
    }
    return new Card(fields[NAMECOL], fields[PILECOL], fields[SETCOL], inSupply, isKingdom, types, c, keywords, interactsKW, interactsOther, targets);
}



bool organiseArgs(int argc, char** argv, std::multimap<std::string, std::string>& res, std::string& err)
{
    for (int i=1;i<argc;++i)
    {
        if (!strchr(argv[i], '='))  // no =
        {
            if (legalOptions.find(argv[i])==legalOptions.end())
            {
                err=string("Unknown option ")+argv[i];
                return false;
            }            
            res.insert(make_pair(argv[i], "_"));
        }
        else
        {
            vector<string> splitEq;
            splitString(argv[i], '=', splitEq);
            vector<string> values;
            splitString(splitEq[1], ',', values);
            for (size_t j=0; j<values.size(); ++j)
            {
                res.insert(make_pair(splitEq[0], values[j]));
            }
            if (legalOptions.find(splitEq[0])==legalOptions.end())
            {
                err=string("Unknown option ")+splitEq[0];
                return false;
            }
        }
        
    }
    return true;
}
    
bool readBoxes(const std::string& fname, std::multimap<std::string, std::string>& res, std::string& err)
{
    ifstream ifs(fname);
    if (!ifs)
    {
        err="Can't open file";
        return false;
    }
    string s;
    int num=0;
    while (getline(ifs, s))
    {
        num++;
        if (s.empty() || s.find('#')==0)
        {
            continue;
        }
        vector<string> splitEq;
        splitString(s, '=', splitEq);
        if ((splitEq.size()!=2) || splitEq[1].empty() || splitEq[0].empty())
        {
            ostringstream oss;
            oss << "Can't parse line " << num;
            err=oss.str();
            return false;
        }
        vector<string> groups;
        splitString(splitEq[1], ';', groups);
        for (size_t i=0;i<groups.size();++i)
        {
            res.insert(make_pair(splitEq[0], groups[i]));
        }
    }
    return true;
}

// On false return contents of pilesForCollection can be ignored
//  the function will clean up any memory in that case.
// Note: it is not enough to just not add cards with names in excl
//  we need to remove the whole pile and some cards in it may 
//  have already been added
bool loadCards(const std::string& fname, list<Pile*>& pilesForCollection, 
               std::map<std::string, bool>& excl,  std::string& err)
{
    ifstream ifs(fname);
    if (!ifs) {
        err="Can't open file";
        return false;
    }
    string header, line;
    
    set<Pile*> removePiles;
        // On normal exits, this list is swapped into pilesForCollection
    list<Pile*> cardPiles;
    
    getline(ifs, header);
    map<string, Pile*> pMap;
    unsigned linecount=1;
    ostringstream oss;   
    std::vector<Card*> cards;    
    while (getline(ifs, line), ifs.good())
    {
        linecount++;
        if (line.empty() || line[0]==',')
        {
            continue;   // skip any rows that have no name
        }
        size_t start=0, end;        
        vector<string> fields;
        fields.reserve(15);      
        while (end=line.find(',', start), end!=string::npos) 
        {
            fields.push_back(line.substr(start, end-start));
            start=end+1;
        }
        Card* c=makeCard(fields);
        if (!c) {
            oss << "Error parsing card line " << linecount << endl;
        }
        else
        {
            Pile* p=0;
            const string& pileName=(c->getPileName().empty())?c->getName():c->getPileName();
            auto it=pMap.find(pileName);
            if (it==pMap.end())
            {
                p=new Pile(pileName);
                cardPiles.push_back(p);
                pMap.insert(make_pair(pileName, p));
            }
            else
            {
                p=it->second;
            }
            p->addCard(c);
            auto exIt=excl.find(c->getName());
            if (exIt!=excl.end())
            {
                removePiles.insert(p);
                exIt->second=true;
            }
        }
        // this is where we would handle comments but we aren't storing them 
    }
    for (auto p : removePiles)
    {
        cardPiles.remove(p);
        delete p;
    }
    if (oss.tellp()==0) // only check for unknown card names if no error
    {
        for (auto v : excl)
        {
            if (!v.second)
            {
                err=string("Unknown card ")+v.first;
                break;
            }
        }
    }
    else
    {
        err=oss.str();
    }
    if (!err.empty())
    {
        for (auto p : cardPiles)
        {
            delete p;
        }
        return false;
    }
    pilesForCollection.swap(cardPiles);    // keep hold of piles
    return true;
}



}   // end anonymous namespace

namespace decker
{

Config::Config():rand(0), piles(new PileSet), validate(true), listCollection(false),
    disableAntiCursors(false),disableAttackReact(false)
{
}

Config::~Config()
{
    if (piles)
    {
        for (auto p : *piles)
        {
            delete p;
        }
    }
    delete rand;
}

void showOptions()
{
    cout << "decker [options]\n";
    for (auto v : legalOptions)
    {
        cout << "  " << v.first << "  " << v.second << endl;
    }
    cout << endl;
    cout << "Arguments which take further input are of the form --opt=a,b,c\n";
}

bool Config::loadConfig(int argc, char** argv, Config& conf, std::string& err, const std::string& cardFile, const std::string& boxFile)
{
    err=string();
    PileSet* pSet=conf.piles.get();     // just so we don't keep calling .get()
    if (!organiseArgs(argc, argv, conf.args, err))
    {
        return false;
    }
    if (conf.args.find("--help")!=conf.args.end())
    {
        err.clear();
        showOptions();
        return false;
    }
        // build an exclusion list of cardnames to drop 
    auto ban=conf.args.equal_range("--exclude");
    std::map<std::string, bool> excludeNames;
    for (auto bc=ban.first;bc!=ban.second;++bc)
    {
        excludeNames.insert(make_pair(bc->second,false));
    }
    map<string, bool> requiredGroups;    
    list<Pile*> tempPiles;
    auto it=conf.args.find("--cardfile");
    string cardFilename=(it!=conf.args.end())?it->second:cardFile;
    if (cardFilename.empty())
    {
        err="No card file specified";
        return false;
    }
    else
    {
        if (!loadCards(cardFilename, tempPiles, excludeNames, err))
        {
            return false;
        }
    }
    auto p=conf.args.equal_range("--boxes");
    if (p.first!=p.second)
    {
        auto it=conf.args.find("--boxfile");
        string boxFilename=(it!=conf.args.end())?it->second:boxFile;
        std::multimap<std::string, std::string> boxToSet;
        if (it!=conf.args.end() )    // only try to load box file if boxes are specified
        {
            if (boxFilename.empty())
            {
                err="No box file specified.";
                return false;
            }
            else
            {
                if (!readBoxes(boxFilename, boxToSet, err))
                {
                    return false;
                }
            }
        }        
        if (boxToSet.size()==0)
        {
            err=string("--boxes specified but no boxes known (use --boxfile).");
            return false;
        }
        for (auto it=p.first;it!=p.second;++it)
        {
                // now map the box to groups
            auto q=boxToSet.equal_range(it->second);
            if (q.first==q.second)
            {
                err=string("Box ")+it->second+string(" not known in box file "+boxFile);
                return false;
            }
            for (auto it2=q.first;it2!=q.second;++it2)
            {
                requiredGroups.insert(make_pair(it2->second,false));
            }
        }
    }
    p=conf.args.equal_range("--groups");
    for (auto it=p.first;it!=p.second;++it)
    {
        requiredGroups.insert(make_pair(it->second, false));
    }
    if (requiredGroups.size()>0)
    {
        requiredGroups.insert(make_pair("base",false));    // avoid invalid games
        vector<Pile*> toDrop;
        toDrop.reserve(tempPiles.size());   // can't need more than this
        for (auto el : tempPiles)
        {
            auto it=requiredGroups.find(el->getCardGroup());
            if (it!=requiredGroups.end())
            {
                pSet->insert(el);
                it->second=true;
            }
            else
            {
                toDrop.push_back(el);
            }
        }
        // now we check to see if all groups are known
        for (auto x : requiredGroups)
        {
            if (!x.second)
            {
                if (!err.empty())
                {
                    err+='\n';
                }
                err+=string("Unknown group ")+x.first;
            }
        }
        for (auto drop : toDrop)
        {
            delete drop;
        }
    }
    else    
    {
        for (auto el : tempPiles)
        {
            pSet->insert(el);   // add all piles
        }
    }
    auto inc=conf.args.equal_range("--include");
    for (auto ic=inc.first;ic!=inc.second;++ic)
    {
        bool found=false;
        for (auto it=pSet->begin(); !found && it!=pSet->end(); ++it)
        {
            for (auto c : (*it)->getCards())
            {
                if (c->getName()==ic->second)
                {
                        // include the pile that has that card in it
                    conf.includes.insert(*it);
                    found=true;
                    break;
                }
            }
        }
        if (!found)
        {
            err=string("Can't find card ")+ic->second;
            return false;
        }
    }
    inc=conf.args.equal_range("--min-type");
    for (auto ic=inc.first;ic!=inc.second;++ic)
    {
        size_t sep=ic->second.find(':');
        if ((sep==string::npos) || (sep==0))
        {
            continue;
        }
        string typeName=ic->second.substr(0, sep);
        string val=ic->second.substr(sep+1);
        unsigned typeCount=unsignedValue(val);        
        conf.minTypes.insert(make_pair(typeName, typeCount));
    }
    inc=conf.args.equal_range("--max-type");
    for (auto ic=inc.first;ic!=inc.second;++ic)
    {
        size_t sep=ic->second.find(':');
        if ((sep==string::npos) || (sep==0))
        {
            continue;
        }
        string typeName=ic->second.substr(0, sep);
        string val=ic->second.substr(sep+1);
        unsigned typeCount=unsignedValue(val);        
        conf.maxTypes.insert(make_pair(typeName, typeCount));
    }
    bool useBadRand=false;
    if (conf.args.find("--badrand")!=conf.args.end())
    {
        useBadRand=true;
    }
    unsigned seed=0;
    auto sit=conf.args.find("--seed");
    bool choseSeed=false;
    if (sit!=conf.args.end())
    {
        seed=unsignedValue(sit->second);
        choseSeed=true;
    }
    sit=conf.args.find("--max-cost-repeat");
    if (sit!=conf.args.end())
    {
        conf.maxCostRepeat=unsignedValue(sit->second);
    }
    else
    {
        conf.maxCostRepeat=0;
    }
    if (conf.args.find("--no-validate")!=conf.args.end())
    {
        conf.validate=false;
    }
    if (conf.args.find("--list")!=conf.args.end())
    {
        conf.listCollection=true;
    }
    
        // now we set up randomiser
        // The cap here is arbitrary (want it to be at least as big 
        // as 3 x pile size for shuffling
    conf.rand=RandStream::getRandStream(seed, 10*conf.piles->size(), useBadRand);   
    if (!choseSeed)
    {
        ostringstream oss;
        oss << conf.rand->initSeed();
        conf.args.insert(make_pair("--seed",oss.str()));
    }

        // Now we limit the groups we can draw from.
        // We'll do it here before anything gets into the 
        // CardCollection object.
        // We need to take into account which prefixes any 
        // --include cards force in (and also if that exceeds
        // the limit
    {
        auto it=conf.args.find("--max-prefixes");
        if (it!=conf.args.end())
        {
            unsigned suggestedMax=unsignedValue(it->second);
            if (suggestedMax>0) // ==0 is equivalent to no limit
            {
                suggestedMax++;             // because we must include base
                set<string> chosenPrefixes;
                chosenPrefixes.insert("base");                
                for (auto p : conf.includes)
                {
                    chosenPrefixes.insert(groupNamePrefix(p->getCardGroup()));
                }
                if (chosenPrefixes.size()>suggestedMax)
                {
                        // need to -1 from both numbers because of hidden "base" group
                    ostringstream oss;
                    oss << "Requested at most " << (suggestedMax-1) << " big groups, but included cards are drawn from ";
                    oss << (chosenPrefixes.size()-1) << ".";
                    err=oss.str();
                    return false;
                }
                std::set<std::string> groupPrefixes;
                    // now we need to know all available big groups
                for (auto p : *conf.piles)
                {
                    groupPrefixes.insert(groupNamePrefix(p->getCardGroup()));
                }
                
                // now shuffle the prefixes
                vector<std::string> shufflePrefixes;
                for (auto s : groupPrefixes)
                {
                    shufflePrefixes.push_back(s);
                }
                size_t nSets=shufflePrefixes.size();
                for (int i=0; i<3; ++i)
                {
                    for (size_t i=0; i<nSets; ++i)
                    {
                        size_t pos=conf.rand->get()%nSets;
                        if (i!=pos)
                        {
                            shufflePrefixes[i].swap(shufflePrefixes[pos]);
                        }
                    }
                }
                for (size_t i=0; (i<nSets) && (chosenPrefixes.size()<suggestedMax) ; ++i)
                {
                    chosenPrefixes.insert(shufflePrefixes[i]);
                }
                
                    // now filter any piles which don't belong to those groups
                PileSet newPiles;
                for (auto p : *conf.piles)
                {
                    if (chosenPrefixes.find(groupNamePrefix(p->getCardGroup()))!=chosenPrefixes.end())
                    {
                        newPiles.insert(p);
                    }
                    else
                    {
                        delete p;
                    }
                }
                newPiles.swap(*conf.piles);
            }
        }
    }
    
        // Now let's work out how many optional extras we need
    unsigned optExtra;
    sit=conf.args.find("--landscape-count");
    if (sit!=conf.args.end())
    {
        optExtra=unsignedValue(sit->second);
    }
    else    // didn't specify how many they wanted so random
    {
        unsigned x=conf.rand->get()%7;
        optExtra=(x<3)?x:0;
    }
    conf.optionalExtras=optExtra;
    sit=conf.args.find("--why");
    if (sit!=conf.args.end())
    {
        conf.why=true;
    }
    else
    {
        conf.why=false;
    }
    sit=conf.args.find("--info");
    if (sit!=conf.args.end())
    {
        conf.moreInfo=true;
    }
    else
    {
        conf.moreInfo=false;
    }    
    conf.options=conf.getString();
    conf.disableAntiCursors=conf.args.find("--no-anti-cursor")!=conf.args.end();
    conf.disableAttackReact=conf.args.find("--no-attack-react")!=conf.args.end();
    if (!err.empty())
    {
        return false;
    }
    return true;
}


std::string Config::getString() const
{
    ostringstream oss;
    std::string prev;
    for (auto m : args)
    {
        if (m.first!=prev)
        {
            if (!prev.empty())
            {
                oss << ' ';
            }
            prev=m.first;
            oss << m.first << '=' << m.second;
        }
        else
        {
            oss << ',' << m.second;
        }
    }
    return oss.str();
}


std::vector<Constraint*> Config::buildConstraints(const CardCollection& coll, std::string& err)
{
    PropertyPtr failProp(new FailProperty());
    std::vector<Constraint*> cons;
    cons.push_back(baneConstraint(coll));  
    cons.push_back(prospConstraint(coll));

        // let's abuse the Constraint machinery
        // The missing poition check will be zero both when there are no Potion costs
        //  and when there are Potion costs, but Potion cards are in the deck.
        // ie transition= 0 -> 1 -> 0
        //  when it hits 1 we want to add the potion which will drop it back
        // So, we want the main property to always fail to force the action to run.
        //  Since the precondition will remain 0 from then on, so the always failing main
        //   doesn't matter.
    Constraint* c=new Constraint("AddPotion", PropertyPtr(new MissingPotionProperty()), failProp, new AddGroup(&coll, "Alchemy-base"), 1, decker::MANY, decker::MANY, decker::MANY);
    cons.push_back(c);
    
    c=new Constraint("AddProsperityCards", PropertyPtr(new NeedProsperity(coll, rand->get()%10)), failProp, new AddProsperity(&coll), 1, decker::MANY, decker::MANY, decker::MANY);
    cons.push_back(c);
    
    c=new Constraint("AddInteractingGroup", PropertyPtr(new MissingInteractingCardGroupProperty()), failProp, new AddMissingDependencyGroup(&coll), 1, decker::MANY, decker::MANY, decker::MANY);
    cons.push_back(c);

    c=new Constraint("AddInteractingCard", PropertyPtr(new MissingInteractingCardProperty()), failProp, new AddMissingDependency(&coll), 1, decker::MANY, decker::MANY, decker::MANY);
    cons.push_back(c);

    c=new Constraint("AddHexForDoom", PropertyPtr(new MissingGroupForKeywordProperty("Doom", "Nocturne-Hexes")), failProp, new AddGroup(&coll, "Nocturne-Hexes"), 1, decker::MANY, decker::MANY, decker::MANY);
    cons.push_back(c);

    c=new Constraint("AddBoonForFate", PropertyPtr(new MissingGroupForKeywordProperty("Fate", "Nocturne-Boons")), failProp, new AddGroup(&coll, "Nocturne-Boons"), 1, decker::MANY, decker::MANY, decker::MANY);
    cons.push_back(c);
    
    if (!disableAntiCursors)
    {
        Constraint* c=curserConstraint(coll,1 );
        if (c) 
        {
            cons.push_back(c);
        }
    }
    if (!disableAttackReact)
    {
        Constraint* c=attackReactConstraint(coll, 2);
        if (c) 
        {
            cons.push_back(c);
        }    
    }    
    if (maxCostRepeat>0)
    {
        c=new Constraint("RepeatedCosts", PropertyPtr(new RepeatedCostProperty(maxCostRepeat)), nullptr, 0, 1);
        cons.push_back(c);        
    }
    for (auto p : minTypes)
    {
            // Note there are two TypeProperties in use here
            // When getting iterators for possible piles to add we only want to
            // consider piles which are in the supply and kingdom cards 
            //  (eg we don't want Platinum being picked as a possibility)
            // However when searching to see if we have too many, we want 
            // to include all cards already selected.
        const string& typeName=p.first;
        unsigned typeCount=p.second;

        PileIt tBegin, tEnd;
        Property* searcher=new TypeProperty(typeName, true);
        if (!coll.getIterators(PropertyPtr(searcher), tBegin, tEnd))
        {
            err=string("No matches found for type ")+typeName;
            return cons;
        }
            // we count all treasures, but we don't want non-kingdom cards to be selected to satisfy a constraint
        ostringstream oss;
        oss << "At least " << typeCount << " " << typeName << "s";
        c=new Constraint(oss.str(), PropertyPtr(new TypeProperty(typeName, false)), new FindPile(&coll, tBegin, tEnd), typeCount, decker::MANY);
        cons.push_back(c);       
    }
    for (auto p : maxTypes)
    {
        const string& typeName=p.first;
        unsigned typeCount=p.second;

        PileIt tBegin, tEnd;
        Property* searcher=new TypeProperty(typeName, false);
        if (coll.getIterators(PropertyPtr(searcher), tBegin, tEnd))          // we don't 
        {       // care if there are no cards for a max constraint
            ostringstream oss;
            oss << "At most " << typeCount << " " << typeName << "s";
            c=new Constraint(oss.str(), PropertyPtr(new TypeProperty(typeName, false)), nullptr, 0, typeCount);
            cons.push_back(c);
        }
    }    
        // now we need to find any keyword interactions
    set<string> interactsKw;
    for (auto p : coll.getPiles())
    {
        for (auto s : p->getKWInteractions())
        {
            interactsKw.insert(s);
        }
    }
    for (auto s : interactsKw)
    {
        if (s=="gain")
        {
            Property* prop=new EitherProperty(new KeywordProperty("gain", true), new KeywordProperty("+buy", true));
            string name="Provide interacted keyword (gain/+buy)";
            PileIt begin, end;
            if (coll.getIterators(PropertyPtr(prop), begin, end))
            {
                auto c=new Constraint(name, PropertyPtr(new HangingInteractsWith("gain", "gain", "+buy")), failProp, new FindPile(&coll, begin, end), 1, decker::MANY, decker::MANY, decker::MANY);
                cons.push_back(c);
            }
        }
        else if (s=="trash")
        {
            Property* prop=new EitherProperty(new KeywordProperty("trash_any", true), new KeywordProperty("trash_limited", true));
            string name="Provide interacted keyword (trash_any/trash_limited)";
            PileIt begin, end;
            if (coll.getIterators(PropertyPtr(prop), begin, end))
            {
                auto c=new Constraint(name, PropertyPtr(new HangingInteractsWith("trash", "trash_limited", "trash_any")), failProp, new FindPile(&coll, begin, end), 1, decker::MANY, decker::MANY, decker::MANY);
                cons.push_back(c);
            }
        }
        else
        {
            Property* prop=new KeywordProperty(s, true);
            string name=string("Provide interacted keyword ")+s;
            PileIt begin, end;
            if (coll.getIterators(PropertyPtr(prop), begin, end))
            {
                auto c=new Constraint(name, PropertyPtr(new HangingInteractsWith(s, s)), failProp, new FindPile(&coll, begin, end), 1, decker::MANY, decker::MANY, decker::MANY);
                cons.push_back(c);
            }
        }
        
    }
    return cons;
    
}

}   // end decker namespace

