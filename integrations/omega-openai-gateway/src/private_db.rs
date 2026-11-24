//! Private database paths for single-operator Gateway state.
use std::{io,path::{Path,PathBuf}};

#[cfg(unix)]
pub fn prepare_private_database(path:&Path)->io::Result<PathBuf>{
    use std::fs::OpenOptions;
    use std::os::unix::fs::{MetadataExt,OpenOptionsExt};
    use std::path::Component;
    let denied=||io::Error::new(io::ErrorKind::PermissionDenied,"database path must be owned and private");
    if !path.is_absolute() || path.components().any(|c|matches!(c,Component::ParentDir)) {return Err(denied());}
    let parent=path.parent().ok_or_else(denied)?;
    if !parent.symlink_metadata()?.is_dir(){return Err(denied());}
    let parent=parent.canonicalize()?;
    let uid=unsafe{libc::geteuid()};
    let meta=parent.symlink_metadata()?;
    if !meta.is_dir() || meta.uid()!=uid || meta.mode()&0o077!=0{return Err(denied());}
    let file=parent.join(path.file_name().ok_or_else(denied)?);
    match OpenOptions::new().write(true).create_new(true).mode(0o600)
        .custom_flags(libc::O_NOFOLLOW).open(&file){
        Ok(f)=>drop(f),
        Err(e) if e.kind()==io::ErrorKind::AlreadyExists=>(),
        Err(e)=>return Err(e),
    }
    let meta=file.symlink_metadata()?;
    if !meta.is_file() || meta.uid()!=uid || meta.mode()&0o077!=0 || meta.nlink()!=1{return Err(denied());}
    Ok(file)
}
#[cfg(not(unix))]
pub fn prepare_private_database(_path:&Path)->io::Result<PathBuf>{
    Err(io::Error::new(io::ErrorKind::Unsupported,"private state requires a verified Unix host"))
}
