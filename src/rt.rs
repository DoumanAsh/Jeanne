use get_random_const::random;

#[random]
const RANDOM_SEED: u64 = 0;

pub fn init() {
    cute_dnd_dice::set_seed(RANDOM_SEED);
}
