use rand::Rng;

const ALPH9: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ9";

pub fn create_seed() -> String {
    let seed: &str = &(0..81)
        .map(|_| {
            ALPH9
                .chars()
                .nth(rand::thread_rng().gen_range(0, 27))
                .unwrap()
        })
        .collect::<String>();
    String::from(seed)
}
