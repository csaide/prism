// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    marker::PhantomData,
    path::Path,
};

use memmap::MmapOptions;

use super::{
    block::{Allocatable, Block},
    mapping::BlockMapping,
    util, Error, Result,
};

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Header {
    pub length: usize,
    pub capacity: usize,
}

unsafe impl Allocatable for Header {}

/// A mapper handles mapping a given OS file to a sequential set of
/// memory blocks which have the size of the supplied [T]. These blocks
/// can be operated on independently and in parallel.
///
/// The blocks themselves are persistent once allocated and subsequently mapped.
/// This means that dropping a value doesn't operate in the same sense as it does
/// elsewhere. Here when the block is "dropped" in the traditional sense it becomes
/// unmapped, but NOT unallocated. The block can then be retrieved at a latter date,
/// by index, via the [Mapper::map] function.
///
/// In order to drop the memory completely a call to [Mapper::free] must be made, which would
/// return the block to the file unmapping it, and then "forgetting" the current value so it
/// can be overwritten at a later time.
///
/// Resulting file format is:
///
/// {directory}/{name}.db    -> main block storage.
///     | header | block | block | ... | block |
///
/// {directory}/{name}.map -> block index mapping for free/mapped blocks.
///     | mapping | mapping | ... | mapping |
///
pub struct BlockAllocator<T: Allocatable> {
    file: File,
    header: Block<Header>,
    mapping: BlockMapping,
    free: VecDeque<usize>,
    t: PhantomData<T>,
}

impl<T: Allocatable> BlockAllocator<T> {
    pub fn new(name: impl AsRef<str>, dir: impl AsRef<Path>) -> Result<BlockAllocator<T>> {
        BlockAllocator::with_capacity(name, dir, 1)
    }

    pub fn with_capacity(
        name: impl AsRef<str>,
        dir: impl AsRef<Path>,
        cap: usize,
    ) -> Result<BlockAllocator<T>> {
        let file = dir.as_ref().join(format!("{}.db", name.as_ref()));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file)
            .map_err(Error::file)?;
        if file.metadata().map_err(Error::file)?.len() == 0 {
            file.set_len(
                (util::aligned_size_of::<Header>() + (cap * util::aligned_size_of::<T>())) as u64,
            )
            .map_err(Error::file)?;
        }

        let header = unsafe {
            MmapOptions::new()
                .len(util::aligned_size_of::<Header>())
                .map_mut(&file)
                .map_err(Error::mmap)?
        };
        let mut header = Block::<Header>::with_mmap(header);
        if header.capacity == 0 {
            // We can't have a 0 capacity, so if we _do_ have a 0 capacity
            // it's because this is a brand new instance, so update accordingly.
            header.capacity = cap;
            header.flush()?;
        }

        let mapping = BlockMapping::new(name, dir)?;
        let free = mapping
            .iter()
            .enumerate()
            .filter_map(|(idx, slot)| if slot.is_free() { Some(idx) } else { None })
            .collect();
        Ok(BlockAllocator {
            file,
            header,
            mapping,
            free,
            t: PhantomData,
        })
    }

    fn grow(&mut self) {
        // If we are empty default to a single block, this matches with
        // the standard lib collections. Otherwise double the existing
        // capacity.
        self.header.capacity = if self.header.capacity == 0 {
            1
        } else {
            self.header.capacity * 2
        };

        // Because this header is actually file backed, lets flush or panic if it fails.
        self.header.flush().expect("failed to flush header to disk");

        // So size here has to be page aligned for this all to come together and work, so really
        // regardless of type we have stored we will be at 4096B boundries (usually), so this isn't
        // exactly efficient for small types. To fix that though requires arbitrary sizing, which really
        // we can't do due to the limitations of mmap.
        let size =
            util::aligned_size_of::<T>() * self.header.capacity + util::aligned_size_of::<Header>();
        self.file
            .set_len(size as u64)
            .expect("failed to grow db file");

        // Now grow the mapping index, to the correct number of elements.
        self.mapping.grow(self.header.capacity);

        // TODO(csaide): Fix this to be more efficient.
        self.free = self
            .mapping
            .iter()
            .enumerate()
            .filter_map(|(idx, slot)| if slot.is_free() { Some(idx) } else { None })
            .collect();
    }

    pub fn cap(&self) -> usize {
        self.header.capacity
    }

    pub fn len(&self) -> usize {
        self.header.length
    }

    pub fn is_empty(&self) -> bool {
        self.header.length == 0
    }

    pub fn allocate(&mut self) -> Block<T> {
        let mut offset = self.free.pop_front();
        while offset.is_none() {
            self.grow();
            offset = self.free.pop_front();
        }
        let offset = offset.unwrap();

        self.mapping[offset].map();
        self.mapping
            .flush()
            .expect("Failed to flush mapping index.");
        self.header.length += 1;
        self.header.flush().expect("Failed to flush file header.");

        let mmap = unsafe {
            MmapOptions::new()
                .offset(
                    (util::aligned_size_of::<Header>() + offset * util::aligned_size_of::<T>())
                        as u64,
                )
                .len(util::aligned_size_of::<T>())
                .map_mut(&self.file)
                .map_err(Error::mmap)
                .expect("failed to map file")
        };
        Block::with_mmap(mmap)
    }

    pub fn map(&self, offset: usize) -> Block<T> {
        assert!(offset < self.header.capacity);
        assert!(self.mapping[offset].is_mapped());

        let mmap = unsafe {
            MmapOptions::new()
                .offset(
                    (util::aligned_size_of::<Header>() + offset * util::aligned_size_of::<T>())
                        as u64,
                )
                .len(util::aligned_size_of::<T>())
                .map_mut(&self.file)
                .map_err(Error::mmap)
                .expect("failed to map file")
        };
        Block::with_mmap(mmap)
    }

    pub fn free(&mut self, offset: usize) {
        assert!(offset < self.header.capacity);
        assert!(self.mapping[offset].is_mapped());

        self.mapping[offset].free();
        self.mapping
            .flush()
            .expect("Failed to flush mapping index.");
        self.header.length -= 1;
        self.header.flush().expect("Failed to flush file header.");
        self.free.push_back(offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C)]
    struct Testing {
        pub first: u64,
        pub second: u64,
    }

    unsafe impl Allocatable for Testing {}

    #[test]
    fn test_mapper() {
        let mut mapper =
            BlockAllocator::<Testing>::new("mapper", "").expect("failed to open new mapper");

        {
            let mut first = mapper.allocate();
            first.first = 1;
            first.second = 2;
        }

        {
            let mut second = mapper.allocate();
            second.first = 3;
            second.second = 4;
        }

        let first = mapper.map(0);
        assert_eq!(first.first, 1);
        assert_eq!(first.second, 2);
        let second = mapper.map(1);
        assert_eq!(second.first, 3);
        assert_eq!(second.second, 4);
    }
}
