use fs_set_times::SetTimes;
use io_lifetimes::AsFilelike;
use is_terminal::IsTerminal;
use std::{
    any::Any,
    fs::File,
    io::{self, Read, Write},
};
use system_interface::io::ReadReady;
use wasmedge_wasi_common::{
    clocks::SystemTimeSpec,
    error::{Error, ErrorExt},
    file::{FdFlags, FileType, WasiFile},
};

#[cfg(unix)]
use io_lifetimes::{AsFd, BorrowedFd};

pub fn stdin() -> Stdin {
    Stdin(std::io::stdin())
}

pub struct Stdin(std::io::Stdin);
impl WasiFile for Stdin {
    fn as_any(&self) -> &dyn Any {
        self
    }
    #[cfg(unix)]
    fn pollable(&self) -> Option<rustix::fd::BorrowedFd> {
        Some(self.0.as_fd())
    }
    fn get_filetype(&mut self) -> Result<FileType, Error> {
        if self.isatty() {
            Ok(FileType::CharacterDevice)
        } else {
            Ok(FileType::Unknown)
        }
    }
    fn read_vectored<'a>(&mut self, bufs: &mut [io::IoSliceMut<'a>]) -> Result<u64, Error> {
        let n = (&*self.0.as_filelike_view::<File>()).read_vectored(bufs)?;
        Ok(n.try_into().map_err(|_| Error::range())?)
    }
    fn read_vectored_at<'a>(
        &mut self,
        _bufs: &mut [io::IoSliceMut<'a>],
        _offset: u64,
    ) -> Result<u64, Error> {
        Err(Error::seek_pipe())
    }
    fn seek(&mut self, _pos: std::io::SeekFrom) -> Result<u64, Error> {
        Err(Error::seek_pipe())
    }
    fn peek(&mut self, _buf: &mut [u8]) -> Result<u64, Error> {
        Err(Error::seek_pipe())
    }
    fn set_times(
        &mut self,
        atime: Option<SystemTimeSpec>,
        mtime: Option<SystemTimeSpec>,
    ) -> Result<(), Error> {
        self.0.set_times(
            crate::file::convert_systimespec(atime),
            crate::file::convert_systimespec(mtime),
        )?;
        Ok(())
    }
    fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(self.0.num_ready_bytes()?)
    }
    fn isatty(&mut self) -> bool {
        self.0.is_terminal()
    }
}
#[cfg(unix)]
impl AsFd for Stdin {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

macro_rules! wasi_file_write_impl {
    ($ty:ty, $ident:ident) => {
        impl WasiFile for $ty {
            fn as_any(&self) -> &dyn Any {
                self
            }
            #[cfg(unix)]
            fn pollable(&self) -> Option<rustix::fd::BorrowedFd> {
                Some(self.0.as_fd())
            }
            fn get_filetype(&mut self) -> Result<FileType, Error> {
                if self.isatty() {
                    Ok(FileType::CharacterDevice)
                } else {
                    Ok(FileType::Unknown)
                }
            }
            fn get_fdflags(&mut self) -> Result<FdFlags, Error> {
                Ok(FdFlags::APPEND)
            }
            fn write_vectored<'a>(&mut self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
                let n = (&*self.0.as_filelike_view::<File>()).write_vectored(bufs)?;
                Ok(n.try_into().map_err(|c| Error::range().context(c))?)
            }
            fn write_vectored_at<'a>(
                &mut self,
                _bufs: &[io::IoSlice<'a>],
                _offset: u64,
            ) -> Result<u64, Error> {
                Err(Error::seek_pipe())
            }
            fn seek(&mut self, _pos: std::io::SeekFrom) -> Result<u64, Error> {
                Err(Error::seek_pipe())
            }
            fn set_times(
                &mut self,
                atime: Option<SystemTimeSpec>,
                mtime: Option<SystemTimeSpec>,
            ) -> Result<(), Error> {
                self.0.set_times(
                    crate::file::convert_systimespec(atime),
                    crate::file::convert_systimespec(mtime),
                )?;
                Ok(())
            }
            fn isatty(&mut self) -> bool {
                self.0.is_terminal()
            }
        }
        #[cfg(unix)]
        impl AsFd for $ty {
            fn as_fd(&self) -> BorrowedFd<'_> {
                self.0.as_fd()
            }
        }
    };
}

pub fn stdout() -> Stdout {
    Stdout(std::io::stdout())
}

pub struct Stdout(std::io::Stdout);
wasi_file_write_impl!(Stdout, Stdout);

pub fn stderr() -> Stderr {
    Stderr(std::io::stderr())
}

pub struct Stderr(std::io::Stderr);
wasi_file_write_impl!(Stderr, Stderr);
