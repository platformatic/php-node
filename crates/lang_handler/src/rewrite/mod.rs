// use std::ops::{Deref, DerefMut};

// use super::Request;

mod condition;
mod conditional_rewriter;
mod rewriter;

pub use condition::*;
pub use conditional_rewriter::ConditionalRewriter;
pub use rewriter::*;
