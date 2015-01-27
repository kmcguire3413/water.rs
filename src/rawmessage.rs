use std::rt::heap::allocate;
use std::rt::heap::deallocate;
use std::mem::size_of;
use std::sync::Mutex;
use std::intrinsics::transmute;
use std::mem::transmute_copy;
use std::mem::uninitialized;
use std::mem::forget;
use std::intrinsics::copy_memory;
use std::raw;
use std::sync::Arc;
use std::sync::StaticMutex;
use std::sync::MUTEX_INIT;
use std::thread::Thread;
use std;

/// This is used to signify that a type is safe to
/// be written and read from a RawMessage across
/// process and machine boundaries. This trait can
/// create dangerous code.
pub trait NoPointers { }

struct Internal {
    len:            usize,
    cap:            usize,
    buf:            *mut u8,
}

unsafe impl Send for Internal{}

fn hextype<T>(t: &T) -> String {
    unsafe {
        let p: usize = transmute(t);
        let mut s = String::new();
        for i in range(0, size_of::<T>()) {
            s.push_str(format!("{:02x}", *((p + i) as *const u8)).as_slice());
        }
        s
    }
}

impl Drop for Internal {
    fn drop(&mut self) {
        let cthread = Thread::current();

        unsafe { deallocate(self.buf, self.cap, size_of::<usize>()); }
    }
}

impl Internal {
    fn new(mut cap: usize) -> Internal {
        // This helps let sync and clone messages work. Since sometimes
        // they may use a zero size type just for conveying a signal of
        // some sort and the will try to allocate a message of zero bytes
        // which will result in undefined behavior.
        if cap == 0 {
            cap = 1;
        }

        // TODO: do we need to zero out the memory allocates?
        let i = Internal {
            len:        cap,
            cap:        cap,
            buf:        unsafe { allocate(cap, size_of::<usize>()) },
        };

        i
    }

    fn id(&self) -> usize {
        unsafe { transmute(self) }
    }

    fn dup(&self) -> Internal {
        let i = Internal {
            len:        self.cap,
            cap:        self.cap,
            buf:        unsafe { allocate(self.cap, size_of::<usize>() ) },
        };

        unsafe { 
            copy_memory::<u8>(i.buf, self.buf, self.cap); 
        }

        i
    }

    fn setlen(&mut self, len: usize) {
        if len > self.cap {
            panic!("len:{} must be <= cap:{}", len, self.cap);
        }

        self.len = len;
    }

    fn resize(&mut self, newcap: usize) {
        unsafe {
            let nbuf = allocate(newcap, size_of::<usize>());

            if newcap <= self.cap {
                copy_memory(nbuf, self.buf, self.cap);
            } else {
                copy_memory(nbuf, self.buf, newcap);
            }

            deallocate(self.buf, self.cap, size_of::<usize>());
            self.buf = nbuf;

            self.cap = newcap;

            if self.len > self.cap {
                self.len = self.cap;
            }
        }
    }
}

pub struct RawMessage {
    i:              Arc<Mutex<Internal>>,   
}

unsafe impl Send for RawMessage { }

impl Clone for RawMessage {
    /// Duplicate the raw message creating a new one that _shares the buffer_.
    fn clone(&self) -> RawMessage {
        RawMessage {
            i:          self.i.clone(),
        }
    }
}

impl RawMessage {
    /// Create a new raw message with the specified capacity.
    pub fn new(cap: usize) -> RawMessage {
        RawMessage {i:  Arc::new(Mutex::new(Internal::new(cap)))}
    }

    /// Duplicate the raw message creating a new one not sharing the buffer.
    pub fn dup(&self) -> RawMessage {
        let i = self.i.lock().unwrap().dup();
        let rm = RawMessage {i:  Arc::new(Mutex::new(i))};
        rm
    }

    pub fn id(&self) -> usize {
        unsafe { transmute(self) }
    }

    /// Create a raw message from a &str type.
    ///
    /// `let rmsg = RawMessage:new_fromstr("Hello World");`
    pub fn new_fromstr(s: &str) -> RawMessage {
        let m = RawMessage::new(s.len());
        unsafe {
            copy_memory(m.i.lock().unwrap().buf, *(transmute::<&&str, *const usize>(&s)) as *const u8, s.len());
        }
        m
    }

    /// Get the capacity.
    pub fn cap(&self) -> usize {
        /*unsafe {
            let m: &Mutex<Internal> = &*self.i;
            let ptr: *mut usize = transmute(&*self.i);
            let ptr: usize = *ptr;
            *((ptr + 8 * 2) as *mut usize) = 0;
            std::io::stdio::stdout_raw().write(
            format!("@trying to lock\nlock:{:x}\ncount:{:x}\nowner:{:x}\nnusers:{:x}\n",
                *((ptr + 4 * 0) as *mut u32),
                *((ptr + 4 * 1) as *mut u32),
                *((ptr + 4 * 2) as *mut u32),
                *((ptr + 4 * 3) as *mut u32),
            ).as_bytes());
        }*/
        let i = self.i.lock().unwrap();
        /*unsafe {
            let m: &Mutex<Internal> = &*self.i;
            let ptr: *mut usize = transmute(&*self.i);
            let ptr: usize = *ptr;
            std::io::stdio::stdout_raw().write(
            format!("@locked\nlock:{:x}\ncount:{:x}\nowner:{:x}\nnusers:{:x}\n",
                *((ptr + 4 * 0) as *mut u32),
                *((ptr + 4 * 1) as *mut u32),
                *((ptr + 4 * 2) as *mut u32),
                *((ptr + 4 * 3) as *mut u32),
            ).as_bytes());
        } */       
        //println!("@{:p}", &*i);
        let cap = i.cap;
        drop(i);
        //println!("got cap");
        return cap;
    }

    /// Set the length.
    ///
    /// The length must not exceed the capacity or it will panic.
    pub fn setlen(&mut self, len: usize) {
        self.i.lock().unwrap().setlen(len);
    }

    /// Get the length.
    pub fn len(&self) -> usize {
        self.i.lock().unwrap().len
    }

    /// Resize the capacity of the message keeping the old contents or truncating them.
    pub fn resize(&mut self, newcap: usize) {
        self.i.lock().unwrap().resize(newcap);
    }

    /// Write into the buffer from a byte slice.
    pub fn write_from_slice(&mut self, mut offset: usize, f: &[u8]) {
        let mut i = self.i.lock().unwrap();

        if offset + f.len() > i.cap {
            panic!("write past end of buffer");
        }

        for item in f.iter() {
            unsafe { *((i.buf as usize + offset) as *mut u8) = *item };
            offset += 1;
        }

        if offset > i.len {
            i.len = offset;
        }
    }

    /// Return a reference to a mutable byte slice that can be used to alter the contents.
    pub fn as_mutslice(&mut self) -> &mut [u8] {
        unsafe {
            let i = self.i.lock().unwrap();
            transmute(raw::Slice { data: i.buf as *const u8, len: i.len })
        }        
    }

    /// Return a reference to a immutable byte slice.
    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            let i = self.i.lock().unwrap();
            transmute(raw::Slice { data: i.buf as *const u8, len: i.len })
        }
    }

    /// Safely write a structure/value by reference into the buffer at the specified offet.
    ///
    /// The offset must not exceed the capacity. The length is not updated.
    pub fn writestructref<T>(&mut self, offset: usize, t: &T) {
        let i = self.i.lock().unwrap();

        if offset + size_of::<T>() > i.cap {
            panic!("write past end of buffer! {}:{}", offset + size_of::<T>(), i.cap);
        }

        if size_of::<T>() == 0 {
            return;
        }        

        unsafe { copy_memory::<T>((i.buf as usize + offset) as *mut T, transmute(t), 1); }        
    }

    /// Safely write a structure/value by consuming it, and _DO NOT_ drop the value.
    pub fn writestruct<T>(&mut self, offset: usize, t: T) {
        self.writestructref(offset, &t);
        unsafe { forget(t) }; 
    }

    /// Read a structure only if it is marked as safe and return the value.
    ///
    /// This function is only intended to help keep you from having to use
    /// unsafe code blocks. You can easily mark a structure as safe and
    /// crash your program.
    pub fn readstruct<T: NoPointers>(&self, offset: usize) -> T {
        unsafe { self.readstructunsafe(offset) }
    }

    /// Read a structure only if it is marked as safe into a mutable reference.
    pub fn readstructref<T: NoPointers>(&self, offset: usize, t: &mut T) {
        unsafe { self.readstructunsaferef(offset, t) };
    }

    /// This is the same as readstruct except it is an unsafe call.
    pub unsafe fn readstructunsafe<T>(&self, offset: usize) -> T {
        let mut out: T = uninitialized::<T>();
        self.readstructunsaferef(offset, &mut out);
        out
    }

    /// This is the same as readstructref except it is an unsafe call.
    pub unsafe fn readstructunsaferef<T>(&self, offset: usize, t: &mut T) {
        let i = self.i.lock().unwrap();

        if offset + size_of::<T>() > i.cap {
            panic!("read past end of buffer!")
        }

        copy_memory(t, (i.buf as usize + offset) as *const T, 1);
    }

    /// Write a unsigned 8-bit value at the specified offset.
    pub fn writeu8(&mut self, offset: usize, value: u8) { self.writestruct(offset, value); }
    /// Write a unsigned 16-bit value at the specified offset.
    pub fn writeu16(&mut self, offset: usize, value: u16) { self.writestruct(offset, value); }
    /// Write a unsigned 32-bit value at the specified offset.
    pub fn writeu32(&mut self, offset: usize, value: u32) { self.writestruct(offset, value); }
    /// Write a signed 8-bit value at the specified offset.
    pub fn writei8(&mut self, offset: usize, value: i8) { self.writestruct(offset, value); }
    /// Write a signed 16-bit value at the specified offset.
    pub fn writei16(&mut self, offset: usize, value: i16) { self.writestruct(offset, value); }
    /// Write a signed 32-bit value at the specified offset.
    pub fn writei32(&mut self, offset: usize, value: i32) { self.writestruct(offset, value); }
    /// Read a unsigned 8-bit value at the specified offset.
    pub fn readu8(&self, offset: usize) -> u8 { unsafe { self.readstructunsafe(offset) } }
    /// Read a unsigned 16-bit value at the specified offset.
    pub fn readu16(&self, offset: usize) -> u16 { unsafe { self.readstructunsafe(offset) } }
    /// Read a unsigned 32-bit value at the specified offset.
    pub fn readu32(&self, offset: usize) -> u32 { unsafe { self.readstructunsafe(offset) } }
    /// Read a signed 8-bit value at the specified offset.
    pub fn readi8(&self, offset: usize) -> i8 { unsafe { self.readstructunsafe(offset) } }
    /// Read a signed 16-bit value at the specified offset.
    pub fn readi16(&self, offset: usize) -> i16 { unsafe { self.readstructunsafe(offset) } }
    /// Read a signed 32-bit value at the specified offset.
    pub fn readi32(&self, offset: usize) -> i32 { unsafe { self.readstructunsafe(offset) } }
}
