#ifndef RANDSTREAM_H
#define RANDSTREAM_H

class RandStream
{
public:
    static RandStream* getRandStream(unsigned s, unsigned cap, bool useBadRandom=true);  
    virtual ~RandStream(){};
    virtual unsigned get()=0;
    RandStream(const RandStream&)=delete;
    virtual unsigned initSeed()=0;
protected:
    RandStream(){};
};



#endif
