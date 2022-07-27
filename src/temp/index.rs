// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ops::{Deref, DerefMut};

use super::{block::Allocatable, Allocator};

pub type Index = Allocator<InnerIndex>;

/// This inner object represents the persistent length and capacity of a [Log] object.
#[repr(C)]
pub struct InnerIndex {
    pub length: u64,
    pub capacity: u64,
}

impl InnerIndex {
    /// Return whether or not we need to grow the underlying mmap file.
    pub fn should_grow(&self) -> bool {
        self.length == self.capacity
    }

    /// Fetch and then increment the current length, returning the old value.
    pub fn fetch_and_inc(&mut self) -> u64 {
        let len = self.length;
        self.length += 1;
        len
    }

    /// Decrement the current length and return the result, if the current length is 0 then
    /// 0 is returned.
    pub fn dec_and_fetch(&mut self) -> u64 {
        if self.length == 0 {
            return 0;
        }
        self.length -= 1;
        self.length
    }
}

unsafe impl Allocatable for InnerIndex {}

impl AsRef<InnerIndex> for Index {
    #[inline]
    fn as_ref(&self) -> &InnerIndex {
        self
    }
}

impl AsMut<InnerIndex> for Index {
    #[inline]
    fn as_mut(&mut self) -> &mut InnerIndex {
        self
    }
}

impl Deref for Index {
    type Target = InnerIndex;

    fn deref(&self) -> &Self::Target {
        // This is safe because we are guaranteed to be at the 0 position in the memory,
        // AND this memory is guaranteed to be at least 4096 bytes where the InnerIndex
        // is only 16 bytes.
        //
        // Also the InnerIndex is a repr(C) with two u64 values. This is _not_ cross platform
        // compatible, but that should be fine in theory. Means back ups may prove to be problematic
        // in the future.
        //
        // TODO(csaide): Decide whether we need to use the zerocopy crate here to handle endianess.
        unsafe { &*(self.as_ptr() as *const InnerIndex) }
    }
}

impl DerefMut for Index {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // This is safe because we are guaranteed to be at the 0 position in the memory,
        // AND this memory is guaranteed to be at least 4096 bytes where the InnerIndex
        // is only 16 bytes.
        //
        // Also the InnerIndex is a repr(C) with two u64 values. This is _not_ cross platform
        // compatible, but that should be fine in theory. Means back ups may prove to be problematic
        // in the future.
        //
        // TODO(csaide): Decide whether we need to use the zerocopy crate here to handle endianess.
        unsafe { &mut *(self.as_mut_ptr() as *mut InnerIndex) }
    }
}
