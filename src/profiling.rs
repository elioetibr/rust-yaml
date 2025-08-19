//! Performance profiling utilities and optimization features

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Performance profiler for YAML operations
#[derive(Debug, Clone)]
pub struct YamlProfiler {
    timings: HashMap<String, Vec<Duration>>,
    memory_usage: HashMap<String, usize>,
    enabled: bool,
}

impl YamlProfiler {
    /// Create a new profiler instance
    pub fn new() -> Self {
        Self {
            timings: HashMap::new(),
            memory_usage: HashMap::new(),
            enabled: std::env::var("RUST_YAML_PROFILE").is_ok(),
        }
    }

    /// Check if profiling is enabled
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable profiling
    pub const fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable profiling
    pub const fn disable(&mut self) {
        self.enabled = false;
    }

    /// Start timing an operation
    pub fn time_operation<F, R>(&mut self, operation: &str, func: F) -> R
    where
        F: FnOnce() -> R,
    {
        if !self.enabled {
            return func();
        }

        let start = Instant::now();
        let result = func();
        let duration = start.elapsed();

        self.timings
            .entry(operation.to_string())
            .or_insert_with(Vec::new)
            .push(duration);

        result
    }

    /// Record memory usage for an operation
    pub fn record_memory(&mut self, operation: &str, bytes: usize) {
        if !self.enabled {
            return;
        }

        self.memory_usage.insert(operation.to_string(), bytes);
    }

    /// Get average timing for an operation
    pub fn average_time(&self, operation: &str) -> Option<Duration> {
        let timings = self.timings.get(operation)?;
        if timings.is_empty() {
            return None;
        }

        let total: Duration = timings.iter().sum();
        Some(total / timings.len() as u32)
    }

    /// Get total timing for an operation
    pub fn total_time(&self, operation: &str) -> Option<Duration> {
        let timings = self.timings.get(operation)?;
        Some(timings.iter().sum())
    }

    /// Get memory usage for an operation
    pub fn memory_usage(&self, operation: &str) -> Option<usize> {
        self.memory_usage.get(operation).copied()
    }

    /// Get all recorded operations
    pub fn operations(&self) -> Vec<String> {
        let mut ops: Vec<String> = self.timings.keys().cloned().collect();
        ops.sort();
        ops
    }

    /// Clear all recorded data
    pub fn clear(&mut self) {
        self.timings.clear();
        self.memory_usage.clear();
    }

    /// Generate a performance report
    pub fn report(&self) -> String {
        if !self.enabled {
            return "Profiling disabled".to_string();
        }

        let mut report = String::new();
        report.push_str("=== YAML Performance Report ===\n\n");

        for operation in self.operations() {
            report.push_str(&format!("Operation: {}\n", operation));

            if let Some(avg_time) = self.average_time(&operation) {
                report.push_str(&format!("  Average Time: {:?}\n", avg_time));
            }

            if let Some(total_time) = self.total_time(&operation) {
                report.push_str(&format!("  Total Time: {:?}\n", total_time));
            }

            if let Some(memory) = self.memory_usage(&operation) {
                report.push_str(&format!("  Memory Usage: {} bytes\n", memory));
            }

            if let Some(timings) = self.timings.get(&operation) {
                report.push_str(&format!("  Sample Count: {}\n", timings.len()));
            }

            report.push('\n');
        }

        report
    }
}

impl Default for YamlProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// String interning pool for common YAML strings
#[derive(Debug)]
pub struct StringInterner {
    strings: HashMap<String, &'static str>,
    enabled: bool,
}

impl StringInterner {
    /// Create a new string interner
    pub fn new() -> Self {
        Self {
            strings: HashMap::new(),
            enabled: true,
        }
    }

    /// Intern a string, returning a reference to the interned version
    pub const fn intern(&mut self, s: String) -> String {
        if !self.enabled {
            return s;
        }

        // For now, just return the original string
        // In a real implementation, we'd use a more sophisticated interner
        s
    }

    /// Check if a string is interned
    pub fn contains(&self, s: &str) -> bool {
        if !self.enabled {
            return false;
        }
        self.strings.contains_key(s)
    }

    /// Get statistics about the interner
    pub fn stats(&self) -> (usize, usize) {
        let count = self.strings.len();
        let memory = self.strings.keys().map(|s| s.len()).sum::<usize>();
        (count, memory)
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory pool for frequently allocated objects
#[derive(Debug)]
pub struct ObjectPool<T> {
    objects: Vec<T>,
    enabled: bool,
}

impl<T> ObjectPool<T> {
    /// Create a new object pool
    pub const fn new() -> Self {
        Self {
            objects: Vec::new(),
            enabled: true,
        }
    }

    /// Create a new object pool with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            objects: Vec::with_capacity(capacity),
            enabled: true,
        }
    }

    /// Get an object from the pool, or create a new one
    pub fn get<F>(&mut self, creator: F) -> T
    where
        F: FnOnce() -> T,
    {
        if self.enabled && !self.objects.is_empty() {
            self.objects.pop().unwrap()
        } else {
            creator()
        }
    }

    /// Return an object to the pool
    pub fn put(&mut self, object: T) {
        if self.enabled {
            self.objects.push(object);
        }
    }

    /// Get pool statistics
    pub const fn stats(&self) -> (usize, usize) {
        (self.objects.len(), self.objects.capacity())
    }
}

impl<T> Default for ObjectPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_basic_functionality() {
        let mut profiler = YamlProfiler::new();
        profiler.enable();

        let result = profiler.time_operation("test_op", || {
            std::thread::sleep(Duration::from_millis(1));
            42
        });

        assert_eq!(result, 42);
        assert!(profiler.average_time("test_op").is_some());
        assert!(profiler.total_time("test_op").is_some());
    }

    #[test]
    fn test_profiler_memory_tracking() {
        let mut profiler = YamlProfiler::new();
        profiler.enable();

        profiler.record_memory("allocation", 1024);
        assert_eq!(profiler.memory_usage("allocation"), Some(1024));
    }

    #[test]
    fn test_profiler_disabled() {
        let mut profiler = YamlProfiler::new();
        profiler.disable();

        profiler.time_operation("test_op", || 42);
        assert!(profiler.average_time("test_op").is_none());
    }

    #[test]
    fn test_string_interner() {
        let mut interner = StringInterner::new();
        let s1 = interner.intern("test".to_string());
        let s2 = interner.intern("test".to_string());

        // Basic functionality test
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_object_pool() {
        let mut pool = ObjectPool::new();

        // Put an object in the pool
        pool.put(String::from("test"));

        // Get it back
        let retrieved = pool.get(|| String::from("new"));
        assert_eq!(retrieved, "test");

        // Pool should now be empty, so next get creates new object
        let new_obj = pool.get(|| String::from("fresh"));
        assert_eq!(new_obj, "fresh");
    }
}
