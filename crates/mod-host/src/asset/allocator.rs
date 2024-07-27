use std::ffi::c_void;

#[repr(C)]
pub(crate) struct HeapAllocator {
    pub vmt: *const HeapAllocatorVMT,
}

#[repr(C)]
pub(crate) struct HeapAllocatorVMT {
    pub destructor: fn(this: *const HeapAllocator, param_2: bool),
    pub allocator_id: fn(this: *const HeapAllocator) -> u32,
    _unk10: fn(this: *const Self),
    pub heap_flags: fn(this: *const HeapAllocator, out: *mut u64) -> *const u64,
    pub heap_capacity: fn(this: *const HeapAllocator) -> u64,
    pub heap_size: fn(this: *const HeapAllocator) -> u64,
    pub backing_heap_capacity: fn(this: *const HeapAllocator) -> u64,
    pub heap_allocation_count: fn(this: *const HeapAllocator) -> u64,
    pub msize: fn(this: *const HeapAllocator, allocation: *const c_void) -> u64,
    pub allocate: fn(this: *const HeapAllocator, size: u64) -> *const c_void,
    pub allocate_aligned: fn(this: *const HeapAllocator, size: u64, alignment: u64) -> *const c_void,
    pub reallocate: fn(this: *const HeapAllocator, allocation: *const c_void, size: u64) -> *const c_void,
    pub reallocate_aligned: fn(this: *const HeapAllocator, allocation: *const c_void, size: u64, alignment: u64) -> *const c_void,
    pub deallocate: fn(this: *const HeapAllocator, allocation: *const c_void),
    _unk70: fn(this: *const HeapAllocator),
    pub allocate_second: fn(this: *const HeapAllocator, size: u64) -> *const c_void,
    pub allocate_second_aligned: fn(this: *const HeapAllocator, size: u64, alignment: u64) -> *const c_void,
    pub reallocate_second: fn(this: *const HeapAllocator, allocation: *const c_void, size: u64) -> *const c_void,
    pub reallocate_second_aligned: fn(this: *const HeapAllocator, allocation: *const c_void, size: u64, alignment: u64) -> *const c_void,
    pub deallocate_second: fn(this: *const HeapAllocator, allocation: *const c_void),
    pub unka0: fn(this: *const HeapAllocator) -> bool,
    pub allocation_belongs_to_first_allocator: fn(this: *const HeapAllocator, allocation: *const c_void) -> bool,
    pub allocation_belongs_to_second_allocator: fn(this: *const HeapAllocator, allocation: *const c_void) -> bool,
    pub lock: fn(this: *const HeapAllocator),
    pub unlock: fn(this: *const HeapAllocator),
    pub get_memory_block: fn(this: *const HeapAllocator, allocation: *const c_void) -> *const c_void,
}
