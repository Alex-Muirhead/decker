// To do random stream, I need traits
// This should go in a namespace eventually
pub trait RandStream {
    fn get(&mut self) -> u64;
    fn init_seed(&self) -> u64;
}

// This was hidden as an implementation detail in c++
// Can I do something like that here?
struct BadRand {
    seed: u64,
    cap: u64,
    step: u64,
    init: u64,
}

fn make_bad_rand(s: u64, bound: u64) -> BadRand {
    let cap = bound;
    let mut setstep: u64 = 1; // Not convinced this init is necessary
    let mut f = bound / 2 + 1;
    while f < cap {
        let mut i: u64 = 2;
        while i < f {
            if f % i == 0 {
                break;
            };
            i += 1;
        }
        if i == f {
            setstep = f;
            break;
        }
        f += 1;
    }
    if f == cap {
        setstep = 1;
    }
    BadRand {
        seed: s,
        cap: bound,
        init: s,
        step: setstep,
    }
}

impl RandStream for BadRand {
    fn get(&mut self) -> u64 {
        if self.cap == 0 {
            return 0;
        }
        let newseed = (self.seed + self.step) % self.cap;
        self.seed = newseed;
        newseed
    }
    fn init_seed(&self) -> u64 {
        self.init
    }
}

pub fn get_rand_stream(s: u64, cap: u64, _use_bad_random: bool) -> impl RandStream {
    // eventually want to make this conditional on use_bad_random
    make_bad_rand(s, cap)
}
