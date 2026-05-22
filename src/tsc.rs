#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__cpuid, __rdtscp, _rdtsc};

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn cycle_start() -> u64 {
    unsafe {
        __cpuid(0);
        _rdtsc() as u64
    }
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn cycle_end() -> u64 {
    unsafe {
        let mut _aux: u32 = 0;
        let tsc = __rdtscp(&mut _aux);
        __cpuid(0);
        tsc
    }
}

#[cfg(target_arch = "aarch64")]
#[inline]
pub fn cycle_start() -> u64 {
    let val: u64;
    unsafe {
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) val);
    }
    val
}
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn cycle_end() -> u64 {
    cycle_start()
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub fn cycle_start() -> u64 {
    use std::time::Instant;
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let start = START.get_or_init(Instant::now);
    start.elapsed().as_nanos() as u64
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub fn cycle_end() -> u64 {
    cycle_start()
}
