use rand::Rng;
use rand_core::{impls, Error, RngCore};

// This was hidden as an implementation detail in c++
// Can I do something like that here?
struct BadRand {
    bound: u64,
    seed: u64,
    step: u64,
}

impl BadRand {
    fn new(seed: u64, bound: u64) -> Self {
        // Step through values till a prime is found
        let step = (bound / 2 + 1..bound)
            .into_iter()
            .find(|&n| is_prime(n))
            .unwrap_or(1);

        BadRand { seed, bound, step }
    }
}

impl RngCore for BadRand {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        if self.bound == 0 {
            return 0;
        }
        self.seed = (self.seed + self.step) % self.bound;
        self.seed
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        impls::fill_bytes_via_next(self, dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        Ok(self.fill_bytes(dest))
    }
}

fn is_prime(num: u64) -> bool {
    // Deal with special case
    if num == 1 {
        return false;
    }
    let root = (num as f64).sqrt() as u64;
    (2..=root).into_iter().find(|i| num % i == 0).is_none()
}

pub fn get_rand_stream(seed: u64, bound: u64, _use_bad_random: bool) -> impl Rng {
    // eventually want to make this conditional on use_bad_random
    BadRand::new(seed, bound)
}

#[cfg(test)]
mod test_primes {
    use super::*;

    #[test]
    fn test_first_numbers() {
        assert!(is_prime(1) == false);
        assert!(is_prime(2) == true);
        assert!(is_prime(3) == true);
        assert!(is_prime(4) == false);
    }

    #[test]
    fn test_larger_numbers() {
        assert!(is_prime(997) == true);
        assert!(is_prime(524) == false);
    }
}
