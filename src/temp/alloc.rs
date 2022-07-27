// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    fs::{File, OpenOptions},
    marker::PhantomData,
    mem::size_of,
};

use memmap::MmapMut;

fn round_to_page(size: u64) -> u64 {
    let pagesize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 };
    let rem = size % pagesize;
    if rem == 0 {
        size
    } else {
        size + (pagesize - rem)
    }
}

/// The allocator handles managing the underlying OS file and overlay mmap. The Allocator itself,
/// does not actually contain a [T] in this case, its just used for sizing initial length of the OS
/// file, and as a marker to allow for simple [std::ops::Deref] and [std::ops::DerefMut] overrides.
pub struct Allocator<T> {
    file: File,
    mmap: MmapMut,
    phantom: PhantomData<T>,
}

impl<T> Allocator<T> {
    /// Create a new allocator using the file at the specified path. If the file does
    /// not exist, it will be created. However the directory must exist. The size parameter
    /// denotes the initial size of the mmap given the file is empty, otherwise the parameter
    /// is ignored.
    pub fn new(path: impl AsRef<str>) -> Result<Allocator<T>, String> {
        let size = round_to_page(size_of::<T>() as u64);

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path.as_ref())
            .map_err(|e| e.to_string())?;
        let len = file.metadata().map_err(|e| e.to_string())?.len();
        if len == 0 {
            file.set_len(size).map_err(|e| e.to_string())?;
        }

        let mmap = unsafe { MmapMut::map_mut(&file).map_err(|e| e.to_string())? };

        Ok(Allocator {
            file,
            mmap,
            phantom: PhantomData,
        })
    }

    pub fn with_capacity(path: impl AsRef<str>, cap: usize) -> Result<Allocator<T>, String> {
        let mut allocator = Allocator::new(path)?;
        allocator.grow(cap);
        Ok(allocator)
    }

    /// Grow the internal file to the specified capacity, this will allocate a minimum
    /// size of 4096 bytes to fill an entire page at a time. This function is immutable
    /// given the specified capacity fits within the current size of the on disk file.
    pub(crate) fn grow(&mut self, capacity: usize) {
        let size = capacity * size_of::<T>();
        let size = round_to_page(size as u64);

        let current = self
            .file
            .metadata()
            .expect("failed to read current file size.")
            .len();
        if size <= current {
            return;
        }

        self.file
            .set_len(size)
            .expect("failed to allocate more space on disk");
        self.mmap = unsafe { MmapMut::map_mut(&self.file).expect("failed to mmap file") };
    }

    /// Retrieve a valid constant pointer to the underlying memory.
    pub fn as_ptr(&self) -> *const u8 {
        self.mmap.as_ptr()
    }

    /// Retrieve a valid mutable point to the underlying memory.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.mmap.as_mut_ptr()
    }

    /// Flush dirty buffers to disk.
    pub fn flush(&self) -> Result<(), String> {
        self.mmap.flush().map_err(|e| e.to_string())
    }
}

impl<T> AsRef<[u8]> for Allocator<T> {
    fn as_ref(&self) -> &[u8] {
        &self.mmap
    }
}

impl<T> AsMut<[u8]> for Allocator<T> {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.mmap
    }
}

impl<T> Drop for Allocator<T> {
    fn drop(&mut self) {
        // Ensure we always attempt to flush the allocator. If we failed well we tried, and there is
        // nothing else we can do so just return.
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case::less_4k(3*1024, 4096)]
    #[case::greater_4k(5*1024, 8192)]
    #[case::equal_4k(4096, 4096)]
    fn test_round_to_page(#[case] input: u64, #[case] expected: u64) {
        let actual = round_to_page(input);
        assert_eq!(actual, expected)
    }
}
