use super::imports::*;
use std::{
    io::{self, IoSlice, IoSliceMut},
    marker::PhantomData,
    mem::ManuallyDrop,
};
use to_method::To;

pub(super) struct FdOps(pub(super) c_int, PhantomData<*mut ()>);
impl FdOps {
    pub fn new(fd: c_int) -> Self {
        Self(fd, PhantomData)
    }
    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let (success, bytes_read) = unsafe {
            let length_to_read = buf.len();
            let size_or_err =
                libc::read(self.as_raw_fd(), buf.as_mut_ptr() as *mut _, length_to_read);
            (size_or_err >= 0, size_or_err as usize)
        };
        if success {
            Ok(bytes_read)
        } else {
            Err(io::Error::last_os_error())
        }
    }
    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        let (success, bytes_read) = unsafe {
            let num_bufs = bufs.len().try_to::<c_int>().unwrap_or(c_int::MAX);
            let size_or_err =
                libc::readv(self.as_raw_fd(), bufs.as_mut_ptr() as *const _, num_bufs);
            (size_or_err >= 0, size_or_err as usize)
        };
        if success {
            Ok(bytes_read)
        } else {
            Err(io::Error::last_os_error())
        }
    }
    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let (success, bytes_written) = unsafe {
            let length_to_write = buf.len();
            let size_or_err =
                libc::write(self.as_raw_fd(), buf.as_ptr() as *const _, length_to_write);
            (size_or_err >= 0, size_or_err as usize)
        };
        if success {
            Ok(bytes_written as usize)
        } else {
            Err(io::Error::last_os_error())
        }
    }
    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        let (success, bytes_written) = unsafe {
            let num_bufs = bufs.len().try_to::<c_int>().unwrap_or(c_int::MAX);
            let size_or_err = libc::writev(self.as_raw_fd(), bufs.as_ptr() as *const _, num_bufs);
            (size_or_err >= 0, size_or_err as usize)
        };
        if success {
            Ok(bytes_written)
        } else {
            Err(io::Error::last_os_error())
        }
    }
    pub fn flush(&self) -> io::Result<()> {
        let success = unsafe { libc::fsync(self.as_raw_fd()) >= 0 };
        if success {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}
impl AsRawFd for FdOps {
    fn as_raw_fd(&self) -> c_int {
        self.0
    }
}
impl IntoRawFd for FdOps {
    fn into_raw_fd(self) -> c_int {
        let self_ = ManuallyDrop::new(self);
        self_.as_raw_fd()
    }
}
impl FromRawFd for FdOps {
    unsafe fn from_raw_fd(fd: c_int) -> Self {
        Self::new(fd)
    }
}
impl Drop for FdOps {
    fn drop(&mut self) {
        unsafe { close_fd(self.0) };
    }
}
unsafe impl Send for FdOps {
}
unsafe impl Sync for FdOps {
}

pub(super) unsafe fn close_fd(fd: i32) {
    let success = unsafe {
        let mut success = true;
        // If the close() call fails, the loop starts and keeps retrying until either
        // the error value isn't Interrupted (in which case the assertion fails)
        // or the close operation properly fails with a non-Interrupted error
        // type. Why does Unix even have this idiotic error type?
        while libc::close(fd) != 0 {
            if io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
                // An actual close error happened — return early now
                success = false;
                break;
            }
        }
        success
    };
    debug_assert!(success);
}
/// Captures [`io::Error::last_os_error()`] and closes the file descriptor.
pub(super) unsafe fn handle_fd_error(fd: i32) -> io::Error {
    let e = io::Error::last_os_error();
    unsafe { close_fd(fd) };
    e
}
pub(super) unsafe fn close_by_error(socket: i32) -> impl FnOnce(io::Error) -> io::Error {
    move |e| {
        unsafe { close_fd(socket) };
        e
    }
}
