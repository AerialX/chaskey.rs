#![no_std]

use core::fmt;
use unchecked_ops::*;

const TAG_SIZE: usize = 0x10;

#[derive(Copy, Clone)]
#[repr(C)]
union TagData {
    u8_: [u8; TAG_SIZE],
    u32_: [u32; 4],
}

impl fmt::Debug for TagData {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.ref_u32(), f)
    }
}

#[allow(dead_code)]
impl TagData {
    #[inline]
    pub const fn new_u32(u32_: [u32; 4]) -> Self {
        Self {
            u32_,
        }
    }

    #[inline]
    pub const fn new_u8(u8_: [u8; TAG_SIZE]) -> Self {
        Self {
            u8_,
        }
    }

    #[inline]
    pub const fn new() -> Self {
        Self {
            u32_: [0; 4],
        }
    }

    #[inline]
    pub fn ref_u8(&self) -> &[u8; TAG_SIZE] {
        unsafe {
            &self.u8_
        }
    }

    #[inline]
    pub fn mut_u8(&mut self) -> &mut [u8; TAG_SIZE] {
        unsafe {
            &mut self.u8_
        }
    }

    #[inline]
    pub fn ref_u32(&self) -> &[u32; 4] {
        unsafe {
            &self.u32_
        }
    }

    #[inline]
    pub fn mut_u32(&mut self) -> &mut [u32; 4] {
        unsafe {
            &mut self.u32_
        }
    }
}

impl Default for TagData {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl From<[u32; 4]> for TagData {
    #[inline]
    fn from(data: [u32; 4]) -> Self {
        Self::new_u32(data)
    }
}

impl From<[u8; TAG_SIZE]> for TagData {
    #[inline]
    fn from(data: [u8; TAG_SIZE]) -> Self {
        Self::new_u8(data)
    }
}

pub mod sys {
    use super::TagData;

    #[derive(Debug, Copy, Clone)]
    #[cfg_attr(feature = "ffi", repr(C))]
    pub struct ChaskeyContext {
        pub(crate) tag: TagData,
        pub(crate) k1: [u32; 4],
        pub(crate) k2: [u32; 4],
        pub(crate) len: usize,
        pub(crate) m: TagData,
    }

    #[cfg(feature = "ffi")]
    extern "C" {
        pub fn chaskey_subkeys(k1: *mut [u32; 4], k2: *mut [u32; 4], k: &[u32; 4]);
        pub fn chaskey_init(context: *mut ChaskeyContext, k: &[u32; 4], k1: &[u32; 4], k2: &[u32; 4]);
        pub fn chaskey_process(context: &mut ChaskeyContext, m: *const u8, len: usize);
        pub fn chaskey_finish(context: &mut ChaskeyContext);
        pub fn chaskey_tag(context: &ChaskeyContext) -> &[u8; super::TAG_SIZE];
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Context {
    context: sys::ChaskeyContext,
}

impl Context {
    #[inline]
    pub const fn new(k: [u32; 4], k1: [u32; 4], k2: [u32; 4]) -> Self {
        Self {
            context: sys::ChaskeyContext {
                tag: TagData::new_u32(k),
                k1,
                k2,
                len: 0,
                m: TagData::new(),
            },
        }
    }

    #[inline]
    pub const fn from_subkeys(k: [u32; 4], subkeys: Subkeys) -> Self {
        Self::new(k, subkeys.k1, subkeys.k2)
    }

    pub const fn from_key(k: [u32; 4]) -> Self {
        let subkeys = Subkeys::new(&k);
        Self::from_subkeys(k, subkeys)
    }

    #[inline]
    #[cfg(feature = "ffi")]
    pub fn process(&mut self, m: &[u8]) {
        unsafe {
            sys::chaskey_process(&mut self.context, m.as_ptr(), m.len())
        }
    }

    #[inline]
    #[cfg(feature = "ffi")]
    pub fn commit(&mut self) {
        unsafe {
            sys::chaskey_finish(&mut self.context)
        }
    }

    #[inline]
    pub fn tag(&self) -> &[u8; TAG_SIZE] {
        self.context.tag.ref_u8()
    }

    const fn timestwo(input: &[u32; 4]) -> [u32; 4] {
        const C: [u32; 2] = [0x00, 0x87];

        [
            (input[0] << 1) ^ C[(input[3] >> 31) as usize],
            (input[1] << 1) | (input[0] >> 31),
            (input[2] << 1) | (input[1] >> 31),
            (input[3] << 1) | (input[2] >> 31),
        ]
    }

    #[cfg(not(feature = "ffi"))]
    fn rotl(x: u32, b: u32) -> u32 {
        unsafe {
            let rev = 32.unchecked_sub(b);
            let x = Unchecked::new(x);
            ((x >> rev) | (x << b)).value()
        }
    }

    #[cfg(not(feature = "ffi"))]
    fn permute(v: &mut [u32; 4]) {
        for _ in 0..Self::ROUNDS {
            v[0] = v[0].wrapping_add(v[1]);
            v[1] = Self::rotl(v[1], 5);
            v[1] ^= v[0];
            v[0] = Self::rotl(v[0], 16);

            v[2] = v[2].wrapping_add(v[3]);
            v[3] = Self::rotl(v[3], 8);
            v[3] ^= v[2];

            v[0] = v[0].wrapping_add(v[3]);
            v[3] = Self::rotl(v[3], 13);
            v[3] ^= v[0];

            v[2] = v[2].wrapping_add(v[1]);
            v[1] = Self::rotl(v[1], 7);
            v[1] ^= v[2];
            v[2] = Self::rotl(v[2], 16);
        }
    }

    #[cfg(not(feature = "ffi"))]
    fn mix(tag: &mut [u32; 4], l: &[u32; 4]) {
        for (l, tag) in l.iter().zip(tag.iter_mut()) {
            *tag ^= l;
        }
  }

    #[cfg(all(not(feature = "ffi"), target_endian = "big"))]
    fn bswap(tag: &mut [u32; 4]) {
        for tag in tag.iter_mut() {
            *tag = tag.to_le();
        }
    }

    #[inline]
    #[cfg(all(not(feature = "ffi"), target_endian = "little"))]
    fn bswap(_tag: &mut [u32; 4]) { }

    #[cfg(not(feature = "ffi"))]
    pub fn process(&mut self, mut m: &[u8]) {
        let mut i = self.context.len & (TAG_SIZE - 1);
        while m.len() > 0 {
            if i == 0 && self.context.len > 0 {
                Self::permute(self.context.tag.mut_u32());
            }

            let blocklen = core::cmp::min(m.len(), TAG_SIZE - i);
            let (block, rest) = m.split_at(blocklen);
            for (cm, &m) in self.context.m.mut_u8()[i..].iter_mut().zip(block) {
                *cm = m;
            }
            self.context.len = self.context.len.wrapping_add(blocklen);
            m = rest;

            if i + blocklen == TAG_SIZE {
                Self::bswap(self.context.m.mut_u32());
                Self::mix(self.context.tag.mut_u32(), self.context.m.ref_u32());
            }
            i = 0;
        }
    }

    #[cfg(not(feature = "ffi"))]
    pub fn commit(&mut self) {
        let i = self.context.len & (TAG_SIZE - 1);
        let l = if self.context.len != 0 && i == 0 {
            &self.context.k1
        } else {
            unsafe {
                *self.context.m.mut_u8().get_unchecked_mut(i) = 0x01; // padding bit
                for m in self.context.m.mut_u8().get_unchecked_mut(i + 1..) {
                    *m = 0;
                }
            }

            Self::bswap(self.context.m.mut_u32());
            Self::mix(self.context.tag.mut_u32(), self.context.m.ref_u32());
            &self.context.k2
        };

        Self::mix(self.context.tag.mut_u32(), l);

        Self::permute(self.context.tag.mut_u32());

        Self::mix(self.context.tag.mut_u32(), l);
        Self::bswap(self.context.tag.mut_u32());
    }

    #[cfg(chaskey_rounds = "8")]
    pub const ROUNDS: usize = 8;

    #[cfg(chaskey_rounds = "12")]
    pub const ROUNDS: usize = 12;
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

/*pub trait Tag {
    fn as_u32(&self) -> &[u32; 4];
    fn as_u8(&self) -> &[u8; 16];
    fn as_u128(&self) -> &u128;
}*/

#[derive(Debug, Copy, Clone)]
pub struct Subkeys {
    pub k1: [u32; 4],
    pub k2: [u32; 4],
}

impl Subkeys {
    #[inline]
    pub const fn new(k: &[u32; 4]) -> Subkeys {
        let k1 = Context::timestwo(&k);
        Subkeys {
            k2: Context::timestwo(&k1),
            k1,
        }
    }
}
