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

pub fn create_psk_seed() -> [u8; 32] {
    rand::thread_rng().gen::<[u8; 32]>()
}