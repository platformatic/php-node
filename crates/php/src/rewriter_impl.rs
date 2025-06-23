// Re-export common rewriter types for convenience
pub use http_rewriter::{
    PathRewriter, HeaderRewriter, MethodRewriter, HrefRewriter,
    PathCondition, HeaderCondition, MethodCondition, ExistenceCondition, NonExistenceCondition,
    Rewriter, RewriteError,
};