use std::path::{Path, PathBuf};

pub(crate) fn normalize_windows_path_prefix(path: PathBuf) -> PathBuf {
    normalize_windows_path_prefix_ref(path.as_path())
}

pub(crate) fn normalize_windows_path_prefix_ref(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if let Some(stripped) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{stripped}"));
        }
        if let Some(stripped) = text.strip_prefix(r"\\?\") {
            return PathBuf::from(stripped);
        }
    }

    path.to_path_buf()
}
