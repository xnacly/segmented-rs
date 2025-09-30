//! A segmented list and bump allocator ripped out and ported from purple garden
//!
//! 0 Dependencies, high performance, 0 locks, not thread safe

/// Segmented bump allocator
pub mod alloc;
/// Segmented list
pub mod list;
