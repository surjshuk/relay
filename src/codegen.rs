use rand::{Rng, thread_rng};
use crate::state::ServerState;

const ALPHABET : &[u8]= b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

pub const CODE_LEN: usize = 8;

pub fn make_code(len: usize) -> String {
    let mut rng = thread_rng();

    let mut buf = vec![0u8; len];

    for b in &mut buf {
        let i = rng.gen_range(0..ALPHABET.len());

        *b = ALPHABET[i];
    }

    String::from_utf8(buf).expect("ascii only")
}

pub fn unique_code(state: &ServerState, len: usize) -> String {
    for _ in 0..16 {
        let code = make_code(len);

        if !state.rooms.contains_key(&code) {
            return code;
        }
    }

    make_code(len + 1)
}