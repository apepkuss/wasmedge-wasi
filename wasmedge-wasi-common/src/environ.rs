use crate::dir::{DirCaps, DirEntry, WasiDir};
use crate::error::Error;
use crate::file::{FileCaps, FileEntry, WasiFile};
use crate::string_array::{StringArray, StringArrayError};
use crate::table::Table;
use crate::WasiSnapshotPreview1;
use crate::{Ciovec, CiovecArray};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub struct WasiEnviron {
    pub args: StringArray,
    pub env: StringArray,
    pub table: Table,
    pub exit_code: i32,
}
impl WasiEnviron {
    pub fn new() -> Self {
        let mut environ = WasiEnviron {
            args: StringArray::new(),
            env: StringArray::new(),
            table: Table::new(),
            exit_code: 0,
        };

        environ.set_stdin(Box::new(crate::pipe::ReadPipe::new(std::io::empty())));
        environ.set_stdout(Box::new(crate::pipe::WritePipe::new(std::io::sink())));
        environ.set_stderr(Box::new(crate::pipe::WritePipe::new(std::io::sink())));

        environ
    }

    pub fn push_arg(&mut self, arg: &str) -> Result<(), StringArrayError> {
        self.args.push(arg.to_owned())
    }

    pub fn push_env(&mut self, var: &str, value: &str) -> Result<(), StringArrayError> {
        self.env.push(format!("{}={}", var, value))
    }

    pub fn push_preopened_dir(
        &mut self,
        dir: Box<dyn WasiDir>,
        path: impl AsRef<Path>,
    ) -> Result<(), Error> {
        let caps = DirCaps::all();
        let file_caps = FileCaps::all();
        self.table().push(Box::new(DirEntry::new(
            caps,
            file_caps,
            Some(path.as_ref().to_owned()),
            dir,
        )))?;
        Ok(())
    }

    pub fn set_stdin(&mut self, mut f: Box<dyn WasiFile>) {
        let rights = Self::stdio_rights(&mut *f);
        self.insert_file(0, f, rights);
    }

    pub fn set_stdout(&mut self, mut f: Box<dyn WasiFile>) {
        let rights = Self::stdio_rights(&mut *f);
        self.insert_file(1, f, rights);
    }

    pub fn set_stderr(&mut self, mut f: Box<dyn WasiFile>) {
        let rights = Self::stdio_rights(&mut *f);
        self.insert_file(2, f, rights);
    }

    fn stdio_rights(f: &mut dyn WasiFile) -> FileCaps {
        let mut rights = FileCaps::all();

        // If `f` is a tty, restrict the `tell` and `seek` capabilities, so
        // that wasi-libc's `isatty` correctly detects the file descriptor
        // as a tty.
        if f.isatty() {
            rights &= !(FileCaps::TELL | FileCaps::SEEK);
        }

        rights
    }

    pub fn insert_file(&mut self, fd: u32, file: Box<dyn WasiFile>, caps: FileCaps) {
        self.table()
            .insert_at(fd, Box::new(FileEntry::new(caps, file)));
    }

    pub fn push_file(&mut self, file: Box<dyn WasiFile>, caps: FileCaps) -> Result<u32, Error> {
        self.table().push(Box::new(FileEntry::new(caps, file)))
    }

    pub fn table(&mut self) -> &mut Table {
        &mut self.table
    }

    pub fn insert_dir(
        &mut self,
        fd: u32,
        dir: Box<dyn WasiDir>,
        caps: DirCaps,
        file_caps: FileCaps,
        path: PathBuf,
    ) {
        self.table().insert_at(
            fd,
            Box::new(DirEntry::new(caps, file_caps, Some(path), dir)),
        );
    }

    pub fn push_dir(
        &mut self,
        dir: Box<dyn WasiDir>,
        caps: DirCaps,
        file_caps: FileCaps,
        path: PathBuf,
    ) -> Result<u32, Error> {
        self.table()
            .push(Box::new(DirEntry::new(caps, file_caps, Some(path), dir)))
    }
}
impl WasiSnapshotPreview1 for WasiEnviron {
    fn args_sizes_get(&self) -> (i32, i32) {
        println!("in WasiEnviron::args_sizes_get");
        (
            self.args.number_elements() as i32,
            self.args.cumulative_size() as i32,
        )
    }

    fn args_get(&self, out: &mut Vec<Ciovec>) {
        println!("in WasiEnviron::args_get");
        for arg in self.args.elements() {
            let iov = Ciovec {
                buf: arg.as_ptr(),
                buf_len: arg.as_bytes().len(),
            };
            out.push(iov);
        }
    }

    fn environ_sizes_get(&self) -> (i32, i32) {
        println!("in WasiEnviron::environ_sizes_get");
        (
            self.env.number_elements() as i32,
            self.env.cumulative_size() as i32,
        )
    }

    fn environ_get(&self, out: &mut Vec<Ciovec>) {
        println!("in WasiEnviron::environ_get");
        for env in self.env.elements() {
            let iov = Ciovec {
                buf: env.as_ptr(),
                buf_len: env.as_bytes().len(),
            };
            out.push(iov);
        }
    }

    fn fd_write(&mut self, fd: i32, iovs: CiovecArray) -> i32 {
        println!("in WasiEnviron::fd_write");

        let mut io_slice_vec = vec![];
        for iov in iovs {
            let buf: &[u8] = unsafe { std::slice::from_raw_parts(iov.buf, iov.buf_len) };
            let io_slice = std::io::IoSlice::new(buf);
            io_slice_vec.push(io_slice);
        }

        let mut buffer = std::fs::File::create("foo.txt").expect("failed to create file");
        let nwritten = buffer
            .write_vectored(&io_slice_vec)
            .expect("failed to write to file");

        nwritten as i32
    }

    fn proc_exit(&mut self, code: i32) {
        println!("in WasiEnviron::proc_exit");

        println!("code: {}", code);
        self.exit_code = code;
        println!("exit_code: {}", self.exit_code);
    }
}
