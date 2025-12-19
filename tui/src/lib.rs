//! TUI library for interaction-tree monitoring.
//!
//! This library provides the core components for the it-tui application,
//! including app state management, UI rendering, and WebSocket communication.

mod address;
pub mod app;
pub mod chat;
mod commands;
mod event;
pub mod flutter_log;
mod markdown;
pub mod project;
pub mod session;
pub mod theme;
mod tree_format;
pub mod types;
pub mod ui;
pub mod ws;
