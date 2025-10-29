use std::process::Command;

// This is used for abstracting process commands so we can test the generation of the commands that
// would be executed without actually performing the execution, and testing error scenarios.

/// Trait for executing commands
pub trait Executor {
    fn execute(&self, args: &[String]) -> Result<(), String>;
}

/// A command executor that captures the executable name
pub struct CommandExecutor {
    executable: String,
    verbose: bool,
}

impl CommandExecutor {
    pub fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
            verbose: false,
        }
    }

    pub fn with_verbose(executable: impl Into<String>, verbose: bool) -> Self {
        Self {
            executable: executable.into(),
            verbose,
        }
    }
}

impl Executor for CommandExecutor {
    fn execute(&self, args: &[String]) -> Result<(), String> {
        if self.verbose {
            println!("Executing command: {} {}", self.executable, args.join(" "));
            println!("Environment variables:");
            for (key, value) in std::env::vars() {
                println!("  {}={}", key, value);
            }
        }

        let output = Command::new(&self.executable)
            .args(args)
            .envs(std::env::vars())  // Pass through all environment variables
            .output()
            .map_err(|e| format!("Failed to execute {}: {}", self.executable, e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Always show stdout
        if !stdout.is_empty() {
            print!("{}", stdout);
        }

        let error_msg = stderr.trim();
        if !error_msg.starts_with("Error:") {
            eprint!("{}", stderr);
        }

        if !output.status.success() {
            // If stderr starts with "Error:", just return that message without wrapping
            if error_msg.starts_with("Error:") {
                Err(error_msg[6..].trim().to_string())
            } else {
                Err(stderr.into_owned())
            }
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Mock executor that records commands and returns predefined results
    pub struct MockExecutor {
        commands: Rc<RefCell<Vec<Vec<String>>>>,
        results: Vec<Result<(), String>>,
    }

    impl MockExecutor {
        pub fn new(results: Vec<Result<(), String>>) -> Self {
            Self {
                commands: Rc::new(RefCell::new(Vec::new())),
                results,
            }
        }

        pub fn get_commands(&self) -> Vec<Vec<String>> {
            self.commands.borrow().clone()
        }
    }

    impl Executor for MockExecutor {
        fn execute(&self, args: &[String]) -> Result<(), String> {
            self.commands.borrow_mut().push(args.to_vec());
            self.results.get(self.commands.borrow().len() - 1)
                .cloned()
                .unwrap_or(Ok(()))
        }
    }

    #[test]
    fn test_command_executor_success() {
        let executor = CommandExecutor::new("echo");
        let args = vec!["hello".to_string()];
        assert!(executor.execute(&args).is_ok());
    }

    #[test]
    fn test_command_executor_failure() {
        let executor = CommandExecutor::new("nonexistent");
        let args = vec!["test".to_string()];
        assert!(executor.execute(&args).is_err());
    }

    #[test]
    fn test_mock_executor() {
        let mock = MockExecutor::new(vec![
            Ok(()),
            Err("mock error".to_string()),
            Ok(()),
        ]);

        let args1 = vec!["test1".to_string()];
        let args2 = vec!["test2".to_string()];
        let args3 = vec!["test3".to_string()];

        assert!(mock.execute(&args1).is_ok());
        assert!(mock.execute(&args2).is_err());
        assert!(mock.execute(&args3).is_ok());

        let commands = mock.get_commands();
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0], args1);
        assert_eq!(commands[1], args2);
        assert_eq!(commands[2], args3);
    }
} 