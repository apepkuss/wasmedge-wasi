pub mod clocks;
pub mod dir;
pub mod environ;
pub mod error;
pub mod file;
pub mod pipe;
pub mod string_array;
pub mod table;

pub use error::{Context, Error, ErrorExt, ErrorKind};

pub trait WasiSnapshotPreview1 {
    /// Return the number of command-line arguments and the size of the command-line argument data.
    fn args_sizes_get(&self) -> (i32, i32);

    /// Read command-line argument data.
    /// The size of the array should match that returned by `args_sizes_get`.
    /// Each argument is expected to be `\0` terminated.
    fn args_get(&self, out: &mut Vec<Ciovec>);

    /// Return the number of environment variable pairs and the total size of the environment variable data.
    fn environ_sizes_get(&self) -> (i32, i32);

    /// Read environment variable data.
    /// The sizes of the buffers should match that returned by `environ_sizes_get`.
    /// Key/value pairs are expected to be joined with `=`s, and terminated with `\0`s.
    fn environ_get(&self, out: &mut Vec<Ciovec>);

    /// Write data described by `iovs` to the file associated with the file descriptor `fd`.
    ///
    /// Return the number of bytes written.
    fn fd_write(&mut self, fd: i32, iovs: CiovecArray) -> i32;

    /// Terminate the process normally. An exit code of 0 indicates successful
    /// termination of the program. The meanings of other values is dependent on
    /// the environment.
    fn proc_exit(&mut self, code: i32);
}

pub type Size = usize;
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Ciovec {
    /// The address of the buffer to be written.
    pub buf: *const u8,
    /// The length of the buffer to be written.
    pub buf_len: Size,
}
pub type CiovecArray<'a> = &'a [Ciovec];
