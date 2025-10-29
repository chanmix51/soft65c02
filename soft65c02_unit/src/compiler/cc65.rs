use std::path::{Path, PathBuf};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::fmt::{self, Debug};

use super::Compiler;
use crate::config::Config;
use crate::executor::{Executor, CommandExecutor};

pub struct CC65Compiler {
    target: String,
    include_paths: Vec<PathBuf>,
    asm_include_paths: Vec<PathBuf>,
    config_file: PathBuf,
    c_flags: Vec<String>,
    asm_flags: Vec<String>,
    ld_flags: Vec<String>,
    verbose: bool,
    dry_run: bool,
    executor: Box<dyn Executor>,
}

impl Debug for CC65Compiler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CC65Compiler")
            .field("target", &self.target)
            .field("include_paths", &self.include_paths)
            .field("asm_include_paths", &self.asm_include_paths)
            .field("config_file", &self.config_file)
            .field("c_flags", &self.c_flags)
            .field("asm_flags", &self.asm_flags)
            .field("ld_flags", &self.ld_flags)
            .field("verbose", &self.verbose)
            .field("dry_run", &self.dry_run)
            .field("executor", &"<dyn Executor>")
            .finish()
    }
}

impl CC65Compiler {
    pub fn new(config: &Config, verbose: bool, dry_run: bool) -> Result<Self> {
        // Verify we have the required configuration
        let target = config.target
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No target specified in config"))?
            .clone();
            
        let config_file = config.config_file
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No config file specified in config"))?
            .clone();

        Ok(Self {
            target,
            include_paths: config.include_paths.clone().unwrap_or_default(),
            asm_include_paths: config.asm_include_paths.clone().unwrap_or_default(),
            config_file,
            c_flags: config.c_flags.clone().unwrap_or_default(),
            asm_flags: config.asm_flags.clone().unwrap_or_default(),
            ld_flags: config.ld_flags.clone().unwrap_or_default(),
            verbose,
            dry_run,
            executor: Box::new(CommandExecutor::new("cl65")),
        })
    }

    /// Create a new compiler with custom executor (mainly for testing)
    #[cfg(test)]
    pub fn with_mock_executor(
        config: &Config,
        verbose: bool,
        dry_run: bool,
        executor: Box<dyn Executor>,
    ) -> Result<Self> {
        let target = config.target
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No target specified in config"))?
            .clone();
            
        let config_file = config.config_file
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No config file specified in config"))?
            .clone();

        Ok(Self {
            target,
            include_paths: config.include_paths.clone().unwrap_or_default(),
            asm_include_paths: config.asm_include_paths.clone().unwrap_or_default(),
            config_file,
            c_flags: config.c_flags.clone().unwrap_or_default(),
            asm_flags: config.asm_flags.clone().unwrap_or_default(),
            ld_flags: config.ld_flags.clone().unwrap_or_default(),
            verbose,
            dry_run,
            executor,
        })
    }

    fn debug_command(&self, args: &[String]) {
        if self.verbose || self.dry_run {
            println!("Command: cl65 {}", args.join(" "));
        }
    }

    /// Creates a mapping of absolute source paths to their unique output paths
    pub fn create_output_path_mapping(&self, sources: &[PathBuf]) -> HashMap<PathBuf, PathBuf> {
        if self.dry_run {
            println!("[MAPPING] Creating output path mapping for {} sources", sources.len());
            for source in sources {
                println!("[MAPPING] Input source: {}", source.display());
            }
        }
        
        let mut mapping = HashMap::new();
        let mut seen_names = HashMap::new();
        
        // First pass: try just filenames and identify conflicts
        for path in sources {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            seen_names.entry(name).or_insert_with(Vec::new).push(path.clone());
        }

        // Second pass: create unique paths
        for (name, paths) in seen_names {
            if paths.len() == 1 {
                // No conflict - use just the filename
                mapping.insert(paths[0].clone(), PathBuf::from(&name));
                if self.dry_run {
                    println!("[PATH] Using filename for {}", paths[0].display());
                }
            } else {
                // Conflict - use parent directory names until unique
                for path in paths {
                    let mut n = 2;
                    loop {
                        let components: Vec<_> = path.components().collect();
                        let path_str = if components.len() < n {
                            components.iter()
                                .map(|c| c.as_os_str().to_string_lossy())
                                .collect::<Vec<_>>()
                                .join("/")
                        } else {
                            components[components.len().saturating_sub(n)..]
                                .iter()
                                .map(|c| c.as_os_str().to_string_lossy())
                                .collect::<Vec<_>>()
                                .join("/")
                        };

                        // Check if this path is unique among all values in the mapping
                        if !mapping.values().any(|p| p.to_string_lossy() == path_str) {
                            if self.dry_run {
                                println!("[PATH] Using {}-component path for {}", n, path.display());
                            }
                            mapping.insert(path.clone(), PathBuf::from(path_str));
                            break;
                        }
                        n += 1;
                    }
                }
            }
        }

        mapping
    }

    /// Creates a directory structure in the work_dir that matches the source file's path
    fn create_output_dirs(&self, source: &Path, work_dir: &Path, path_mapping: &HashMap<PathBuf, PathBuf>) -> Result<PathBuf> {
        // Get the unique path for this source
        let relative_path = path_mapping.get(source)
            .map(|p| p.clone())
            .unwrap_or_else(|| {
                // Fallback to just filename if something went wrong
                let name = source.file_name().unwrap().to_string_lossy().to_string();
                if self.dry_run {
                    println!("[PATH] Falling back to filename for {}", source.display());
                }
                PathBuf::from(name)
            });

        // Create output directory under work_dir/build
        let output_dir = work_dir.join("build").join(relative_path.parent().unwrap_or_else(|| Path::new("")));

        if !self.dry_run {
            // Create the directory structure
            fs::create_dir_all(&output_dir)
                .map_err(|e| anyhow::anyhow!("Failed to create output directory {}: {}", output_dir.display(), e))?;
        } else {
            println!("[DRY RUN] Would create directory: {:?}", output_dir);
        }

        Ok(output_dir)
    }

    /// Compiles a single source file
    /// 
    /// # Arguments
    /// * `abs_source` - Canonicalized path to the source file (must match a key in path_mapping)
    /// * `work_dir` - Path to the work directory (will be canonicalized)
    /// * `path_mapping` - Mapping from canonicalized source paths to their unique output paths
    fn compile_source(&self, abs_source: &Path, work_dir: &Path, path_mapping: &HashMap<PathBuf, PathBuf>) -> Result<PathBuf> {
        if self.dry_run {
            println!("[COMPILE] Source: {}", abs_source.display());
            println!("[COMPILE] Work dir: {}", work_dir.display());
            println!("[COMPILE] Path mapping contains {} entries", path_mapping.len());
            for (k, v) in path_mapping {
                println!("[COMPILE]   {} -> {}", k.display(), v.display());
            }
        }

        // Create output directory structure matching source path
        let output_dir = self.create_output_dirs(abs_source, work_dir, path_mapping)?;
        
        let obj_name = abs_source.file_stem().unwrap();
        let obj_file = output_dir.join(obj_name).with_extension("o");
        let dep_file = output_dir.join(obj_name).with_extension("d");
        let lst_file = output_dir.join(obj_name).with_extension("lst");

        if self.dry_run {
            println!("[COMPILE] Output paths:");
            println!("[COMPILE]   obj: {}", obj_file.display());
            println!("[COMPILE]   dep: {}", dep_file.display());
            println!("[COMPILE]   lst: {}", lst_file.display());
        }
        
        let args = self.generate_compile_args(abs_source, &obj_file, &dep_file, &lst_file);
        self.execute_cl65(&args)
            .map_err(|e| anyhow::anyhow!("Failed to compile {}: {}", abs_source.display(), e))?;

        Ok(obj_file)
    }

    fn generate_compile_args(&self, source: &Path, obj_file: &Path, dep_file: &Path, lst_file: &Path) -> Vec<String> {
        if self.dry_run {
            println!("[ARGS] Generating compile args for:");
            println!("[ARGS]   source: {}", source.display());
            println!("[ARGS]   obj: {}", obj_file.display());
            println!("[ARGS]   dep: {}", dep_file.display());
            println!("[ARGS]   lst: {}", lst_file.display());
        }

        let mut args = Vec::new();
        
        // Target platform must come first
        args.extend(["-t".to_string(), self.target.clone()]);
        
        // Compile only
        args.push("-c".to_string());
        
        // Dependency file - output files don't need canonicalization as they're constructed from canonicalized paths
        args.extend([
            "--create-dep".to_string(),
            dep_file.to_string_lossy().to_string()
        ]);
        
        // Add appropriate flags based on file extension
        if let Some(extension) = source.extension() {
            match extension.to_string_lossy().to_lowercase().as_str() {
                "c" => {
                    // Add C flags
                    for flag in &self.c_flags {
                        args.push(flag.clone());
                    }
                    if self.dry_run {
                        println!("[ARGS] Added C flags: {:?}", self.c_flags);
                    }
                }
                "s" => {
                    // Add assembly flags
                    for flag in &self.asm_flags {
                        args.push(flag.clone());
                    }
                    if self.dry_run {
                        println!("[ARGS] Added ASM flags: {:?}", self.asm_flags);
                    }
                }
                _ => {
                    if self.dry_run {
                        println!("[ARGS] Unknown file extension: {:?}, no specific flags added", extension);
                    }
                }
            }
        }
        
        // C include paths in order
        for path in &self.include_paths {
            args.extend([
                "--include-dir".to_string(),
                path.to_string_lossy().to_string()
            ]);
        }

        // Assembly include paths in order
        for path in &self.asm_include_paths {
            args.extend([
                "--asm-include-dir".to_string(),
                path.to_string_lossy().to_string()
            ]);
        }
        
        // Listing file
        args.extend([
            "--listing".to_string(),
            lst_file.to_string_lossy().to_string()
        ]);
        
        // Output file must come before input
        args.extend([
            "-o".to_string(),
            obj_file.to_string_lossy().to_string()
        ]);
        
        // Input source must be last - source is already canonicalized
        args.push(source.to_string_lossy().to_string());

        if self.dry_run {
            println!("[ARGS] Generated args: {}", args.join(" "));
        }

        args
    }

    fn generate_link_args(&self, objects: &[PathBuf], output: &Path, map_file: &Path, lbl_file: &Path) -> Vec<String> {
        let mut args = Vec::new();
        
        // Target platform must come first
        args.extend(["-t".to_string(), self.target.clone()]);
        
        // Linker config
        args.extend([
            "-C".to_string(),
            self.config_file.to_string_lossy().to_string()
        ]);
        
        // Add linker flags
        for flag in &self.ld_flags {
            args.push(flag.clone());
        }
        if self.dry_run {
            println!("[ARGS] Added LD flags: {:?}", self.ld_flags);
        }
        
        // Map and label files
        args.extend([
            "--mapfile".to_string(),
            map_file.to_string_lossy().to_string(),
            "-Ln".to_string(),
            lbl_file.to_string_lossy().to_string()
        ]);
        
        // Output binary must come before inputs
        args.extend([
            "-o".to_string(),
            output.to_string_lossy().to_string()
        ]);
        
        // Input objects must be last, in order
        for obj in objects {
            args.push(obj.to_string_lossy().to_string());
        }

        args
    }

    /// Execute cl65 with the given arguments
    fn execute_cl65(&self, args: &[String]) -> Result<(), String> {
        self.debug_command(args);
        if !self.dry_run {
            self.executor.execute(args)
        } else {
            Ok(())
        }
    }
}

impl Compiler for CC65Compiler {
    fn create_output_path_mapping(&self, sources: &[PathBuf]) -> HashMap<PathBuf, PathBuf> {
        self.create_output_path_mapping(sources)
    }

    fn compile_source(&self, source: &Path, work_dir: &Path, path_mapping: &HashMap<PathBuf, PathBuf>) -> Result<PathBuf> {
        self.compile_source(source, work_dir, path_mapping)
    }

    fn link_objects(&self, objects: &[PathBuf], output: &Path, work_dir: &Path) -> Result<()> {
        let map_file = work_dir.join("app.map");
        let lbl_file = work_dir.join("app.lbl");
        
        let args = self.generate_link_args(objects, output, &map_file, &lbl_file);
        self.execute_cl65(&args)
            .map_err(|e| anyhow::anyhow!("Failed to link objects: {}", e))
    }

    fn get_symbols_path(&self, work_dir: &Path) -> PathBuf {
        work_dir.join("app.lbl")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::tests::MockExecutor;
    use crate::config::CompilerType;
    use tempfile::TempDir;

    fn create_test_config() -> Config {
        let mut config = Config::default();
        config.target = Some("atari".to_string());
        config.compiler = Some(CompilerType::CC65);
        config.config_file = Some(PathBuf::from("config.cfg"));
        config.asm_include_paths = Some(vec![
            PathBuf::from("asm1"),
            PathBuf::from("asm2"),
        ]);
        config.include_paths = Some(vec![
            PathBuf::from("include1"),
            PathBuf::from("include2"),
        ]);
        config.c_flags = Some(vec![
            "-O".to_string(),
            "-g".to_string(),
        ]);
        config.asm_flags = Some(vec![
            "-g".to_string(),
        ]);
        config.ld_flags = Some(vec![
            "-m".to_string(),
            "atari.map".to_string(),
        ]);
        config
    }

    fn create_test_compiler() -> CC65Compiler {
        CC65Compiler::new(&create_test_config(), true, false).unwrap()
    }

    #[test]
    fn test_compile_args_generation() {
        let compiler = create_test_compiler();
        let source = Path::new("src/test.c");
        let obj_file = Path::new("out/build/src/test.o");
        let dep_file = Path::new("out/build/src/test.d");
        let lst_file = Path::new("out/build/src/test.lst");

        let args = compiler.generate_compile_args(source, obj_file, dep_file, lst_file);

        // Define the expected order of arguments
        let expected_args = vec![
            "-t", "atari",           // Target platform must come first
            "-c",                    // Compile only
            "--create-dep", "out/build/src/test.d",  // Dependency file
            "-O", "-g",              // C flags for .c files
            "--include-dir", "include1",     // C include paths in order
            "--include-dir", "include2",
            "--asm-include-dir", "asm1",     // ASM include paths in order
            "--asm-include-dir", "asm2",
            "--listing", "out/build/src/test.lst",   // Listing file
            "-o", "out/build/src/test.o",           // Output file must come before input
        ];

        // Convert expected args to String for comparison
        let expected: Vec<String> = expected_args.into_iter().map(String::from).collect();
        
        // The source path will be absolute, so we only check that it ends with our relative path
        let source_arg = args.last().unwrap();
        assert!(source_arg.ends_with("src/test.c"), "Source path '{}' does not end with 'src/test.c'", source_arg);
        
        // Remove the last argument (source path) from both vectors before comparing
        let mut args = args;
        args.pop();
        assert_eq!(args, expected, "Arguments are not in the expected order");
    }

    #[test]
    fn test_link_args_generation() {
        let compiler = create_test_compiler();
        let objects = vec![
            PathBuf::from("build/test1.o"),
            PathBuf::from("build/test2.o"),
        ];
        let output = Path::new("output/game.nes");
        let map_file = Path::new("output/app.map");
        let lbl_file = Path::new("output/app.lbl");

        let args = compiler.generate_link_args(&objects, output, map_file, lbl_file);

        // Define the expected order of arguments
        let expected_args = vec![
            "-t", "atari",           // Target platform must come first
            "-C", "config.cfg",      // Linker configuration
            "-m", "atari.map",       // Linker flags
            "--mapfile", "output/app.map",  // Map file
            "-Ln", "output/app.lbl",       // Label file
            "-o", "output/game.nes",       // Output file must come before inputs
            "build/test1.o",              // Object files must be last, in order
            "build/test2.o",
        ];

        // Convert expected args to String for comparison
        let expected: Vec<String> = expected_args.into_iter().map(String::from).collect();
        assert_eq!(args, expected, "Arguments are not in the expected order");
    }

    #[test]
    fn test_asm_compile_args_generation() {
        let compiler = create_test_compiler();
        let source = Path::new("src/test.s");
        let obj_file = Path::new("out/build/src/test.o");
        let dep_file = Path::new("out/build/src/test.d");
        let lst_file = Path::new("out/build/src/test.lst");

        let args = compiler.generate_compile_args(source, obj_file, dep_file, lst_file);

        // Define the expected order of arguments
        let expected_args = vec![
            "-t", "atari",           // Target platform must come first
            "-c",                    // Compile only
            "--create-dep", "out/build/src/test.d",  // Dependency file
            "-g",                    // ASM flags for .s files
            "--include-dir", "include1",     // C include paths in order
            "--include-dir", "include2",
            "--asm-include-dir", "asm1",     // ASM include paths in order
            "--asm-include-dir", "asm2",
            "--listing", "out/build/src/test.lst",   // Listing file
            "-o", "out/build/src/test.o",           // Output file must come before input
        ];

        // Convert expected args to String for comparison
        let expected: Vec<String> = expected_args.into_iter().map(String::from).collect();
        
        // The source path will be absolute, so we only check that it ends with our relative path
        let source_arg = args.last().unwrap();
        assert!(source_arg.ends_with("src/test.s"), "Source path '{}' does not end with 'src/test.s'", source_arg);
        
        // Remove the last argument (source path) from both vectors before comparing
        let mut args = args;
        args.pop();
        assert_eq!(args, expected, "Arguments are not in the expected order for assembly files");
    }

    #[test]
    fn test_compile_source() {
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("src/test.c");
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::write(&source_path, "int main() { return 0; }").unwrap();

        let mock_executor = Box::new(MockExecutor::new(vec![Ok(())]));
        let compiler = CC65Compiler::with_mock_executor(
            &create_test_config(),
            false,
            false,
            mock_executor,
        ).unwrap();

        let work_dir = PathBuf::from("build");
        let path_mapping = compiler.create_output_path_mapping(&[source_path.clone()]);
        let result = compiler.compile_source(&source_path, &work_dir, &path_mapping);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_failure() {
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("src/test.c");
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::write(&source_path, "int main() { return 0; }").unwrap();

        let mock_executor = Box::new(MockExecutor::new(vec![Err("Mock compilation error".to_string())]));
        let compiler = CC65Compiler::with_mock_executor(
            &create_test_config(),
            false,
            false,
            mock_executor,
        ).unwrap();

        let work_dir = PathBuf::from("build");
        let path_mapping = compiler.create_output_path_mapping(&[source_path.clone()]);
        let result = compiler.compile_source(&source_path, &work_dir, &path_mapping);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Mock compilation error"));
    }

    #[test]
    fn test_unique_path_components() {
        let compiler = create_test_compiler();
        
        // Test case 1: No conflicts
        let sources = vec![
            PathBuf::from("src/foo/a.s"),
            PathBuf::from("src/bar/b.s"),
        ];
        let mapping = compiler.create_output_path_mapping(&sources);
        assert_eq!(mapping.len(), 2);
        assert_eq!(mapping.get(&sources[0]).unwrap(), &PathBuf::from("a.s"));
        assert_eq!(mapping.get(&sources[1]).unwrap(), &PathBuf::from("b.s"));
        
        // Test case 2: Conflict in filenames
        let sources = vec![
            PathBuf::from("src/foo/util.s"),
            PathBuf::from("src/bar/util.s"),
            PathBuf::from("long/path/to/other/not-util.s"),
        ];
        let mapping = compiler.create_output_path_mapping(&sources);
        assert_eq!(mapping.len(), 3);
        assert_eq!(mapping.get(&sources[0]).unwrap(), &PathBuf::from("foo/util.s"));
        assert_eq!(mapping.get(&sources[1]).unwrap(), &PathBuf::from("bar/util.s"),
        "Conflicting util.s files should include parent directory");
        assert_eq!(mapping.get(&sources[2]).unwrap(), &PathBuf::from("not-util.s"));
    }
} 