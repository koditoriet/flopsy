use std::ptr::null_mut;
use libc::{c_int, SPLICE_F_MOVE, __errno_location, O_NONBLOCK, SPLICE_F_NONBLOCK, EAGAIN, EWOULDBLOCK};

pub(crate) struct Pipe {
    pub read_fd: i32,
    pub write_fd: i32
}

pub(crate) enum Error { EAgain, Other(i32) }
type Result<T> = std::result::Result<T, Error>;

impl Pipe {
    pub fn new() -> Pipe {
        let mut pipes = std::mem::MaybeUninit::<[c_int; 2]>::uninit();
        unsafe {
            libc::pipe2(pipes.as_mut_ptr() as *mut c_int, O_NONBLOCK);
            Pipe {
                read_fd: pipes.assume_init()[0],
                write_fd: pipes.assume_init()[1]
            }
        }
    }

    #[inline(always)]
    pub fn splice_from(&self, src_fd: i32, len: usize) -> Result<usize> {
        splice(src_fd, self.write_fd, len)
    }

    #[inline(always)]
    pub fn splice_into(&self, dst_fd: i32, len: usize) -> Result<usize> {
        splice(self.read_fd, dst_fd, len)
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.read_fd);
            libc::close(self.write_fd);
        }
    }
}

#[inline(always)]
fn splice(src_fd: i32, dst_fd: i32, len: usize) -> Result<usize> {
    unsafe {
        let bytes_copied = libc::splice(
            src_fd,
            null_mut(),
            dst_fd,
            null_mut(),
            len,
            SPLICE_F_MOVE | SPLICE_F_NONBLOCK
        );
        if bytes_copied < 0 {
            let errno = *__errno_location();
            if errno == EAGAIN || errno == EWOULDBLOCK {
                return Err(Error::EAgain)
            } else {
                return Err(Error::Other(errno))
            }
        } else {
            return Ok(bytes_copied as usize)
        }
    }
}