use std::fs;
use tempfile::TempDir;

pub fn create_test_file(temp_dir: &TempDir, path: &str, contents: &str) -> anyhow::Result<()> {
    let full_path = temp_dir.path().join(path);
    fs::create_dir_all(full_path.parent().unwrap())?;
    fs::write(full_path, contents)?;
    Ok(())
}
