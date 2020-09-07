#[cfg(target_arch = "x86")]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
#[inline(always)]
pub fn fract(value: f32) -> f32 {
    use crate::fontdue::platform::trunc;
    value - trunc(value)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline(always)]
pub fn fract(value: f32) -> f32 {
    unsafe {
        let packed = _mm_set_ss(value);
        _mm_cvtss_f32(_mm_sub_ps(
            packed,
            _mm_cvtepi32_ps(_mm_cvttps_epi32(packed)),
        ))
    }
}
