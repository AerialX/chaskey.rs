#![no_std]

use core::mem::MaybeUninit;

pub mod sys {
    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct ChaskeyContext {
        tag: [u32; 4], // union [u8;16]
        k1: [u32; 4],
        k2: [u32; 4],
        len: usize,
        m: [u32; 4], // union [u8; 16]
    }

    extern "C" {
        pub fn chaskey_subkeys(k1: *mut [u32; 4], k2: *mut [u32; 4], k: &[u32; 4]);
        pub fn chaskey_init(context: *mut ChaskeyContext, k: &[u32; 4], k1: &[u32; 4], k2: &[u32; 4]);
        pub fn chaskey_process(context: &mut ChaskeyContext, m: *const u8, len: usize);
        pub fn chaskey_finish(context: &mut ChaskeyContext);
        pub fn chaskey_tag(context: &ChaskeyContext) -> &[u8; 16];
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Context {
    context: sys::ChaskeyContext,
}

impl Context {
    #[inline]
    pub fn new(k: &[u32; 4], k1: &[u32; 4], k2: &[u32; 4]) -> Self {
        let mut context = MaybeUninit::uninit();
        unsafe {
            sys::chaskey_init(context.as_mut_ptr() as *mut _, k, k1, k2);
            context.assume_init()
        }
    }

    #[inline]
    pub fn from_subkeys(k: &[u32; 4], subkeys: &Subkeys) -> Self {
        Self::new(k, &subkeys.k1, &subkeys.k2)
    }

    pub fn from_key(k: &[u32; 4]) -> Self {
        Self::from_subkeys(k, &Subkeys::new(k))
    }

    #[inline]
    pub fn process(&mut self, m: &[u8]) {
        unsafe {
            sys::chaskey_process(&mut self.context, m.as_ptr(), m.len())
        }
    }

    #[inline]
    pub fn commit(&mut self) {
        unsafe {
            sys::chaskey_finish(&mut self.context)
        }
    }

    #[inline]
    pub fn tag(&self) -> &[u8; 16] {
        unsafe {
            sys::chaskey_tag(&self.context)
        }
    }
}

impl core::hash::Hasher for Context {
    #[inline]
    fn write(&mut self, buf: &[u8]) {
        self.process(buf)
    }

    fn finish(&self) -> u64 {
        let mut s = self.clone();
        s.commit();
        let &[tag0, tag1, tag2, tag3, tag4, tag5, tag6, tag7, _, _, _, _, _, _, _, _] = s.tag();
        u64::from_ne_bytes([tag0, tag1, tag2, tag3, tag4, tag5, tag6, tag7])
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Subkeys {
    pub k1: [u32; 4],
    pub k2: [u32; 4],
}

impl Subkeys {
    #[inline]
    pub fn new(k: &[u32; 4]) -> Subkeys {
        let mut k1 = MaybeUninit::uninit();
        let mut k2 = MaybeUninit::uninit();
        unsafe {
            sys::chaskey_subkeys(k1.as_mut_ptr(), k2.as_mut_ptr(), k);
            Subkeys {
                k1: k1.assume_init(),
                k2: k2.assume_init(),
            }
        }
    }

    #[inline]
    pub fn in_place(k1: &mut [u32; 4], k2: &mut [u32; 4], k: &[u32; 4]) {
        unsafe {
            sys::chaskey_subkeys(k1, k2, k)
        }
    }
}
