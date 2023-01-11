use crate::file::File;
use cap_fs_ext::{DirEntryExt, DirExt, MetadataExt};
use std::any::Any;
use std::path::{Path, PathBuf};
use system_interface::fs::GetSetFdFlags;
use wasmedge_wasi_common::{
    clocks,
    dir::{ReaddirCursor, ReaddirEntity, WasiDir},
    error::{Error, ErrorExt},
    file::{FdFlags, FileType, Filestat, OFlags, WasiFile},
};

pub struct Dir(cap_std::fs::Dir);
impl Dir {
    pub fn from_cap_std(dir: cap_std::fs::Dir) -> Self {
        Dir(dir)
    }

    pub fn open_file_(
        &self,
        symlink_follow: bool,
        path: &str,
        oflags: OFlags,
        read: bool,
        write: bool,
        fdflags: FdFlags,
    ) -> Result<File, Error> {
        use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};

        let mut opts = cap_std::fs::OpenOptions::new();

        if oflags.contains(OFlags::CREATE | OFlags::EXCLUSIVE) {
            opts.create_new(true);
            opts.write(true);
        } else if oflags.contains(OFlags::CREATE) {
            opts.create(true);
            opts.write(true);
        }
        if oflags.contains(OFlags::TRUNCATE) {
            opts.truncate(true);
        }
        if read {
            opts.read(true);
        }
        if write {
            opts.write(true);
        } else {
            // If not opened write, open read. This way the OS lets us open the file.
            // If FileCaps::READ is not set, read calls will be rejected at the
            // get_cap check.
            opts.read(true);
        }
        if fdflags.contains(FdFlags::APPEND) {
            opts.append(true);
        }

        if symlink_follow {
            opts.follow(FollowSymlinks::Yes);
        } else {
            opts.follow(FollowSymlinks::No);
        }
        // the DSYNC, SYNC, and RSYNC flags are ignored! We do not
        // have support for them in cap-std yet.
        // ideally OpenOptions would just support this though:
        // https://github.com/bytecodealliance/cap-std/issues/146
        if fdflags.intersects(
            wasmedge_wasi_common::file::FdFlags::DSYNC
                | wasmedge_wasi_common::file::FdFlags::SYNC
                | wasmedge_wasi_common::file::FdFlags::RSYNC,
        ) {
            return Err(Error::not_supported().context("SYNC family of FdFlags"));
        }

        let mut f = self.0.open_with(Path::new(path), &opts)?;
        // NONBLOCK does not have an OpenOption either, but we can patch that on with set_fd_flags:
        if fdflags.contains(wasmedge_wasi_common::file::FdFlags::NONBLOCK) {
            let set_fd_flags = f.new_set_fd_flags(system_interface::fs::FdFlags::NONBLOCK)?;
            f.set_fd_flags(set_fd_flags)?;
        }
        Ok(File::from_cap_std(f))
    }

    pub fn open_dir_(&self, symlink_follow: bool, path: &str) -> Result<Self, Error> {
        let d = if symlink_follow {
            self.0.open_dir(Path::new(path))?
        } else {
            self.0.open_dir_nofollow(Path::new(path))?
        };
        Ok(Dir::from_cap_std(d))
    }

    pub fn rename_(&self, src_path: &str, dest_dir: &Self, dest_path: &str) -> Result<(), Error> {
        self.0
            .rename(Path::new(src_path), &dest_dir.0, Path::new(dest_path))?;
        Ok(())
    }

    pub fn hard_link_(
        &self,
        src_path: &str,
        target_dir: &Self,
        target_path: &str,
    ) -> Result<(), Error> {
        let src_path = Path::new(src_path);
        let target_path = Path::new(target_path);
        self.0.hard_link(src_path, &target_dir.0, target_path)?;
        Ok(())
    }
}
impl WasiDir for Dir {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn open_file(
        &self,
        symlink_follow: bool,
        path: &str,
        oflags: OFlags,
        read: bool,
        write: bool,
        fdflags: FdFlags,
    ) -> Result<Box<dyn WasiFile>, Error> {
        let f = self.open_file_(symlink_follow, path, oflags, read, write, fdflags)?;
        Ok(Box::new(f))
    }

    fn open_dir(&self, symlink_follow: bool, path: &str) -> Result<Box<dyn WasiDir>, Error> {
        let d = self.open_dir_(symlink_follow, path)?;
        Ok(Box::new(d))
    }

    fn create_dir(&self, path: &str) -> Result<(), Error> {
        self.0.create_dir(Path::new(path))?;
        Ok(())
    }
    fn readdir(
        &self,
        cursor: ReaddirCursor,
    ) -> Result<Box<dyn Iterator<Item = Result<ReaddirEntity, Error>> + Send>, Error> {
        // cap_std's read_dir does not include . and .., we should prepend these.
        // Why does the Ok contain a tuple? We can't construct a cap_std::fs::DirEntry, and we don't
        // have enough info to make a ReaddirEntity yet.
        let dir_meta = self.0.dir_metadata()?;
        let rd = vec![
            {
                let name = ".".to_owned();
                Ok((FileType::Directory, dir_meta.ino(), name))
            },
            {
                let name = "..".to_owned();
                Ok((FileType::Directory, dir_meta.ino(), name))
            },
        ]
        .into_iter()
        .chain({
            // Now process the `DirEntry`s:
            let entries = self.0.entries()?.map(|entry| {
                let entry = entry?;
                let meta = entry.full_metadata()?;
                let inode = meta.ino();
                let filetype = crate::file::filetype_from(&meta.file_type());
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| Error::illegal_byte_sequence().context("filename"))?;
                Ok((filetype, inode, name))
            });

            entries
        })
        // Enumeration of the iterator makes it possible to define the ReaddirCursor
        .enumerate()
        .map(|(ix, r)| match r {
            Ok((filetype, inode, name)) => Ok(ReaddirEntity {
                next: ReaddirCursor::from(ix as u64 + 1),
                filetype,
                inode,
                name,
            }),
            Err(e) => Err(e),
        })
        .skip(u64::from(cursor) as usize);

        Ok(Box::new(rd))
    }

    fn symlink(&self, src_path: &str, dest_path: &str) -> Result<(), Error> {
        self.0.symlink(src_path, dest_path)?;
        Ok(())
    }
    fn remove_dir(&self, path: &str) -> Result<(), Error> {
        self.0.remove_dir(Path::new(path))?;
        Ok(())
    }

    fn unlink_file(&self, path: &str) -> Result<(), Error> {
        self.0.remove_file_or_symlink(Path::new(path))?;
        Ok(())
    }
    fn read_link(&self, path: &str) -> Result<PathBuf, Error> {
        let link = self.0.read_link(Path::new(path))?;
        Ok(link)
    }
    fn get_filestat(&self) -> Result<Filestat, Error> {
        let meta = self.0.dir_metadata()?;
        Ok(Filestat {
            device_id: meta.dev(),
            inode: meta.ino(),
            filetype: crate::file::filetype_from(&meta.file_type()),
            nlink: meta.nlink(),
            size: meta.len(),
            atim: meta.accessed().map(|t| Some(t.into_std())).unwrap_or(None),
            mtim: meta.modified().map(|t| Some(t.into_std())).unwrap_or(None),
            ctim: meta.created().map(|t| Some(t.into_std())).unwrap_or(None),
        })
    }
    fn get_path_filestat(&self, path: &str, follow_symlinks: bool) -> Result<Filestat, Error> {
        let meta = if follow_symlinks {
            self.0.metadata(Path::new(path))?
        } else {
            self.0.symlink_metadata(Path::new(path))?
        };
        Ok(Filestat {
            device_id: meta.dev(),
            inode: meta.ino(),
            filetype: crate::file::filetype_from(&meta.file_type()),
            nlink: meta.nlink(),
            size: meta.len(),
            atim: meta.accessed().map(|t| Some(t.into_std())).unwrap_or(None),
            mtim: meta.modified().map(|t| Some(t.into_std())).unwrap_or(None),
            ctim: meta.created().map(|t| Some(t.into_std())).unwrap_or(None),
        })
    }
    fn rename(&self, src_path: &str, dest_dir: &dyn WasiDir, dest_path: &str) -> Result<(), Error> {
        let dest_dir = dest_dir
            .as_any()
            .downcast_ref::<Self>()
            .ok_or(Error::badf().context("failed downcast to cap-std Dir"))?;
        self.rename_(src_path, dest_dir, dest_path)
    }
    fn hard_link(
        &self,
        src_path: &str,
        target_dir: &dyn WasiDir,
        target_path: &str,
    ) -> Result<(), Error> {
        let target_dir = target_dir
            .as_any()
            .downcast_ref::<Self>()
            .ok_or(Error::badf().context("failed downcast to cap-std Dir"))?;
        self.hard_link_(src_path, target_dir, target_path)
    }
    fn set_times(
        &self,
        path: &str,
        atime: Option<clocks::SystemTimeSpec>,
        mtime: Option<clocks::SystemTimeSpec>,
        follow_symlinks: bool,
    ) -> Result<(), Error> {
        if follow_symlinks {
            self.0.set_times(
                Path::new(path),
                convert_systimespec(atime),
                convert_systimespec(mtime),
            )?;
        } else {
            self.0.set_symlink_times(
                Path::new(path),
                convert_systimespec(atime),
                convert_systimespec(mtime),
            )?;
        }
        Ok(())
    }
}

fn convert_systimespec(t: Option<clocks::SystemTimeSpec>) -> Option<cap_fs_ext::SystemTimeSpec> {
    match t {
        Some(clocks::SystemTimeSpec::Absolute(t)) => Some(cap_fs_ext::SystemTimeSpec::Absolute(t)),
        Some(clocks::SystemTimeSpec::SymbolicNow) => Some(cap_fs_ext::SystemTimeSpec::SymbolicNow),
        None => None,
    }
}
