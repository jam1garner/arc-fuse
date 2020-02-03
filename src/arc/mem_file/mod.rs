use std::slice;
use std::ops::Deref;
use std::marker::PhantomData;
use std::mem::{size_of, transmute};
use std::borrow::Borrow;
use std::sync::RwLock;
use std::ops::Add;

#[cfg(test)]
mod test;

pub trait Num:  Copy + IntoUsize + Add<Output=Self> + Sized {}
impl<T> Num for T where T: Copy + IntoUsize + Add<Output=Self> + Sized {}

lazy_static::lazy_static! {
    pub static ref FILE: RwLock<Option<&'static [u8]>> = RwLock::new(None);
}

pub fn set_file(file: &[u8]) {
    *FILE.write().unwrap() = Some(unsafe { transmute(file) });
}

pub fn get_header<T: Sized>() -> FilePtr<usize, T> {
    FilePtr::new(0)
}

pub fn get_footer<T: Sized>() -> FilePtr<usize, T> {
    let file = (*FILE.read().unwrap()).unwrap();
    FilePtr::new(file.len() - size_of::<T>())
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct FilePtr<P: Num, T: Sized>(P, PhantomData<T>);

pub type FilePtr8<T> = FilePtr<u8, T>;
pub type FilePtr16<T> = FilePtr<u16, T>;
pub type FilePtr32<T> = FilePtr<u32, T>;
pub type FilePtr64<T> = FilePtr<u64, T>;

#[derive(Clone, Copy)]
pub struct FileSlice<T: Sized>(usize, usize, PhantomData<[T]>);

impl<P: Num, T> FilePtr<P, T> {
    pub fn inner(&self) -> P {
        self.0
    }

    pub fn offset(&self, amt: P) -> FilePtr<P, T> {
        FilePtr(self.0 + amt, PhantomData)
    }

    pub fn slice(&self, size: usize) -> FileSlice<T> {
        FileSlice(self.0.into(), size, PhantomData)
    }

    pub fn next<U: Sized>(&self) -> FilePtr<usize, U> {
        FilePtr(self.0.into() + size_of::<T>(), PhantomData)
    }

    pub fn next_slice<U: Sized>(&self, size: usize) -> FileSlice<U> {
        FileSlice(self.0.into() + size_of::<T>(), size, PhantomData)
    }

    pub fn new(ptr: P) -> Self {
        FilePtr(ptr, PhantomData)
    }
}

impl<T> FileSlice<T> {
    pub fn inner_ptr(&self) -> usize {
        self.0
    }

    pub fn len(&self) -> usize {
        self.1
    }

    pub fn as_file_ptr(&self) -> FilePtr<usize, T> {
        FilePtr(self.0, PhantomData)
    }

    pub fn next<U: Sized>(&self) -> FilePtr<usize, U> {
        FilePtr(self.0 + (size_of::<T>() * self.1), PhantomData)
    }

    pub fn next_slice<U: Sized>(&self, size: usize) -> FileSlice<U> {
        FileSlice(self.0 + (size_of::<T>() * self.1), size, PhantomData)
    }

    pub fn new(ptr: usize, size: usize) -> Self {
        FileSlice(ptr, size, PhantomData)
    }
}

impl<P: Num, T: Sized> Borrow<T> for FilePtr<P, T> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: Sized> Borrow<[T]> for FileSlice<T> {
    fn borrow(&self) -> &[T] {
        &**self
    }
}

impl<P: Num, T: Sized> Deref for FilePtr<P, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let file = (*FILE.read().unwrap()).unwrap();

        if self.0.into() + size_of::<T>() > file.len() {
            panic!(
                "Out of bounds read 0x{:X} size 0x{:X} > file size 0x{:X}",
                self.0.into(),
                size_of::<T>(),
                file.len()
            );
        }

        unsafe {
            transmute(&*file.as_ptr().offset(self.0.into() as isize))
        }
    }
}

impl<T: Sized> Deref for FileSlice<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        let file = (*FILE.read().unwrap()).unwrap();

        if self.0 + (size_of::<T>() + self.1) > file.len() {
            panic!(
                "Out of bounds read 0x{:X} size 0x{:X} > file size 0x{:X}",
                self.0,
                size_of::<T>(),
                file.len()
            );
        }

        unsafe {
            &slice::from_raw_parts(file.as_ptr().offset(self.0 as isize) as _, self.1)
        }
    }
}

impl<P: Num, T: Sized> std::fmt::Debug for FilePtr<P, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "FilePtr(0x{:X})", self.0.into())
    }
}

impl<T: Sized> std::fmt::Debug for FileSlice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "FileSlice(0x{:X}, len={})", self.0, self.1)
    }
}

impl<P: Num, T: Sized + PartialEq> PartialEq<T> for FilePtr<P, T> {
    fn eq(&self, rhs: &T) -> bool {
        let x: &T = &*self;
        x == rhs
    }
}

impl<T: Sized + PartialEq> PartialEq<[T]> for FileSlice<T> {
    fn eq(&self, rhs: &[T]) -> bool {
        let x: &[T] = &*self;
        x == rhs
    }
}

pub trait IntoUsize {
    fn into(self) -> usize;
}

macro_rules! impl_into_usize {
    ($($t:ty),*) => {
        $(
            impl IntoUsize for $t {
                fn into(self) -> usize {
                    self as usize
                }
            }
        )*
    };
}

impl_into_usize!(u8, u16, u32, u64, usize);

impl<P: Num, T: Sized> Into<usize> for FilePtr<P, T> {
    fn into(self) -> usize {
        self.inner().into()
    }
}
