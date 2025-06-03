use std::str::FromStr;

// use napi::bindgen_prelude::*;
use napi::{Error, Result};

use php::{
  rewrite::{
    Condition, ConditionExt, HeaderCondition, HeaderRewriter, PathCondition, PathRewriter,
    Rewriter, RewriterExt,
  },
  Request,
};

use crate::PhpRequest;

//
// Conditions
//

#[napi(object)]
#[derive(Debug, Default)]
pub struct PhpRewriteCondOptions {
  #[napi(js_name = "type")]
  pub cond_type: String,
  pub args: Vec<String>,
}

pub enum PhpRewriteCond {
  Path(String),
  Header(String, String),
}

impl Condition for PhpRewriteCond {
  fn matches(&self, request: &php::Request) -> bool {
    let condition: Box<dyn Condition> = match self {
      PhpRewriteCond::Path(pattern) => PathCondition::new(pattern.as_str()).unwrap(),
      PhpRewriteCond::Header(name, pattern) => {
        HeaderCondition::new(name.as_str(), pattern.as_str()).unwrap()
      }
    };

    condition.matches(request)
  }
}

impl TryFrom<&PhpRewriteCondOptions> for Box<PhpRewriteCond> {
  type Error = Error;

  fn try_from(value: &PhpRewriteCondOptions) -> std::result::Result<Self, Self::Error> {
    let PhpRewriteCondOptions { cond_type, args } = value;
    let cond_type = cond_type.to_lowercase();
    match cond_type.as_str() {
      "path" => match args.len() {
        1 => Ok(Box::new(PhpRewriteCond::Path(args[0].to_owned()))),
        _ => Err(Error::from_reason("Wrong number of parameters")),
      },
      "header" => match args.len() {
        2 => {
          let name = args[0].to_owned();
          let pattern = args[1].to_owned();
          Ok(Box::new(PhpRewriteCond::Header(name, pattern)))
        }
        _ => Err(Error::from_reason("Wrong number of parameters")),
      },
      _ => Err(Error::from_reason(format!(
        "Unknown condition type: {}",
        cond_type
      ))),
    }
  }
}

//
// Rewriters
//

#[napi(object)]
#[derive(Debug, Default)]
pub struct PhpRewriterOptions {
  #[napi(js_name = "type")]
  pub rewriter_type: String,
  pub args: Vec<String>,
}

pub enum PhpRewriterType {
  Path(String, String),
  Header(String, String, String),
}

impl Rewriter for PhpRewriterType {
  fn rewrite(&self, request: Request) -> Request {
    let rewriter: Box<dyn Rewriter> = match self {
      PhpRewriterType::Path(pattern, replacement) => {
        PathRewriter::new(pattern.as_str(), replacement.as_str()).unwrap()
      }
      PhpRewriterType::Header(name, pattern, replacement) => {
        HeaderRewriter::new(name.as_str(), pattern.as_str(), replacement.as_str()).unwrap()
      }
    };

    rewriter.rewrite(request)
  }
}

impl TryFrom<&PhpRewriterOptions> for Box<PhpRewriterType> {
  type Error = Error;

  fn try_from(value: &PhpRewriterOptions) -> std::result::Result<Self, Self::Error> {
    let PhpRewriterOptions {
      rewriter_type,
      args,
    } = value;
    let rewriter_type = rewriter_type.to_lowercase();
    match rewriter_type.as_str() {
      "path" => match args.len() {
        2 => {
          let pattern = args[0].to_owned();
          let replacement = args[1].to_owned();
          Ok(Box::new(PhpRewriterType::Path(pattern, replacement)))
        }
        _ => Err(Error::from_reason("Wrong number of parameters")),
      },
      "header" => match args.len() {
        3 => {
          let name = args[0].to_owned();
          let pattern = args[1].to_owned();
          let replacement = args[2].to_owned();
          Ok(Box::new(PhpRewriterType::Header(
            name,
            pattern,
            replacement,
          )))
        }
        _ => Err(Error::from_reason("Wrong number of parameters")),
      },
      _ => Err(Error::from_reason(format!(
        "Unknown rewriter type: {}",
        rewriter_type
      ))),
    }
  }
}

//
// Conditional Rewriter
//

pub enum OperationType {
  And,
  Or,
}
impl FromStr for OperationType {
  type Err = Error;

  fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
    match s {
      "and" | "&&" => Ok(OperationType::And),
      "or" | "||" => Ok(OperationType::Or),
      op => Err(Error::from_reason(format!(
        "Unrecognized operation type: {}",
        op
      ))),
    }
  }
}

#[napi(object)]
#[derive(Debug, Default)]
pub struct PhpConditionalRewriterOptions {
  pub operation: Option<String>,
  pub conditions: Vec<PhpRewriteCondOptions>,
  pub rewriters: Vec<PhpRewriterOptions>,
}

pub struct PhpConditionalRewriter(Box<dyn Rewriter>);

impl Rewriter for PhpConditionalRewriter {
  fn rewrite(&self, request: Request) -> Request {
    self.0.rewrite(request)
  }
}

impl TryFrom<&PhpConditionalRewriterOptions> for Box<PhpConditionalRewriter> {
  type Error = Error;

  fn try_from(value: &PhpConditionalRewriterOptions) -> std::result::Result<Self, Self::Error> {
    let PhpConditionalRewriterOptions {
      operation,
      rewriters,
      conditions,
    } = value;

    let operation = operation
      .clone()
      .unwrap_or("and".into())
      .parse::<OperationType>()?;

    let rewriter = rewriters
      .iter()
      .try_fold(None::<Box<dyn Rewriter>>, |state, next| {
        let converted: std::result::Result<Box<PhpRewriterType>, Error> = next.try_into();
        converted.map(|converted| {
          let res: Option<Box<dyn Rewriter>> = match state {
            None => Some(converted),
            Some(last) => Some(last.then(converted)),
          };
          res
        })
      })?;

    let condition = conditions
      .iter()
      .try_fold(None::<Box<dyn Condition>>, |state, next| {
        let converted: std::result::Result<Box<PhpRewriteCond>, Error> = next.try_into();
        converted.map(|converted| {
          let res: Option<Box<dyn Condition>> = match state {
            None => Some(converted),
            Some(last) => Some(match operation {
              OperationType::Or => last.or(converted),
              OperationType::And => last.and(converted),
            }),
          };
          res
        })
      })?;

    match rewriter {
      None => Err(Error::from_reason("No rewriters provided")),
      Some(rewriter) => Ok(Box::new(PhpConditionalRewriter(match condition {
        None => rewriter,
        Some(condition) => rewriter.when(condition),
      }))),
    }
  }
}

//
// Rewriter JS type
//

#[napi(js_name = "Rewriter")]
pub struct PhpRewriter(Vec<PhpConditionalRewriterOptions>);

#[napi]
impl PhpRewriter {
  #[napi(constructor)]
  pub fn constructor(options: Vec<PhpConditionalRewriterOptions>) -> Result<Self> {
    Ok(PhpRewriter(options))
  }

  #[napi]
  pub fn rewrite(&self, request: &PhpRequest) -> Result<PhpRequest> {
    let rewriter = self.into_rewriter()?;
    Ok(PhpRequest {
      request: rewriter.rewrite(request.request.to_owned()),
    })
  }

  pub fn into_rewriter(&self) -> Result<Box<dyn Rewriter>> {
    if self.0.is_empty() {
      return Err(Error::from_reason("No rewrite rules provided"));
    }

    let rewriter = self
      .0
      .iter()
      .try_fold(None::<Box<dyn Rewriter>>, |state, next| {
        let converted: std::result::Result<Box<PhpConditionalRewriter>, Error> = next.try_into();
        converted.map(|converted| {
          let res: Option<Box<dyn Rewriter>> = match state {
            None => Some(converted),
            Some(last) => Some(last.then(converted)),
          };
          res
        })
      })?;

    match rewriter {
      None => Err(Error::from_reason("No rewriters provided")),
      Some(rewriter) => Ok(rewriter),
    }
  }
}
