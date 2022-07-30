use crate::prelude::*;
use rand as rnd;


pub async fn clock_cycles(signal: SimObject, n_cycles: u32) -> RstbResult {
    for _ in 0..n_cycles {
        signal.rising_edge().await;
    }
    Ok(Val::None)
}

#[inline]
pub fn rand() -> f32 {
    rnd::random::<f32>()
}

#[inline]
pub fn rand_int(ceil: u32) -> u32 {
    rnd::random::<u32>() % ceil
}