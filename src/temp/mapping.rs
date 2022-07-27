// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    fs::{File, OpenOptions},
    ops::{Deref, DerefMut},
    path::Path,
    slice,
};

use memmap::MmapMut;

use super::{util, Error, Result};

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Slot {
    #[default]
    Free = 0,
    Mapped = 1,
}

impl Slot {
    pub fn is_free(&self) -> bool {
        matches!(self, Slot::Free)
    }

    pub fn is_mapped(&self) -> bool {
        matches!(self, Slot::Mapped)
    }

    pub fn free(&mut self) {
        assert!(self.is_mapped());
        *self = Slot::Free;
    }

    pub fn map(&mut self) {
        assert!(self.is_free());
        *self = Slot::Mapped;
    }
}

pub struct BlockMapping {
    file: File,
    alloc: MmapMut,
}

impl BlockMapping {
    pub fn new(name: impl AsRef<str>, dir: impl AsRef<Path>) -> Result<BlockMapping> {
        let file = dir.as_ref().join(format!("{}.map", name.as_ref()));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file)
            .map_err(Error::file)?;
        if file.metadata().map_err(Error::file)?.len() == 0 {
            file.set_len(util::size_of::<Slot>() as u64)
                .map_err(Error::file)?;
        }

        let alloc = unsafe { MmapMut::map_mut(&file).map_err(Error::mmap)? };
        Ok(BlockMapping { file, alloc })
    }

    pub fn len(&self) -> usize {
        self.file
            .metadata()
            .expect("failed to read index file metadata.")
            .len() as usize
            / util::size_of::<Slot>()
    }

    pub fn flush(&self) -> Result<()> {
        self.alloc.flush_async().map_err(Error::mmap)
    }

    pub fn grow(&mut self, cap: usize) {
        self.flush()
            .expect("failed to flush mmap to disk before growing");

        let size = cap * util::size_of::<Slot>();
        self.file
            .set_len(size as u64)
            .expect("failed to allocate more space on disk for new index entries");
        self.alloc = unsafe {
            MmapMut::map_mut(&self.file).expect("failed to mmap file contents into memory")
        };
    }
}

impl Deref for BlockMapping {
    type Target = [Slot];
    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.alloc.as_ptr() as *const Slot, self.len()) }
    }
}

impl DerefMut for BlockMapping {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { slice::from_raw_parts_mut(self.alloc.as_mut_ptr() as *mut Slot, self.len()) }
    }
}

impl Drop for BlockMapping {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
