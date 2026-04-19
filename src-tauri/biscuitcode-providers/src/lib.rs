//! `biscuitcode-providers` — streaming chat providers behind a unified trait.
//!
//! Design contract: `docs/design/PROVIDER-TRAIT.md`.
//!
//! Public surface:
//! - [`ModelProvider`] — the trait every provider implements
//! - [`ChatEvent`] — the normalized streaming event enum
//! - [`Message`], [`ToolSpec`], [`ChatOptions`], [`Usage`] — shared types
//! - [`ProviderError`] — errors that map to catalogue codes (E004 etc.)
//! - Per-provider impls in [`anthropic`], [`openai`], [`ollama`]
//!
//! Phase 5 fills in [`anthropic`]; Phase 6a fills in [`openai`] and [`ollama`].

#![warn(missing_docs)]

pub mod anthropic;
pub mod r#trait;
pub mod types;

pub use r#trait::ModelProvider;
pub use types::{
    ChatEvent, ChatOptions, ContentBlock, MentionKind, Message, MessageRole,
    ModelInfo, ProviderError, Role, ToolCall, ToolResult, ToolSpec, Usage,
};
