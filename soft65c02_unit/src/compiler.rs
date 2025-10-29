use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::Result;
use std::fmt::Debug;

use crate::config::{Config, CompilerType};

pub mod cc65;

pub trait Compiler: Debug {
    fn create_output_path_mapping(&self, sources: &[PathBuf]) -> HashMap<PathBuf, PathBuf>;
    fn compile_source(&self, source: &Path, work_dir: &Path, path_mapping: &HashMap<PathBuf, PathBuf>) -> Result<PathBuf>;
    fn link_objects(&self, objects: &[PathBuf], output: &Path, work_dir: &Path) -> Result<()>;
    fn get_symbols_path(&self, work_dir: &Path) -> PathBuf;
}

pub fn create_compiler(compiler_type: &CompilerType, config: &Config, verbose: bool, dry_run: bool) -> Result<Box<dyn Compiler>> {
    match compiler_type {
        CompilerType::CC65 => Ok(Box::new(cc65::CC65Compiler::new(config, verbose, dry_run)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Helper function to create a basic test config
    fn create_test_config() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("config.cfg");
        fs::write(&config_file, "mock config").unwrap();
        (temp_dir, config_file)
    }

    #[test]
    fn test_create_compiler() {
        let (_temp_dir, config_file) = create_test_config();
        let config = Config {
            target: Some("mock".to_string()),
            compiler: Some(CompilerType::CC65),
            config_file: Some(config_file),
            ..Config::default()
        };
        
        let result = create_compiler(&CompilerType::CC65, &config, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_compiler_missing_target() {
        let (_temp_dir, config_file) = create_test_config();
        let config = Config {
            target: None,
            compiler: Some(CompilerType::CC65),
            config_file: Some(config_file),
            ..Config::default()
        };
        
        let result = create_compiler(&CompilerType::CC65, &config, false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No target specified"));
    }

} 