//! # Request Rewriting
//!
//! There are two sets of tools to manage request rewriting:
//!
//! - A [`Condition`] matches existing Request state by some given criteria.
//! - A [`Rewriter`] applies replacement logic to produce new Request state.
//!
//! # Conditions
//!
//! There are several types of [`Condition`] for matching Request state:
//!
//! - [`HeaderCondition`] matches if named header matches the given pattern.
//! - [`PathCondition`] matches if Request path matches the given pattern.
//! - [`ExistenceCondition`] matches if Request path resolves to a real file.
//! - [`NonExistenceCondition`] matches if Request path does not resolve.
//!
//! In addition to these core types, any function with a `Fn(&Request) -> bool`
//! signature may also be used anywhere a [`Condition`] is expected. This
//! allows any arbitrary logic to be applied to decide a match. Because a
//! Request may be dispatched to any thread, these functions must be
//! `Send + Sync`.
//!
//! ```
//! # use lang_handler::{Request, rewrite::Condition};
//! let condition = |request: &Request| -> bool {
//!   request.url().path().starts_with("/foo")
//! };
//! ```
//!
//! Multiple [Condition] types may be grouped together to form logical
//! conditions using `condition.and(other)` or `condition.or(other)` to apply
//! conditions with AND or OR logic respectively.
//!
//! # Rewriters
//!
//! There are several types of [`Rewriter`] for rewriting Request state:
//!
//! - [`HeaderRewriter`] rewrites named header using pattern and replacement.
//! - [`PathRewriter`] rewrites Request path using pattern and replacement.
//!
//! As with [`Condition`], any function with a `Fn(Request) -> Request`
//! signature may also be used anywhere a [`Rewriter`] is accepted. This allows
//! any custom logic to be used to produce a rewritten Request. Because a
//! Request may be dispatched to any thread, these functions must be
//! `Send + Sync`.
//!
//! ```
//! # use lang_handler::{Request, RequestBuilderException, rewrite::Rewriter};
//! let rewriter = |request: Request| -> Result<Request, RequestBuilderException> {
//!   request.extend()
//!     .url("http://example.com/rewritten")
//!     .build()
//! };
//! ```
//!
//! Multiple Rewriters may be sequenced using `rewriter.then(other)` to apply
//! in order.
//!
//! # Combining Conditions and Rewriters
//!
//! Rewriters on their own _always_ apply, but this is generally not desirable
//! so Conditions exist to switch their application on or off. This is done
//! using `rewriter.when(condition)` to apply a [`Rewriter`] only when the given
//! [`Condition`] matches.
//!
//! # Complex sequencing
//!
//! Using the condition grouping and rewriter sequencing combinators, one can
//! achieve some quite complex rewriting logic.
//!
//! ```rust
//! # use lang_handler::rewrite::{
//! #   Condition,
//! #   ConditionExt,
//! #   HeaderCondition,
//! #   PathCondition,
//! #   Rewriter,
//! #   RewriterExt,
//! #   PathRewriter
//! # };
//! #
//! let admin = {
//!   let is_admin_path = PathCondition::new("^/admin")
//!     .expect("regex is valid");
//!
//!   let is_admin_header = HeaderCondition::new("ADMIN_PASSWORD", "not-very-secure")
//!     .expect("regex is valid");
//!
//!   let is_bypass = HeaderCondition::new("DEV_BYPASS", "do-not-use-this")
//!     .expect("regex is valid");
//!
//!   let admin_conditions = is_admin_path
//!     .and(is_admin_header)
//!     .or(is_bypass);
//!
//!   let admin_rewrite = PathRewriter::new("^(/admin)", "/secret")
//!     .expect("regex is valid");
//!
//!   admin_rewrite.when(admin_conditions)
//! };
//!
//! let login = {
//!   let condition = PathCondition::new("^/login$")
//!     .expect("regex is valid");
//!
//!   let rewriter = PathRewriter::new(".*", "/auth")
//!     .expect("regex is valid");
//!
//!   rewriter.when(condition)
//! };
//!
//! let rewrite_rules = admin.then(login);
//! ```

mod condition;
mod conditional_rewriter;
mod rewriter;

pub use condition::*;
pub use conditional_rewriter::ConditionalRewriter;
pub use rewriter::*;
