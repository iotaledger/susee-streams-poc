use rand::Rng;

pub fn create_psk_seed() -> [u8; 32] {
    rand::thread_rng().gen::<[u8; 32]>()
}