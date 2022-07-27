// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr, slice,
};

use super::{Allocator, Index};

pub struct Log<T> {
    alloc: Allocator<T>,
    index: Index,
    phantom: PhantomData<T>,
}

impl<T> Log<T> {
    pub fn new(path: impl AsRef<str>) -> Result<Log<T>, String> {
        let alloc = Allocator::new(path.as_ref())?;

        let index = format!("{}.index", path.as_ref());
        let index = Index::new(index)?;

        Ok(Log {
            alloc,
            index,
            phantom: PhantomData,
        })
    }

    pub fn with_capacity(path: impl AsRef<str>, cap: usize) -> Result<Log<T>, String> {
        let alloc = Allocator::with_capacity(path.as_ref(), cap)?;

        let index = format!("{}.index", path.as_ref());
        let index = Index::new(index)?;

        Ok(Log {
            alloc,
            index,
            phantom: PhantomData,
        })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.index.length as usize
    }

    #[inline]
    pub fn cap(&self) -> usize {
        self.index.capacity as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.index.length == 0
    }

    fn grow(&mut self) {
        if self.index.capacity == 0 {
            self.index.capacity = 1;
        } else {
            self.index.capacity *= 2;
        }

        self.alloc.grow(self.cap());
    }

    pub fn push(&mut self, elem: T) {
        if self.index.should_grow() {
            self.grow()
        }

        let loc = self.index.fetch_and_inc() as usize;
        unsafe {
            ptr::write(self.as_mut_ptr().add(loc), elem);
        }
        // self.alloc.flush().expect("failed to flush to disk")
    }

    pub fn pop(&mut self) -> Option<T> {
        let length = self.index.dec_and_fetch();

        let result = if length == 0 {
            None
        } else {
            unsafe { Some(ptr::read(self.as_ptr().add(length as usize))) }
        };
        self.alloc.flush().expect("failed to flush to disk");
        result
    }

    pub fn insert(&mut self, idx: usize, elem: T) {
        assert!(idx as u64 <= self.index.length, "index out of bounds");
        if self.index.should_grow() {
            self.grow()
        }

        let length = self.index.length as usize;
        unsafe {
            ptr::copy(
                self.as_ptr().add(idx),
                self.as_mut_ptr().add(idx + 1),
                length - idx,
            );
            ptr::write(self.as_mut_ptr().add(idx), elem);
            self.index.length += 1;
        }
        self.alloc.flush().expect("failed to flush to disk")
    }

    pub fn remove(&mut self, index: usize) -> T {
        // Note: `<` because it's *not* valid to remove after everything
        let len = self.len();
        assert!(index < len, "index out of bounds");
        self.index.length -= 1;
        let result = unsafe {
            let result = ptr::read(self.as_ptr().add(index));
            ptr::copy(
                self.as_ptr().add(index + 1),
                self.as_mut_ptr().add(index),
                len - index,
            );
            result
        };
        self.alloc.flush().expect("failed to flush to disk");
        result
    }
}

unsafe impl<T: Send> Send for Log<T> {}
unsafe impl<T: Sync> Sync for Log<T> {}

impl<T> Deref for Log<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.alloc.as_ptr() as *const T, self.len()) }
    }
}

impl<T> DerefMut for Log<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { slice::from_raw_parts_mut(self.alloc.as_mut_ptr() as *mut T, self.len()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log() {
        let mut log = Log::<u64>::new("testing.db").expect("Failed to setup database");
        println!(
            "Data: {:?}\nLen: {}\nCap: {}",
            log.deref(),
            log.len(),
            log.cap()
        );
        for idx in 0..8 {
            log.push(idx)
        }
        let actual = log.pop();
        assert!(actual.is_some());
        assert_eq!(actual.unwrap(), 7);

        let ins = 999999;
        log.insert(3, ins);
        let actual = log.remove(3);
        assert_eq!(actual, ins)
    }
}
