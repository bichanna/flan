use std::alloc::{alloc, dealloc, Layout};

#[derive(Clone, Copy)]
pub struct Object {
    pub layout: Layout,
    pub ptr: *mut u8,
    pub marked: bool,
}

pub struct Heap {
    /// Linked list of objects
    objects: Vec<Object>,
    /// How many bytes are allocated on the heap
    bytes_allocated: usize,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            objects: Vec::with_capacity(12),
            bytes_allocated: 0,
        }
    }

    /// Allocates a new object on the heap
    pub fn allocate<T>(&mut self, val: T) -> Object {
        unsafe {
            let layout = Layout::new::<T>();
            let ptr = alloc(layout);

            *(ptr as *mut T) = val;

            let obj = Object {
                layout,
                ptr,
                marked: false,
            };

            self.bytes_allocated += layout.size();
            self.objects.push(obj);

            obj
        }
    }

    /// Deallocates all objects
    pub fn deallocate_all(&mut self) {
        for obj in &self.objects {
            unsafe {
                dealloc(obj.ptr, obj.layout);
            }
        }
    }

    /// Deallocates the given object
    fn deallocate(obj: Object) {
        unsafe {
            dealloc(obj.ptr, obj.layout);
        }
    }
}
