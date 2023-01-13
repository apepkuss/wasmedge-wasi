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

    /// Return the number of environment variable pairs and the total size of the environment variable data.
    fn environ_sizes_get(&self) -> (i32, i32);

    /// Write data described by `iovs` to the file associated with the file descriptor `fd`.
    ///
    /// Return the number of bytes written.
    fn fd_write(&mut self, fd: i32, iovs: &[std::io::IoSlice]) -> i32;

    /// Terminate the process normally. An exit code of 0 indicates successful
    /// termination of the program. The meanings of other values is dependent on
    /// the environment.
    fn proc_exit(&mut self, code: i32);
}
