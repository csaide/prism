// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::borrow::{Borrow, BorrowMut};
use std::marker::PhantomData;
use std::mem::size_of;
use std::ops::{Deref, DerefMut};
use std::slice;

use memmap::{MmapMut, MmapOptions};

use super::{Error, Result};

pub unsafe trait Allocatable {}

pub struct Block<T: Allocatable> {
    mmap: MmapMut,
    t: PhantomData<*mut T>,
}

impl<T: Allocatable> Block<T> {
    pub fn new(item: T) -> Block<T> {
        let b = unsafe { slice::from_raw_parts((&item as *const T) as *const u8, size_of::<T>()) };
        let mut block = Block::<T>::default();
        let raw: &mut [u8] = block.as_mut();
        b.iter()
            .enumerate()
            .for_each(|(idx, byte)| raw[idx] = *byte);
        block
    }

    pub fn with_mmap(mmap: MmapMut) -> Block<T> {
        Block {
            mmap,
            t: PhantomData,
        }
    }

    pub fn flush(&self) -> Result<()> {
        self.mmap.flush_async().map_err(Error::mmap)
    }

    pub fn as_ptr(&self) -> *const T {
        self.mmap.as_ptr() as *const T
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.mmap.as_mut_ptr() as *mut T
    }

    pub fn as_raw_ptr(&self) -> *const u8 {
        self.mmap.as_ptr()
    }

    pub fn as_mut_raw_ptr(&mut self) -> *mut u8 {
        self.mmap.as_mut_ptr()
    }
}

impl<T: Allocatable> AsRef<[u8]> for Block<T> {
    fn as_ref(&self) -> &[u8] {
        &self.mmap
    }
}

impl<T: Allocatable> AsMut<[u8]> for Block<T> {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.mmap
    }
}

impl<T: Allocatable> AsRef<T> for Block<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: Allocatable> AsMut<T> for Block<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: Allocatable> Borrow<T> for Block<T> {
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}

impl<T: Allocatable> BorrowMut<T> for Block<T> {
    fn borrow_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}

impl<T: Allocatable> Deref for Block<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.as_ptr() }
    }
}

impl<T: Allocatable> DerefMut for Block<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.as_mut_ptr() }
    }
}

impl<T: Allocatable + PartialEq> PartialEq for Block<T> {
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T: Allocatable + Eq> Eq for Block<T> {}

impl<T: Allocatable + PartialOrd> PartialOrd for Block<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.deref().partial_cmp(other.deref())
    }
}

impl<T: Allocatable + Ord> Ord for Block<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.deref().cmp(other.deref())
    }
}

impl<T: Allocatable + core::hash::Hash> core::hash::Hash for Block<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}

impl<T: Allocatable + std::fmt::Debug> std::fmt::Debug for Block<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: Allocatable + std::fmt::Display> std::fmt::Display for Block<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: Allocatable> Drop for Block<T> {
    fn drop(&mut self) {
        // Attempt to flush the internall mamp, but if it errors
        // at this point there is nothing we can do. Just give up
        // and return.
        let _ = self.flush();
    }
}

unsafe impl<T: Send + Allocatable> Send for Block<T> {}
unsafe impl<T: Sync + Allocatable> Sync for Block<T> {}

impl<T: Allocatable> From<MmapMut> for Block<T> {
    fn from(mmap: MmapMut) -> Block<T> {
        Block::with_mmap(mmap)
    }
}

impl<T: Allocatable> Default for Block<T> {
    fn default() -> Block<T> {
        let mmap = MmapOptions::new()
            .len(size_of::<T>())
            .map_anon()
            .expect("failed to mmap anonymous map");

        Block::from(mmap)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[repr(C)]
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
    struct Tester {
        pub first: u64,
        pub second: u64,
    }

    unsafe impl Allocatable for Tester {}

    #[rstest]
    fn test_default_block() {
        let mut block = Block::<Tester>::default();
        block.first = 1234;
        block.second = 5678;
        println!("{:?}", block);

        let test = Block::new(Tester {
            first: 1234,
            second: 5678,
        });
        assert_eq!(test, block)
    }
}
