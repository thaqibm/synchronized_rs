pub mod rtm {
    use std::arch::asm;
    pub const _XBEGIN_STARTED: u32 = !0;
    pub const _XABORT_EXPLICIT: u32 = 1 << 0;
    pub const _XABORT_RETRY: u32 = 1 << 1;
    pub const _XABORT_CONFLICT: u32 = 1 << 2;
    pub const _XABORT_CAPACITY: u32 = 1 << 3;
    pub const _XABORT_DEBUG: u32 = 1 << 4;
    pub const _XABORT_NESTED: u32 = 1 << 5;

    #[inline(always)]
    pub unsafe fn _xtest() -> bool
    {
        let result: u32;
        asm!(
            "xtest",
            "setne al",
            "movsx {0:e}, al", out(reg) result
        );
        result == 1
    }


    #[allow(non_snake_case)]
    #[inline(always)]
    pub fn _XABORT_CODE(_xbegin_return_code: u32) -> u8
    {
        (_xbegin_return_code >> 24) as u8
    }


    #[inline(always)]
    pub unsafe fn _xabort<const N: u8>() -> ! {
        macro_rules! xabort_code {
            ( $var:expr ) => ( stringify!(xabort $var) );
        }
        asm!(xabort_code!(N));
        std::hint::unreachable_unchecked()
    }

    #[inline(always)]
    pub unsafe fn _xbegin() -> u32 {
        let status: u32;
        asm!(
            "mov eax, 0xFFFFFFFF",
            "xbegin 2f",
            "2:",
            "mov {0:e}, eax",
            out(reg) status,
        );
        status
    }

    #[inline(always)]
    pub unsafe fn _xend() -> () {
        asm!("xend")
    }

    pub fn htm_supported_runtime() -> bool {
        std::arch::is_x86_feature_detected!("rtm")
    }

    #[test]
    fn rtmtest() {
        fn runrtm(){
            let mut res = false;
            if _xbegin() == _XBEGIN_STARTED {
                res = _xtest();
                xend();
                assert!(res);
            }
            else {
                assert!(!res);
            }
        }
        runrtm();
        runrtm();
        runrtm();
        runrtm();
    }
}
