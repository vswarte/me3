use std::{ffi::c_void, ptr::copy_nonoverlapping, string::FromUtf16Error};

use thiserror::Error;

use super::allocator::HeapAllocator;

#[derive(Debug, Error)]
pub enum DLStringError {
    #[error("Could not find DLString terminator")]
    NoTerminator,

    #[error("Could not decode UTF16 string. {0}")]
    FromUtf16(#[from] FromUtf16Error),
}

#[repr(C)]
union DLStringInner {
    pointer: *mut u16,
    bytes: [u16; 0x8],
}

#[repr(C)]
pub struct DLString {
    allocator: *const HeapAllocator,

    /// Either contains a pointer tot the string with its backing memory coming
    /// from the allocator, or the string's bytes are inlined into the
    /// [u16; 0x8].
    inner: DLStringInner,

    /// Amount of characters in the string. Excludes null terminator.
    length: u64,

    /// Amount of characters the string can host. Excludes null terminator.
    capacity: u64,
}

impl DLString {
    /// Yields a &[u16] representing the strings characters.
    ///
    /// # SAFETY
    /// if the capacity check is wrong it might interpret UTF-16 as a pointer and vice versa.
    unsafe fn bytes(&self) -> &[u16] {
        if 7 < self.capacity {
            std::slice::from_raw_parts(
                self.inner.pointer,
                self.length as usize,
            )
        } else {
            &self.inner.bytes[..self.length as usize]
        }
    }

    /// Yields a &mut [u16] representing the strings characters.
    /// The buffer includes the NULL terminator which is not accounted for
    /// in the length or capacity fields.
    ///
    /// # SAFETY
    /// if the capacity check is wrong it might interpret UTF-16 as a pointer and vice versa.
    unsafe fn buffer_mut(&mut self) -> &mut [u16] {
        if 7 < self.capacity {
            std::slice::from_raw_parts_mut(
                self.inner.pointer,
                self.capacity as usize + 1,
            )
        } else {
            &mut self.inner.bytes[..self.capacity as usize + 1]
        }
    }

    /// Yields a String containing the DLString's string contents.
    pub fn as_string(&self) -> String {
        String::from_utf16_lossy(unsafe { self.bytes() })
    }

    /// Replaces the DLStrings string contents with the input.
    ///
    /// # SAFETY
    /// Writes to the DLString so assumes exclusive mutable access.
    pub unsafe fn set_contents(&mut self, input: &[u16]) {
        // Length does not count the NULL terminator.
        let new_length = input.len() - 1;

        // Check if the input will fit in the current allocation.
        if new_length <= self.capacity as usize {
            // Check if the new string should be hosted in-line.
            let pointer = if 7 >= new_length {
                // We must put the string in-line, so we need to deallocate
                // any heap allocations.
                let vmt = self.allocator.as_ref().unwrap().vmt.as_ref().unwrap();
                (vmt.deallocate)(self.allocator, self.inner.pointer as *const c_void);

                self.capacity = 7;
                self.inner.bytes.as_mut_ptr()
            } else {
                // We need to keep the heap alloc to host the string
                self.inner.pointer
            };

            copy_nonoverlapping(
                input.as_ref().as_ptr(),
                self.buffer_mut().as_mut_ptr(),
                input.len(),
            );
        } else {
            // We need to grow the underlying allocation...

            // Get the byte size of our &[u16]
            let alloc_size = (input.len() * 2) as u64;

            let vmt = self.allocator.as_ref().unwrap().vmt.as_ref().unwrap();

            // We check if there is an allocation for the string already
            // and we reallocate it if so, we do an ordinary allocate if the
            // string is current inlined.
            let new_allocation = if 7 < self.capacity {
                (vmt.reallocate)(
                    self.allocator,
                    self.inner.pointer as *const c_void,
                    alloc_size,
                )
            } else {
                (vmt.allocate)(self.allocator, alloc_size)
            } as *mut u16;

            // Copy the rewritten path into the allocated memory
            copy_nonoverlapping(
                input.as_ref().as_ptr(),
                new_allocation,
                input.len(),
            );

            self.inner.pointer = new_allocation;

            // Capacity is for some reason also NULL terminator excluded.
            self.capacity = input.len() as u64 - 1;
        }

        self.length = new_length as u64;
    }
}

pub trait DLStringAllocator {
    fn allocate(&self, size: u64) -> *mut u16;

    fn reallocate(&self, allocation: *const c_void, size: u64) -> *mut u16;

    fn deallocate(&self, allocation: *const c_void);
}

pub struct DLStringFSHeapAllocator<'a> {
    allocator: &'a HeapAllocator,
}

impl DLStringAllocator for DLStringFSHeapAllocator<'_> {
    fn allocate(&self, size: u64) -> *mut u16 {
        unsafe {
            (self.allocator.vmt.as_ref().unwrap().allocate)(self.allocator, size) as _
        }
    }

    fn reallocate(&self, allocation: *const c_void, size: u64) -> *mut u16 {
        unsafe {
            (self.allocator.vmt.as_ref().unwrap().reallocate)(self.allocator, allocation, size) as _
        }
    }

    fn deallocate(&self, allocation: *const c_void) {
        unsafe {
            (self.allocator.vmt.as_ref().unwrap().deallocate)(self.allocator, allocation);
        }
    }
}

#[cfg(test)]
mod test {
    use super::{DLString, DLStringAllocator, DLStringInner, HeapAllocator};

    struct DummyStringAllocator;

    impl DLStringAllocator for DummyStringAllocator {
        fn allocate(&self, size: u64) -> *mut u16 {
            todo!()
        }

        fn reallocate(&self, allocation: *const std::ffi::c_void, size: u64) -> *mut u16 {
            todo!()
        }

        fn deallocate(&self, allocation: *const std::ffi::c_void) {
            todo!()
        }
    }

    #[test]
    fn fits_in_allocation() {
        let mut dlstring = DLString {
            allocator: std::ptr::null(),
            inner: DLStringInner { bytes: [0x0u16; 0x8] },
            length: 0,
            capacity: 7,
        };

        assert_eq!(dlstring.as_string(), String::new());

        // Put a fucking A\0 in there
        let new_string: [u16; 2] = [0x41, 0x00];
        unsafe { dlstring.set_contents(&new_string) };

        assert_eq!(dlstring.as_string(), String::from("A"));
        assert_eq!(dlstring.length, 1); // Length of the string without null terminator.
        assert_eq!(dlstring.capacity, 7); // Length of the alloc without null terminator.
    }
}
