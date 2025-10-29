use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use std::env;
use std::fmt;

use crate::compiler::{Compiler, create_compiler};
use crate::config::Config;
use crate::executor::{Executor, CommandExecutor};

pub struct TestRunner {
    config: Config,
    work_dir: PathBuf,
    compiler: Box<dyn Compiler>,
    verbose: bool,
    dry_run: bool,
    tester_executor: Box<dyn Executor>,
}

impl fmt::Debug for TestRunner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestRunner")
            .field("config", &self.config)
            .field("work_dir", &self.work_dir)
            .field("compiler", &"<dyn Compiler>")
            .field("verbose", &self.verbose)
            .field("dry_run", &self.dry_run)
            .field("tester_executor", &"<dyn Executor>")
            .finish()
    }
}

impl TestRunner {
    pub fn from_yaml(test_yaml: &Path, build_dir: Option<PathBuf>, verbose: bool, dry_run: bool) -> Result<Self> {
        // Determine build directory - command line takes precedence over environment variable
        let work_dir = build_dir.or_else(|| env::var("SOFT65C02_BUILD_DIR").ok().map(PathBuf::from))
            .ok_or_else(|| anyhow::anyhow!("Build directory must be specified either via --build-dir option or SOFT65C02_BUILD_DIR environment variable"))?;

        if verbose || dry_run {
            println!("Loading test config from: {:?}", test_yaml);
            println!("Build directory: {:?}", work_dir);
        }
        
        // Load test configuration
        let config = Config::load(test_yaml)
            .with_context(|| format!("Failed to load test config from {}", test_yaml.display()))?;
        
        // Get compiler from config
        let compiler_type = config.compiler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No compiler specified in config"))?;

        // Create and clean the build directory
        if !dry_run {
            if work_dir.exists() {
                std::fs::remove_dir_all(&work_dir)?;
            }
            std::fs::create_dir_all(&work_dir)?;
        } else {
            println!("[DRY RUN] Would remove and recreate build directory: {:?}", work_dir);
        }

        // Create compiler implementation
        let compiler = create_compiler(compiler_type, &config, verbose, dry_run)?;
        
        Ok(Self {
            config,
            work_dir,
            compiler,
            verbose,
            dry_run,
            tester_executor: Box::new(CommandExecutor::with_verbose("soft65c02_tester", false)),
        })
    }

    pub fn run(self) -> Result<()> {
        let (binary_path, symbols_path) = self.compile()?;
        self.run_tests(&binary_path, Some(&symbols_path))?;
        Ok(())
    }

    fn compile(&self) -> Result<(PathBuf, PathBuf)> {
        let mut object_files = Vec::new();
        
        // Compile all source files
        if let Some(src_files) = &self.config.src_files {
            // Create path mapping for the already canonicalized paths
            let path_mapping = self.compiler.create_output_path_mapping(src_files);

            // Compile each source file using the mapping
            for src in src_files {
                let obj = self.compiler.compile_source(src, &self.work_dir, &path_mapping)?;
                object_files.push(obj);
            }
        }

        // Link everything together
        let binary_path = self.work_dir.join("app.bin");
        let symbols_path = self.compiler.get_symbols_path(&self.work_dir);
        
        self.compiler.link_objects(&object_files, &binary_path, &self.work_dir)?;

        Ok((binary_path, symbols_path))
    }

    fn run_tests(&self, binary_path: &Path, symbols_path: Option<&Path>) -> Result<()> {
        // Set up environment variables for the test script
        env::set_var("BUILD_DIR", &self.work_dir);
        env::set_var("BINARY_PATH", binary_path);
        if let Some(symbols) = symbols_path {
            env::set_var("SYMBOLS_PATH", symbols);
        }

        if self.verbose {
            println!("Setting test environment variables:");
            println!("  BUILD_DIR={}", self.work_dir.display());
            println!("  BINARY_PATH={}", binary_path.display());
            if let Some(symbols) = symbols_path {
                println!("  SYMBOLS_PATH={}", symbols.display());
            }
        }
        
        // Build command arguments
        let mut args = Vec::new();
        
        if self.verbose {
            args.push("-v".to_string());
        }

        if let Some(test_script) = &self.config.test_script {
            args.extend(["-i".to_string(), test_script.to_string_lossy().to_string()]);
        } else {
            anyhow::bail!("No test script specified in config");
        }

        if self.verbose || self.dry_run {
            println!("Executing: soft65c02_tester {}", args.join(" "));
        }
        
        if self.dry_run {
            println!("[DRY RUN] Would execute soft65c02_tester");
            return Ok(());
        }
        
        // Just propagate the error directly without wrapping
        self.tester_executor.execute(&args)
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use crate::config::CompilerType;
    use crate::compiler::cc65;
    // use crate::executor::tests::MockExecutor;

    // Helper function to create a basic test environment
    fn setup_test_env() -> (TempDir, PathBuf, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.yaml");
        let build_dir = temp_dir.path().join("build");

        // Create test files
        let test_script = temp_dir.path().join("test.s65");
        let config_file = temp_dir.path().join("config.cfg");
        let src_file = temp_dir.path().join("main.s");

        // Create a simple source file
        fs::write(&src_file, r#"
            .export _main
            _main:
                lda #42
                rts
        "#).unwrap();

        // Create test script
        fs::write(&test_script, "fake script").unwrap();

        // Create config file with basic linker configuration
        fs::write(&config_file, "fake config").unwrap();

        // Create config YAML
        let content = format!(r#"
compiler: cc65
target: nes
test_script: {}
config_file: {}
src_files:
  - {}
"#,
            test_script.display(),
            config_file.display(),
            src_file.display(),
        );
        fs::write(&config_path, content).unwrap();

        (temp_dir, config_path, build_dir)
    }

    #[test]
    fn test_from_yaml_missing_build_dir() {
        // Clear the environment variable if it exists
        env::remove_var("SOFT65C02_BUILD_DIR");

        // Create a temporary directory for our test files
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.yaml");

        // Create a minimal valid config file
        let content = r#"
compiler: cc65
target: mock
test_script: test.s65
"#;
        fs::write(&config_path, content).unwrap();

        // Try to create TestRunner without build_dir
        let result = TestRunner::from_yaml(
            &config_path,
            None,  // No build directory specified
            false, // Not verbose
            false, // Not dry run
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Build directory must be specified"));
    }

    #[test]
    fn test_from_yaml_missing_config() {
        // Create a temporary directory for our test files
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.yaml");
        let build_dir = temp_dir.path().join("build");

        // Try to create TestRunner with non-existent config file
        let result = TestRunner::from_yaml(
            &config_path,
            Some(build_dir),
            false,
            false,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to load test config"));
    }

    #[test]
    fn test_from_yaml_missing_compiler() {
        // Create a temporary directory for our test files
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.yaml");
        let build_dir = temp_dir.path().join("build");

        // Create a config file without compiler specification
        let content = r#"
target: mock
test_script: test.s65
"#;
        fs::write(&config_path, content).unwrap();

        // Try to create TestRunner with config missing compiler
        let result = TestRunner::from_yaml(
            &config_path,
            Some(build_dir),
            false,
            false,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No compiler specified in config"));
    }

    #[test]
    fn test_from_yaml_dry_run() {
        // Create a temporary directory for our test files
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.yaml");
        let build_dir = temp_dir.path().join("build");

        // Create test files to reference in the config
        let test_script = temp_dir.path().join("test.s65");
        let config_file = temp_dir.path().join("config.cfg");
        fs::write(&test_script, "").unwrap();  // Create empty files
        fs::write(&config_file, "").unwrap();

        // Create a valid config file with relative paths
        let content = format!(r#"
compiler: cc65
target: mock
test_script: {}
config_file: {}
"#,
            test_script.display(),
            config_file.display(),
        );
        fs::write(&config_path, content).unwrap();

        // Create TestRunner in dry run mode
        let result = TestRunner::from_yaml(
            &config_path,
            Some(build_dir.clone()),
            true,  // verbose
            true,  // dry run
        );

        assert!(result.is_ok());
        let runner = result.unwrap();
        assert_eq!(runner.work_dir, build_dir);
        assert!(runner.verbose);
        assert!(runner.dry_run);
        assert_eq!(runner.config.compiler, Some(CompilerType::CC65));
        assert_eq!(runner.config.target, Some("mock".to_string()));
        assert_eq!(runner.config.test_script.as_ref().map(|p| p.canonicalize().unwrap()), Some(test_script.canonicalize().unwrap()));
        assert_eq!(runner.config.config_file.as_ref().map(|p| p.canonicalize().unwrap()), Some(config_file.canonicalize().unwrap()));
    }

    #[test]
    fn test_compile_success() {
        let (_temp_dir, config_path, build_dir) = setup_test_env();

        // Create TestRunner with mock executor
        let config = Config::load(&config_path).unwrap();
        let compiler = cc65::CC65Compiler::with_mock_executor(
            &config,
            false,  // verbose
            false, // not dry run
            Box::new(crate::executor::tests::MockExecutor::new(vec![
                Ok(()), // For compilation
                Ok(()), // For linking
            ])),
        ).unwrap();

        let runner = TestRunner {
            config,
            work_dir: build_dir.clone(),
            compiler: Box::new(compiler),
            verbose: false,
            dry_run: false,
            tester_executor: Box::new(crate::executor::tests::MockExecutor::new(vec![])),
        };

        // Run compilation
        let result = runner.compile();
        assert!(result.is_ok());

        // Verify the returned paths are correct
        let (binary_path, symbols_path) = result.unwrap();
        assert_eq!(binary_path, build_dir.join("app.bin"), "Binary path should be app.bin in the build directory");
        assert_eq!(symbols_path, build_dir.join("app.lbl"), "Symbols path should be app.lbl in the build directory");
    }

    #[test]
    fn test_compile_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.yaml");
        let build_dir = temp_dir.path().join("build");
        let src_file = temp_dir.path().join("main.s");

        // Create an invalid source file
        fs::write(&src_file, "this is not valid assembly").unwrap();

        // Create config YAML
        let content = format!(r#"
compiler: cc65
target: mock
config_file: config.cfg
src_files:
  - {}
"#,
            src_file.display(),
        );
        fs::write(&config_path, content).unwrap();

        // Create TestRunner with mock executor that will fail
        let config = Config::load(&config_path).unwrap();
        let compiler = cc65::CC65Compiler::with_mock_executor(
            &config,
            false,
            false,
            Box::new(crate::executor::tests::MockExecutor::new(vec![
                Err("Mock compilation error".to_string()),
            ])),
        ).unwrap();

        let runner = TestRunner {
            config,
            work_dir: build_dir.clone(),
            compiler: Box::new(compiler),
            verbose: false,
            dry_run: false,
            tester_executor: Box::new(crate::executor::tests::MockExecutor::new(vec![])),
        };

        // Run compilation
        let result = runner.compile();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Mock compilation error"));
    }

    #[test]
    fn test_run_tests_success() {
        let (_temp_dir, config_path, build_dir) = setup_test_env();

        // Create TestRunner with mock executor
        let config = Config::load(&config_path).unwrap();
        let compiler = cc65::CC65Compiler::with_mock_executor(
            &config,
            false,
            false,
            Box::new(crate::executor::tests::MockExecutor::new(vec![
                Ok(()), // For compilation
                Ok(()), // For linking
            ])),
        ).unwrap();

        let runner = TestRunner {
            config,
            work_dir: build_dir.clone(),
            compiler: Box::new(compiler),
            verbose: false,
            dry_run: false,
            tester_executor: Box::new(crate::executor::tests::MockExecutor::new(vec![])),
        };

        // Run compilation first
        let (binary_path, symbols_path) = runner.compile().unwrap();

        // Create the output files since we're mocking the compiler
        fs::create_dir_all(binary_path.parent().unwrap()).unwrap();
        fs::write(&binary_path, "mock binary").unwrap();
        fs::write(&symbols_path, "mock symbols").unwrap();

        // Run tests
        let result = runner.run_tests(&binary_path, Some(&symbols_path));
        assert!(result.is_ok());

        // Verify environment variables were set
        assert_eq!(env::var("BUILD_DIR").unwrap(), build_dir.to_string_lossy().to_string());
        assert_eq!(env::var("BINARY_PATH").unwrap(), binary_path.to_string_lossy().to_string());
        assert_eq!(env::var("SYMBOLS_PATH").unwrap(), symbols_path.to_string_lossy().to_string());
    }

    #[test]
    fn test_run_tests_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.yaml");
        let build_dir = temp_dir.path().join("build");
        let test_script = temp_dir.path().join("test.s65");

        // Create a failing test script
        fs::write(&test_script, r#"
            load binary
            assert a == 99  # This will fail
        "#).unwrap();

        // Create config YAML
        let content = format!(r#"
compiler: cc65
target: mock
test_script: {}
config_file: config.cfg
"#,
            test_script.display(),
        );
        fs::write(&config_path, content).unwrap();

        // Create TestRunner with mock executor
        let config = Config::load(&config_path).unwrap();
        let compiler = cc65::CC65Compiler::with_mock_executor(
            &config,
            false,
            false,
            Box::new(crate::executor::tests::MockExecutor::new(vec![
                Ok(()), // For compilation
                Ok(()), // For linking
            ])),
        ).unwrap();

        // Create a mock executor that simulates a test failure
        let mock_tester = crate::executor::tests::MockExecutor::new(vec![
            Err("Test assertion failed: expected a == 99".to_string()),  // Simulate test failure
        ]);

        let runner = TestRunner {
            config,
            work_dir: build_dir.clone(),
            compiler: Box::new(compiler),
            verbose: false,
            dry_run: false,
            tester_executor: Box::new(mock_tester),
        };

        // Run compilation first
        let (binary_path, symbols_path) = runner.compile().unwrap();

        // Create the output files since we're mocking the compiler
        fs::create_dir_all(binary_path.parent().unwrap()).unwrap();
        fs::write(&binary_path, "mock binary").unwrap();
        fs::write(&symbols_path, "mock symbols").unwrap();

        // Run tests - should fail
        let result = runner.run_tests(&binary_path, Some(&symbols_path));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Test assertion failed"));
    }

    #[test]
    fn test_full_run() {
        let (_temp_dir, config_path, build_dir) = setup_test_env();

        // Create TestRunner with mock executor
        let config = Config::load(&config_path).unwrap();
        let compiler = cc65::CC65Compiler::with_mock_executor(
            &config,
            false,
            false,
            Box::new(crate::executor::tests::MockExecutor::new(vec![
                Ok(()), // For compilation
                Ok(()), // For linking
            ])),
        ).unwrap();

        // Create a mock executor for the tester command
        let mock_tester = crate::executor::tests::MockExecutor::new(vec![
            Ok(()),  // For tester execution
        ]);
        
        let runner = TestRunner {
            config,
            work_dir: build_dir.clone(),
            compiler: Box::new(compiler),
            verbose: false,
            dry_run: false,
            tester_executor: Box::new(mock_tester),
        };

        // Run the full process
        let result = runner.run();
        assert!(result.is_ok(), "Runner failed with error: {:?}", result.err().unwrap());
    }
} 