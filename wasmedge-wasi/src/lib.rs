pub mod dir;
pub mod file;
pub mod net;
pub mod stdio;

use crate::net::Socket;
use std::path::Path;
use wasmedge_wasi_common::{
    environ::WasiEnviron,
    error::Error,
    file::{FileCaps, WasiFile},
    string_array::StringArrayError,
};

pub struct WasiCtxBuilder(WasiEnviron);

impl WasiCtxBuilder {
    pub fn new() -> Self {
        WasiCtxBuilder(WasiEnviron::new())
    }
    pub fn env(mut self, var: &str, value: &str) -> Result<Self, StringArrayError> {
        self.0.push_env(var, value)?;
        Ok(self)
    }
    pub fn envs(mut self, env: &[(String, String)]) -> Result<Self, StringArrayError> {
        for (k, v) in env {
            self.0.push_env(k, v)?;
        }
        Ok(self)
    }
    pub fn inherit_env(mut self) -> Result<Self, StringArrayError> {
        for (key, value) in std::env::vars() {
            self.0.push_env(&key, &value)?;
        }
        Ok(self)
    }
    pub fn arg(mut self, arg: &str) -> Result<Self, StringArrayError> {
        self.0.push_arg(arg)?;
        Ok(self)
    }
    pub fn args(mut self, arg: &[String]) -> Result<Self, StringArrayError> {
        for a in arg {
            self.0.push_arg(&a)?;
        }
        Ok(self)
    }
    pub fn inherit_args(mut self) -> Result<Self, StringArrayError> {
        for arg in std::env::args() {
            self.0.push_arg(&arg)?;
        }
        Ok(self)
    }
    pub fn stdin(mut self, f: Box<dyn WasiFile>) -> Self {
        self.0.set_stdin(f);
        self
    }
    pub fn stdout(mut self, f: Box<dyn WasiFile>) -> Self {
        self.0.set_stdout(f);
        self
    }
    pub fn stderr(mut self, f: Box<dyn WasiFile>) -> Self {
        self.0.set_stderr(f);
        self
    }
    pub fn inherit_stdin(self) -> Self {
        self.stdin(Box::new(crate::stdio::stdin()))
    }
    pub fn inherit_stdout(self) -> Self {
        self.stdout(Box::new(crate::stdio::stdout()))
    }
    pub fn inherit_stderr(self) -> Self {
        self.stderr(Box::new(crate::stdio::stderr()))
    }
    pub fn inherit_stdio(self) -> Self {
        self.inherit_stdin().inherit_stdout().inherit_stderr()
    }
    pub fn preopened_dir(
        mut self,
        dir: cap_std::fs::Dir,
        guest_path: impl AsRef<Path>,
    ) -> Result<Self, Error> {
        let dir = Box::new(crate::dir::Dir::from_cap_std(dir));
        self.0.push_preopened_dir(dir, guest_path)?;
        Ok(self)
    }
    pub fn preopened_socket(mut self, fd: u32, socket: impl Into<Socket>) -> Result<Self, Error> {
        let socket: Socket = socket.into();
        let file: Box<dyn WasiFile> = socket.into();

        let caps = FileCaps::FDSTAT_SET_FLAGS
            | FileCaps::FILESTAT_GET
            | FileCaps::READ
            | FileCaps::POLL_READWRITE;

        self.0.insert_file(fd, file, caps);
        Ok(self)
    }
    pub fn build(self) -> WasiEnviron {
        self.0
    }
}
