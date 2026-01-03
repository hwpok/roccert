use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

/// 函数实现两个功能
/// 1. suffix_cond为值时, 加后缀
/// 2. file_path为None时, 取当前目录
pub fn get_fix_path_buf(
    file_path: Option<String>,
    suffix: &str,
    suffix_cond: bool,
    default_filename: &str,
) -> Result<PathBuf> {
    // 确定基础路径
    let base_path = match file_path {
        Some(p) => PathBuf::from(p),
        None => env::current_dir()
            .context("Failed to get current working directory")?
            .join(default_filename),
    };

    // 按需添加后缀
    if suffix_cond {
        let mut os_string = base_path.as_os_str().to_owned();
        os_string.push(".");
        os_string.push(suffix);
        Ok(PathBuf::from(os_string))
    } else {
        Ok(base_path)
    }
}

// 给文件加后缀
pub fn append_suffix_to_filename(path: &Path, suffix: &str) -> Result<PathBuf> {
    let filename = path.file_name().context("Path must have a filename")?;
    let mut new_filename = filename.to_os_string();
    new_filename.push(suffix);
    Ok(path.with_file_name(new_filename))
}
