#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod x86_64;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use x86_64::rtm;
