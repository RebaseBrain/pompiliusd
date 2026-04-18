use std::fs;
use std::path::{Path, PathBuf};

pub fn get_all_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Рекурсивно заходим в поддиректории
                get_all_files(&path, files);
            } else {
                // Добавляем файл в список
                files.push(path);
            }
        }
    }
}
