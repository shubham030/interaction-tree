//! TUI library for interaction-tree monitoring.
//!
//! This library provides the core components for the it-tui application,
//! including app state management, UI rendering, and WebSocket communication.
//!
//! Note: This library is primarily compiled as part of the binary.
//! Internal modules may show as unused when compiled as a standalone library.
#![allow(dead_code)]

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
