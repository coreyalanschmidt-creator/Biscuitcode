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

#![allow(missing_docs)] // TODO: document public items and flip back to warn

pub mod anthropic;
pub mod ollama;
pub mod openai;
pub mod r#trait;
pub mod types;

pub use r#trait::ModelProvider;
pub use types::{
    ChatEvent, ChatOptions, ContentBlock, MentionKind, Message, MessageRole, ModelInfo,
    ProviderError, ReasoningEffort, Role, ToolCall, ToolResult, ToolSpec, Usage,
};

pub use anthropic::AnthropicProvider;
pub use ollama::{
    agent_mode_preferred, gemma3_fallback_for_ram_gb, gemma4_tag_for_ram_gb, OllamaProvider,
};
pub use openai::OpenAIProvider;
