use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "v8-test-runner", about = "1C:Enterprise test runner CLI")]
pub struct Cli {
    /// Path to YAML config file
    #[arg(long, global = true, env = "V8TR_CONFIG")]
    pub config: Option<String>,

    /// Output format
    #[arg(long, global = true, default_value = "text", value_parser = ["text", "json"])]
    pub output: String,

    /// Log level
    #[arg(long, global = true, default_value = "info",
          value_parser = ["error", "warn", "info", "debug", "trace"])]
    pub log_level: Option<String>,

    /// Clear log files before execution
    #[arg(long, global = true)]
    pub clean_before_execution: bool,

    /// Disable ANSI colors
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Override working directory
    #[arg(long, global = true)]
    pub workdir: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Load sources into infobase
    Build(BuildArgs),
    /// Run YaXUnit tests
    Test(TestArgs),
    /// Dump configuration from infobase to files
    Dump(DumpArgs),
    /// Run syntax checks
    Syntax(SyntaxArgs),
    /// Launch 1C application
    Launch(LaunchArgs),
}

#[derive(Args, Debug)]
pub struct BuildArgs {
    /// Clear change cache and rebuild everything
    #[arg(long)]
    pub full_rebuild: bool,
}

#[derive(Args, Debug)]
pub struct TestArgs {
    #[command(subcommand)]
    pub scope: TestScope,
}

#[derive(Subcommand, Debug)]
pub enum TestScope {
    /// Run all tests
    All,
    /// Run tests for a specific module
    Module {
        /// Module name
        name: String,
    },
}

#[derive(Args, Debug)]
pub struct DumpArgs {
    /// Dump mode
    #[arg(long, value_parser = ["full", "incremental", "partial"])]
    pub mode: String,

    /// Source set name
    #[arg(long)]
    pub source_set: Option<String>,

    /// Extension name
    #[arg(long)]
    pub extension: Option<String>,

    /// Objects for partial dump (TYPE:NAME)
    #[arg(long = "object")]
    pub objects: Vec<String>,
}

#[derive(Args, Debug)]
pub struct SyntaxArgs {
    #[command(subcommand)]
    pub target: SyntaxTarget,
}

#[derive(Subcommand, Debug)]
pub enum SyntaxTarget {
    /// Check configuration via Designer CheckConfig
    DesignerConfig,
    /// Check modules via Designer CheckModules
    DesignerModules,
    /// Check via EDT validate
    Edt {
        /// EDT project names
        #[arg(long = "project")]
        projects: Vec<String>,
    },
}

#[derive(Args, Debug)]
pub struct LaunchArgs {
    /// Launch mode
    #[arg(long, value_parser = ["designer", "thin", "thick"])]
    pub mode: String,
}
