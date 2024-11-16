//! Allocator algorithm in lab.

#![no_std]
#![allow(unused_variables)]

extern crate axlog as log;
use allocator::{
    AllocError, AllocResult, BaseAllocator, BuddyByteAllocator, ByteAllocator, TlsfByteAllocator,
};
use core::alloc::Layout;
use core::ptr::NonNull;

// pub struct LabByteAllocator(BuddyByteAllocator);

// impl LabByteAllocator {
//     pub const fn new() -> Self {
//         Self(BuddyByteAllocator::new())
//     }
// }

// impl BaseAllocator for LabByteAllocator {
//     fn init(&mut self, start: usize, size: usize) {
//         log::info!(
//             "Initializing LabByteAllocator with start = {:#x}, size = {:#x}",
//             start,
//             size
//         );
//         self.0.init(start, size);
//     }
//     fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
//         self.0.add_memory(start, size)
//     }
// }

// impl ByteAllocator for LabByteAllocator {
//     fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
//         log::debug!("Allocating memory with layout = {:?}", layout);
//         self.0.alloc(layout)
//     }
//     fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
//         log::debug!("Deallocating memory with layout = {:?}", layout);
//         self.0.dealloc(pos, layout)
//     }
//     fn total_bytes(&self) -> usize {
//         self.0.total_bytes()
//     }
//     fn used_bytes(&self) -> usize {
//         self.0.used_bytes()
//     }
//     fn available_bytes(&self) -> usize {
//         self.0.available_bytes()
//     }
// }

const ALLOC_PER_ROUND: usize = 15;
const POOL_SIZE: usize = 1 << 18;
const MEMORY_END: usize = 0xffffffc088000000;

pub struct LabByteAllocator {
    pool_alloc: TlsfByteAllocator,
    start: usize,
    end: usize,
    long_live: usize,
    short_live: usize,
    count: usize,
}

impl LabByteAllocator {
    pub const fn new() -> Self {
        Self {
            pool_alloc: TlsfByteAllocator::new(),
            start: 0,
            end: 0,
            long_live: 0,
            short_live: 0,
            count: 0,
        }
    }
}

impl BaseAllocator for LabByteAllocator {
    fn init(&mut self, start: usize, size: usize) {
        log::info!(
            "Initializing LabByteAllocator with start = {:#x}, size = {:#x}",
            start,
            size
        );
        self.pool_alloc.init(start, POOL_SIZE);
        self.start = start + POOL_SIZE;
        self.end = MEMORY_END; // start + size;
        self.long_live = self.start;
        self.short_live = self.end;
    }
    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        unimplemented!()
    }
}

impl ByteAllocator for LabByteAllocator {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        log::debug!("Allocating memory with layout = {:?}", layout);
        if layout.align() == 8 {
            return self.pool_alloc.alloc(layout).map_err(|_| {
                log::error!("pool exhausted");
                AllocError::NoMemory
            });
        }

        self.count += 1;

        let size = layout.size();
        if (self.count - 1) % ALLOC_PER_ROUND % 2 == 0 {
            self.short_live = self.end - size;
            if self.short_live < self.long_live {
                return AllocResult::Err(AllocError::NoMemory);
            }
            return AllocResult::Ok(unsafe { NonNull::new_unchecked(self.short_live as *mut u8) });
        } else {
            self.long_live += size;
            if self.long_live > self.end {
                return AllocResult::Err(AllocError::NoMemory);
            }
            return AllocResult::Ok(unsafe {
                NonNull::new_unchecked((self.long_live - size) as *mut u8)
            });
        }
    }
    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
        log::debug!("Deallocating memory with layout = {:?}", layout);
        if layout.align() == 8 {
            self.pool_alloc.dealloc(pos, layout);
            return;
        }

        if self.end - self.short_live == layout.size() {
            self.short_live = self.end;
        }
    }
    fn total_bytes(&self) -> usize {
        self.end - self.start
    }
    fn used_bytes(&self) -> usize {
        self.long_live - self.start + self.end - self.short_live
    }
    fn available_bytes(&self) -> usize {
        self.short_live - self.long_live
    }
}
