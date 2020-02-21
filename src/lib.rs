#![no_std]

use core::{fmt, ptr};
use unchecked_ops::*;

#[cfg(chaskey_rounds = "8")]
pub const ROUNDS: usize = 8;

#[cfg(chaskey_rounds = "12")]
pub const ROUNDS: usize = 12;

const TAG_SIZE: usize = 0x10;

pub type KeyData = [u32; 4];

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

#[cfg(feature = "ffi")]
pub mod sys {
    use super::{KeyData, TagData, Context};

    pub type ChaskeyContext = Context<KeyData>;

    extern "C" {
        pub fn chaskey_subkeys(k1: *mut KeyData, k2: *mut KeyData, k: &KeyData);
        pub fn chaskey_init(context: *mut ChaskeyContext, k: &KeyData, k1: &KeyData, k2: &KeyData);
        pub fn chaskey_process(context: &mut ChaskeyContext, m: *const u8, len: usize);
        pub fn chaskey_finish(context: &mut ChaskeyContext);
        pub fn chaskey_tag(context: &ChaskeyContext) -> &[u8; super::TAG_SIZE];
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Context<K = KeyData> {
    tag: TagData,
    subkeys: Subkeys<K>,
    len: usize,
    m: TagData,
}

impl<K> Context<K> {
    #[inline]
    pub const fn new(k: KeyData, subkeys: Subkeys<K>) -> Self {
        Self {
            tag: TagData::new_u32(k),
            subkeys,
            len: 0,
            m: TagData::new(),
        }
    }

    #[inline]
    pub const fn from_subkeys(k: KeyData, k1: K, k2: K) -> Self {
        Self::new(k, Subkeys::new(k1, k2))
    }

    #[inline]
    pub fn tag(&self) -> &[u8; TAG_SIZE] {
        self.tag.ref_u8()
    }
}

impl Context<KeyData> {
    pub const fn from_key(k: KeyData) -> Self {
        let subkeys = Subkeys::from_key(&k);
        Self::new(k, subkeys)
    }
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
    for _ in 0..ROUNDS {
        v[0] = v[0].wrapping_add(v[1]);
        v[1] = rotl(v[1], 5);
        v[1] ^= v[0];
        v[0] = rotl(v[0], 16);

        v[2] = v[2].wrapping_add(v[3]);
        v[3] = rotl(v[3], 8);
        v[3] ^= v[2];

        v[0] = v[0].wrapping_add(v[3]);
        v[3] = rotl(v[3], 13);
        v[3] ^= v[0];

        v[2] = v[2].wrapping_add(v[1]);
        v[1] = rotl(v[1], 7);
        v[1] ^= v[2];
        v[2] = rotl(v[2], 16);
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

#[cfg(feature = "ffi")]
impl Context<KeyData> {
    #[inline]
    pub fn process(&mut self, m: &[u8]) {
        unsafe {
            sys::chaskey_process(self, m.as_ptr(), m.len())
        }
    }

    #[inline]
    pub fn commit(&mut self) {
        unsafe {
            sys::chaskey_finish(self)
        }
    }
}

#[cfg(not(feature = "ffi"))]
impl<K> Context<K> {
    pub fn process(&mut self, mut m: &[u8]) {
        let mut i = self.len & (TAG_SIZE - 1);
        while m.len() > 0 {
            if i == 0 && self.len > 0 {
                permute(self.tag.mut_u32());
            }

            let blocklen = core::cmp::min(m.len(), TAG_SIZE - i);
            let (block, rest) = m.split_at(blocklen);
            for (cm, &m) in self.m.mut_u8()[i..].iter_mut().zip(block) {
                *cm = m;
            }
            self.len = self.len.wrapping_add(blocklen);
            m = rest;

            if i + blocklen == TAG_SIZE {
                bswap(self.m.mut_u32());
                mix(self.tag.mut_u32(), self.m.ref_u32());
            }
            i = 0;
        }
    }
}

#[cfg(not(feature = "ffi"))]
impl<K: AsKeyData> Context<K> {
    pub fn commit(&mut self) {
        let i = self.len & (TAG_SIZE - 1);
        let l = if self.len != 0 && i == 0 {
            self.subkeys.k1()
        } else {
            unsafe {
                *self.m.mut_u8().get_unchecked_mut(i) = 0x01; // padding bit
                for m in self.m.mut_u8().get_unchecked_mut(i + 1..) {
                    *m = 0;
                }
            }

            bswap(self.m.mut_u32());
            mix(self.tag.mut_u32(), self.m.ref_u32());
            self.subkeys.k2()
        };

        mix(self.tag.mut_u32(), l);

        permute(self.tag.mut_u32());

        mix(self.tag.mut_u32(), l);
        bswap(self.tag.mut_u32());
    }
}

#[inline]
fn hasher_data_from_tag(tag: &[u8; TAG_SIZE]) -> u64 {
    unsafe {
        // TODO: TagData is actually aligned if align_of<u32>() == align_of<u64>()
        ptr::read_unaligned(tag.as_ptr() as *const u64)
    }
}

#[cfg(not(feature = "ffi"))]
impl<K: Clone + AsKeyData> core::hash::Hasher for Context<K> {
    #[inline]
    fn write(&mut self, buf: &[u8]) {
        self.process(buf)
    }

    fn finish(&self) -> u64 {
        let mut s = self.clone();
        s.commit();
        hasher_data_from_tag(s.tag())
    }
}

#[cfg(feature = "ffi")]
impl core::hash::Hasher for Context<KeyData> {
    #[inline]
    fn write(&mut self, buf: &[u8]) {
        self.process(buf)
    }

    fn finish(&self) -> u64 {
        let mut s = self.clone();
        s.commit();
        hasher_data_from_tag(s.tag())
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Subkeys<K = KeyData> {
    pub k1: K,
    pub k2: K,
}

impl Subkeys<KeyData> {
    #[inline]
    pub const fn from_key(k: &KeyData) -> Self {
        let k1 = timestwo(&k);
        Self::new(k1, timestwo(&k1))
    }
}

impl<K> Subkeys<K> {
    #[inline]
    pub const fn new(k1: K, k2: K) -> Self {
        Self {
            k1,
            k2,
        }
    }
}

impl<K: AsKeyData> Subkeys<K> {
    pub fn k1(&self) -> &KeyData {
        self.k1.as_key_data()
    }

    pub fn k2(&self) -> &KeyData {
        self.k2.as_key_data()
    }
}

/// Basically `AsRef<KeyData>`
pub trait AsKeyData {
    fn as_key_data(&self) -> &KeyData;
}

impl AsKeyData for KeyData {
    fn as_key_data(&self) -> &KeyData {
        self
    }
}

impl AsKeyData for &'_ KeyData {
    fn as_key_data(&self) -> &KeyData {
        *self
    }
}
