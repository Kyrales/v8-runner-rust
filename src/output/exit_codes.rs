/// Exit codes for v8-test-runner
pub const SUCCESS: i32 = 0;
/// Validation error (bad config, bad args)
pub const VALIDATION_ERROR: i32 = 2;
/// Runtime error (platform command failed, parse error)
pub const RUNTIME_ERROR: i32 = 3;
/// Platform error (binary not found, process spawn failed)
pub const PLATFORM_ERROR: i32 = 4;
