// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use lazy_static::{__Deref, lazy_static};

lazy_static! {
    static ref PAGESIZE: usize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
}

fn round_to_page(size: usize) -> usize {
    let rem = size % PAGESIZE.deref();
    if rem == 0 {
        size
    } else {
        size + (PAGESIZE.deref() - rem)
    }
}

pub fn aligned_size_of<T>() -> usize {
    round_to_page(size_of::<T>())
}

pub fn size_of<T>() -> usize {
    std::mem::size_of::<T>()
}
