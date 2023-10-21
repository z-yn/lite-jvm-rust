use std::alloc::Layout;

/// Models an allocated chunk of memory
pub(crate) struct MemoryChunk {
    memory: *mut u8,
    used: usize,
    capacity: usize,
}

impl MemoryChunk {
    pub(crate) fn new(capacity: usize) -> Self {
        let layout = Layout::from_size_align(capacity, 8).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        MemoryChunk {
            memory: ptr,
            capacity,
            used: 0,
        }
    }

    pub(crate) fn alloc(&mut self, required_size: usize) -> Option<(*mut u8, usize)> {
        if self.used + required_size > self.capacity {
            return None;
        }

        // We require all allocations to be aligned to 8 bytes!
        assert_eq!(required_size % 8, 0);

        let ptr = unsafe { self.memory.add(self.used) };
        self.used += required_size;

        Some((ptr, required_size))
    }

    unsafe fn contains(&self, ptr: *const u8) -> bool {
        ptr >= self.memory && ptr <= self.memory.add(self.used)
    }

    fn reset(&mut self) {
        self.used = 0;

        // Zero the memory, to attempt and catch bugs
        unsafe {
            std::ptr::write_bytes(self.memory, 0, self.capacity);
        }
    }
}
