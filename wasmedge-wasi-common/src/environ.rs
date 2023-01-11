use crate::dir::{DirCaps, DirEntry, WasiDir};
use crate::error::Error;
use crate::file::{FileCaps, FileEntry, WasiFile};
use crate::string_array::{StringArray, StringArrayError};
use crate::table::Table;
use std::path::{Path, PathBuf};

pub struct WasiEnviron {
    pub args: StringArray,
    pub env: StringArray,
    pub table: Table,
}
impl WasiEnviron {
    pub fn new() -> Self {
        let mut environ = WasiEnviron {
            args: StringArray::new(),
            env: StringArray::new(),
            table: Table::new(),
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
        self.env.push(format!("{}={}", var, value))?;
        Ok(())
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
