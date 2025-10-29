use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use serde::Deserialize;
use std::env;
use regex::Regex;
use std::collections::HashSet;

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CompilerType {
    CC65,
}

/// Simple configuration structure that contains all needed settings
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub name: Option<String>,
    pub target: Option<String>,
    pub compiler: Option<CompilerType>,
    pub include_paths: Option<Vec<PathBuf>>,
    pub src_files: Option<Vec<PathBuf>>,
    pub test_script: Option<PathBuf>,
    pub configs: Option<Vec<PathBuf>>,  // References to other config files
    
    // CC65-specific settings
    pub config_file: Option<PathBuf>,
    pub asm_include_paths: Option<Vec<PathBuf>>,
    
    // CC65 compiler flags
    pub c_flags: Option<Vec<String>>,
    pub asm_flags: Option<Vec<String>>,
    pub ld_flags: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: None,
            target: None,
            compiler: None,
            include_paths: None,
            src_files: None,
            test_script: None,
            configs: None,
            config_file: None,
            asm_include_paths: None,
            c_flags: None,
            asm_flags: None,
            ld_flags: None,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        // Get the load order first
        let mut seen = HashSet::new();
        let load_order = Self::get_config_load_order(path, &mut seen)?;
        
        // Now load configs in the right order (base configs first)
        let mut final_config = Config::default();
        for config_path in load_order {
            let mut config = Self::load_single(&config_path)?;
            
            // Resolve paths relative to this config's directory
            let config_dir = config_path.parent().unwrap_or_else(|| Path::new(""));
            config.resolve_paths(config_dir);
            
            // Merge into our final config (this config's values take precedence)
            final_config = final_config.merge(config);
        }
        
        Ok(final_config)
    }

    /// Get the order in which configs should be loaded (depth-first)
    fn get_config_load_order(path: &Path, seen: &mut HashSet<PathBuf>) -> Result<Vec<PathBuf>> {
        // Check for circular dependencies using the path as is
        if !seen.insert(path.to_path_buf()) {
            return Err(anyhow::anyhow!("Circular dependency detected while loading config: {}", path.display()));
        }
        
        let mut load_order = Vec::new();
        
        // Load this config file just to check its 'configs' field
        let config: Config = Self::load_single(path)?;
        let config_dir = path.parent().unwrap_or_else(|| Path::new(""));
        
        // First add all dependencies
        if let Some(ref configs) = config.configs {
            for config_path in configs {
                let full_path = config_dir.join(config_path);
                if !full_path.exists() {
                    return Err(anyhow::anyhow!(
                        "Config file not found: {}. Paths are resolved relative to the config file directory: {}",
                        config_path.display(),
                        config_dir.display()
                    ));
                }
                
                // Recursively get the load order for this dependency
                let mut dep_order = Self::get_config_load_order(&full_path, seen)?;
                load_order.append(&mut dep_order);
            }
        }
        
        // Then add this config
        load_order.push(path.to_path_buf());
        
        Ok(load_order)
    }

    /// Load a single config file without processing its dependencies
    fn load_single(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let contents_with_env = replace_env_vars(&contents)?;
        
        serde_yaml::from_str(&contents_with_env)
            .with_context(|| format!("Failed to parse YAML from {}", path.display()))
    }

    fn resolve_paths(&mut self, base_dir: &Path) {
        // Helper function to canonicalize a path relative to base_dir
        fn canonicalize_path(base_dir: &Path, path: &Path) -> PathBuf {
            let full_path = base_dir.join(path);
            match full_path.canonicalize() {
                Ok(p) => p,
                Err(_) => full_path,  // If canonicalization fails, use the joined path
            }
        }

        if let Some(paths) = &mut self.include_paths {
            *paths = paths.iter()
                .map(|p| canonicalize_path(base_dir, p))
                .collect();
        }
        if let Some(paths) = &mut self.src_files {
            *paths = paths.iter()
                .map(|p| canonicalize_path(base_dir, p))
                .collect();
        }
        if let Some(script) = &mut self.test_script {
            *script = canonicalize_path(base_dir, script);
        }
        if let Some(cf) = &mut self.config_file {
            *cf = canonicalize_path(base_dir, cf);
        }
        if let Some(paths) = &mut self.asm_include_paths {
            *paths = paths.iter()
                .map(|p| canonicalize_path(base_dir, p))
                .collect();
        }
    }

    /// Merge another config into this one (other config takes precedence)
    fn merge(self, other: Config) -> Config {
        Config {
            // Simple values - take other's value if it exists
            name: other.name.or(self.name),
            target: other.target.or(self.target),
            compiler: other.compiler.or(self.compiler),
            test_script: other.test_script.or(self.test_script),
            config_file: other.config_file.or(self.config_file),
            configs: None,  // Don't carry forward config references
            
            // For paths, combine both sets if both exist
            include_paths: match (self.include_paths, other.include_paths) {
                (Some(mut ours), Some(theirs)) => {
                    ours.extend(theirs);
                    Some(ours)
                }
                (Some(paths), None) | (None, Some(paths)) => Some(paths),
                (None, None) => None,
            },
            src_files: match (self.src_files, other.src_files) {
                (Some(mut ours), Some(theirs)) => {
                    ours.extend(theirs);
                    Some(ours)
                }
                (Some(paths), None) | (None, Some(paths)) => Some(paths),
                (None, None) => None,
            },
            asm_include_paths: match (self.asm_include_paths, other.asm_include_paths) {
                (Some(mut ours), Some(theirs)) => {
                    ours.extend(theirs);
                    Some(ours)
                }
                (Some(paths), None) | (None, Some(paths)) => Some(paths),
                (None, None) => None,
            },
            
            // For flags, combine both sets if both exist
            c_flags: match (self.c_flags, other.c_flags) {
                (Some(mut ours), Some(theirs)) => {
                    ours.extend(theirs);
                    Some(ours)
                }
                (Some(flags), None) | (None, Some(flags)) => Some(flags),
                (None, None) => None,
            },
            asm_flags: match (self.asm_flags, other.asm_flags) {
                (Some(mut ours), Some(theirs)) => {
                    ours.extend(theirs);
                    Some(ours)
                }
                (Some(flags), None) | (None, Some(flags)) => Some(flags),
                (None, None) => None,
            },
            ld_flags: match (self.ld_flags, other.ld_flags) {
                (Some(mut ours), Some(theirs)) => {
                    ours.extend(theirs);
                    Some(ours)
                }
                (Some(flags), None) | (None, Some(flags)) => Some(flags),
                (None, None) => None,
            },
        }
    }
}

fn replace_env_vars(content: &str) -> Result<String> {
    let re = Regex::new(r"\$\{([^}]+)\}").context("Failed to compile regex pattern")?;
    let mut modified_content = content.to_string();
    
    for capture in re.captures_iter(content) {
        let full_match = capture.get(0).unwrap().as_str();
        let var_name = capture.get(1).unwrap().as_str();
        let value = env::var(var_name)
            .with_context(|| format!("Environment variable '{}' not found", var_name))?;
        modified_content = modified_content.replace(full_match, &value);
    }
    
    Ok(modified_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_realistic_config_usage() -> Result<()> {
        let temp_dir = TempDir::new()?;
        
        // Create platform config (defines core settings)
        let platform_config_path = temp_dir.path().join("atari.yaml");
        let platform_content = r#"
compiler: cc65
target: atari
config_file: "platform/atari.cfg"
include_paths:
  - "platform/include"  # Platform-specific includes
"#;
        fs::write(&platform_config_path, platform_content)?;

        // Create common library config
        let common_lib_path = temp_dir.path().join("common_lib.yaml");
        let common_lib_content = r#"
src_files:
  - "lib/common/io.s"
  - "lib/common/math.s"
include_paths:
  - "lib/common/include"
"#;
        fs::write(&common_lib_path, common_lib_content)?;

        // Create game-specific library config
        let game_lib_path = temp_dir.path().join("game_lib.yaml");
        let game_lib_content = r#"
src_files:
  - "lib/game/sprites.s"
  - "lib/game/sound.s"
include_paths:
  - "lib/game/include"
"#;
        fs::write(&game_lib_path, game_lib_content)?;

        // Create main project config that brings everything together
        let project_config_path = temp_dir.path().join("game.yaml");
        let project_content = r#"
configs:
  - "atari.yaml"      # Platform config first
  - "common_lib.yaml" # Then common libraries
  - "game_lib.yaml"   # Then game-specific libraries
name: "awesome_game"  # Project-specific settings
src_files:
  - "src/main.s"
  - "src/levels.s"
include_paths:
  - "src/include"
"#;
        fs::write(&project_config_path, project_content)?;

        // Load and verify
        let config = Config::load(&project_config_path)?;
        
        // Core settings should come from platform config
        assert_eq!(config.compiler, Some(CompilerType::CC65));
        assert_eq!(config.target, Some("atari".to_string()));
        assert_eq!(config.config_file, Some(temp_dir.path().join("platform/atari.cfg")));
        assert_eq!(config.name, Some("awesome_game".to_string()));
        
        // Include paths should be combined in order
        let include_paths = config.include_paths.unwrap();
        assert_eq!(include_paths, vec![
            temp_dir.path().join("platform/include"),     // From platform
            temp_dir.path().join("lib/common/include"),   // From common lib
            temp_dir.path().join("lib/game/include"),     // From game lib
            temp_dir.path().join("src/include"),          // From project
        ]);
        
        // Source files should be combined in order
        let src_files = config.src_files.unwrap();
        assert_eq!(src_files, vec![
            temp_dir.path().join("lib/common/io.s"),    // From common lib
            temp_dir.path().join("lib/common/math.s"),
            temp_dir.path().join("lib/game/sprites.s"),  // From game lib
            temp_dir.path().join("lib/game/sound.s"),
            temp_dir.path().join("src/main.s"),         // From project
            temp_dir.path().join("src/levels.s"),
        ]);
        
        Ok(())
    }

    #[test]
    fn test_env_vars() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("test.yaml");

        env::set_var("PROJECT_NAME", "test_game");
        env::set_var("SDK_PATH", "sdk");  // Using relative path

        let content = r#"
name: ${PROJECT_NAME}
include_paths:
  - "${SDK_PATH}/include"
compiler: cc65
"#;
        fs::write(&config_path, content)?;

        let config = Config::load(&config_path)?;
        
        assert_eq!(config.name, Some("test_game".to_string()));
        assert_eq!(config.include_paths, Some(vec![temp_dir.path().join("sdk/include")]));
        assert_eq!(config.compiler, Some(CompilerType::CC65));

        env::remove_var("PROJECT_NAME");
        env::remove_var("SDK_PATH");
        
        Ok(())
    }

    #[test]
    fn test_circular_dependency() -> Result<()> {
        let temp_dir = TempDir::new()?;
        
        // Create two configs that reference each other
        let config1_path = temp_dir.path().join("config1.yaml");
        let config1_content = r#"
configs:
  - "config2.yaml"
"#;
        fs::write(&config1_path, config1_content)?;

        let config2_path = temp_dir.path().join("config2.yaml");
        let config2_content = r#"
configs:
  - "config1.yaml"
"#;
        fs::write(&config2_path, config2_content)?;

        // Loading should fail with circular dependency error
        let result = Config::load(&config1_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular dependency"));
        
        Ok(())
    }
} 