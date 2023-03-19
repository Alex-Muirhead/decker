#include <ctime>
#include <cstdlib>
#include "rand.h"


// very poor randomiser
// using this so I can have a consistent (not good) random stream on any platform
class BadRand : public RandStream
{
public:
    BadRand(unsigned s, unsigned cap);
    unsigned get() override;
    unsigned initSeed() override
    {
        return init;
    }
private:
    unsigned seed;
    unsigned cap;
    unsigned step;
    unsigned init;
};

class CRand : public RandStream
{
public:
    CRand(unsigned s)
    {
        init=s;
        srand(s);
    };
    unsigned get() override
    {
        return rand();
    }
    unsigned initSeed() override
    {
        return init;
    }
private:
    unsigned init;    
};

RandStream* RandStream::getRandStream(unsigned s, unsigned cap, bool useBadRandom)
{
    if (useBadRandom)
    {
        return new BadRand(s, cap);
    }
    else    // use a better random
    {
        if (s==0)
        {
            s=time(0);
        }
        return new CRand(s);
    }
}



BadRand::BadRand(unsigned s, unsigned bound):seed(s), cap(bound), init(s)
{
    // now find a prime a bit above half the cap
    // 2*p > cap so p and cap must be co-prime
    unsigned f=bound/2+1;
    for (;f<cap;++f)
    {
        unsigned int i=2;
        for (; i<f; ++i)
        {
            if (f%i==0)
            {
                break;
            }
        }
        if (i==f) // f is prime
        {
            step=f;
            return;
        }
    }
    if (f==cap) 
    {
        step=1;    // this is truely awful
    }
}

// This is not a good function
// It will do for now though
// Why aren't I using built in versions which are better?
//    Because I want to have a function which works in multiple languages
unsigned BadRand::get()
{
    return seed=((seed+step)%cap);
}

