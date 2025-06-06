use std::{path::Path, str::FromStr};

// use napi::bindgen_prelude::*;
use napi::{Error, Result};

use php::{
  rewrite::{
    Condition, ConditionExt, ExistenceCondition, HeaderCondition, HeaderRewriter, HrefRewriter,
    MethodCondition, MethodRewriter, NonExistenceCondition, PathCondition, PathRewriter, Rewriter,
    RewriterExt,
  },
  Request, RequestBuilderException,
};

use crate::PhpRequest;

//
// Conditions
//

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct PhpRewriteCondOptions {
  #[napi(js_name = "type")]
  pub cond_type: String,
  pub args: Option<Vec<String>>,
}

pub enum PhpRewriteCond {
  Exists,
  Header(String, String),
  Method(String),
  NotExists,
  Path(String),
}

impl Condition for PhpRewriteCond {
  fn matches(&self, request: &php::Request, docroot: &Path) -> bool {
    match self {
      PhpRewriteCond::Exists => ExistenceCondition.matches(request, docroot),
      PhpRewriteCond::Header(name, pattern) => {
        HeaderCondition::new(name.as_str(), pattern.as_str())
          .map(|v| v.matches(request, docroot))
          .unwrap_or_default()
      }
      PhpRewriteCond::Method(pattern) => MethodCondition::new(pattern.as_str())
        .map(|v| v.matches(request, docroot))
        .unwrap_or_default(),
      PhpRewriteCond::NotExists => NonExistenceCondition.matches(request, docroot),
      PhpRewriteCond::Path(pattern) => PathCondition::new(pattern.as_str())
        .map(|v| v.matches(request, docroot))
        .unwrap_or_default(),
    }
  }
}

impl TryFrom<&PhpRewriteCondOptions> for Box<PhpRewriteCond> {
  type Error = Error;

  fn try_from(value: &PhpRewriteCondOptions) -> std::result::Result<Self, Self::Error> {
    let PhpRewriteCondOptions { cond_type, args } = value;
    let cond_type = cond_type.to_lowercase();
    let args = args.to_owned().unwrap_or(vec![]);
    match cond_type.as_str() {
      "exists" => {
        if args.is_empty() {
          Ok(Box::new(PhpRewriteCond::Exists))
        } else {
          Err(Error::from_reason("Wrong number of parameters"))
        }
      }
      "header" => match args.len() {
        2 => {
          let name = args[0].to_owned();
          let pattern = args[1].to_owned();
          Ok(Box::new(PhpRewriteCond::Header(name, pattern)))
        }
        _ => Err(Error::from_reason("Wrong number of parameters")),
      },
      "method" => match args.len() {
        1 => Ok(Box::new(PhpRewriteCond::Method(args[0].to_owned()))),
        _ => Err(Error::from_reason("Wrong number of parameters")),
      },
      "not_exists" | "not-exists" => {
        if args.is_empty() {
          Ok(Box::new(PhpRewriteCond::NotExists))
        } else {
          Err(Error::from_reason("Wrong number of parameters"))
        }
      }
      "path" => match args.len() {
        1 => Ok(Box::new(PhpRewriteCond::Path(args[0].to_owned()))),
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
#[derive(Clone, Debug, Default)]
pub struct PhpRewriterOptions {
  #[napi(js_name = "type")]
  pub rewriter_type: String,
  pub args: Vec<String>,
}

pub enum PhpRewriterType {
  Header(String, String, String),
  Href(String, String),
  Method(String, String),
  Path(String, String),
}

impl Rewriter for PhpRewriterType {
  fn rewrite(
    &self,
    request: Request,
    docroot: &Path,
  ) -> std::result::Result<Request, RequestBuilderException> {
    match self {
      PhpRewriterType::Path(pattern, replacement) => {
        PathRewriter::new(pattern.as_str(), replacement.as_str())
          .map(|v| v.rewrite(request.clone(), docroot))
          .unwrap_or(Ok(request))
      }
      PhpRewriterType::Href(pattern, replacement) => {
        HrefRewriter::new(pattern.as_str(), replacement.as_str())
          .map(|v| v.rewrite(request.clone(), docroot))
          .unwrap_or(Ok(request))
      }
      PhpRewriterType::Method(pattern, replacement) => {
        MethodRewriter::new(pattern.as_str(), replacement.as_str())
          .map(|v| v.rewrite(request.clone(), docroot))
          .unwrap_or(Ok(request))
      }
      PhpRewriterType::Header(name, pattern, replacement) => {
        HeaderRewriter::new(name.as_str(), pattern.as_str(), replacement.as_str())
          .map(|v| v.rewrite(request.clone(), docroot))
          .unwrap_or(Ok(request))
      }
    }
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
      "href" => match args.len() {
        2 => {
          let pattern = args[0].to_owned();
          let replacement = args[1].to_owned();
          Ok(Box::new(PhpRewriterType::Href(pattern, replacement)))
        }
        _ => Err(Error::from_reason("Wrong number of parameters")),
      },
      "method" => match args.len() {
        2 => {
          let pattern = args[0].to_owned();
          let replacement = args[1].to_owned();
          Ok(Box::new(PhpRewriterType::Method(pattern, replacement)))
        }
        _ => Err(Error::from_reason("Wrong number of parameters")),
      },
      "path" => match args.len() {
        2 => {
          let pattern = args[0].to_owned();
          let replacement = args[1].to_owned();
          Ok(Box::new(PhpRewriterType::Path(pattern, replacement)))
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
#[derive(Clone, Debug, Default)]
pub struct PhpConditionalRewriterOptions {
  pub operation: Option<String>,
  pub conditions: Option<Vec<PhpRewriteCondOptions>>,
  pub rewriters: Vec<PhpRewriterOptions>,
}

pub struct PhpConditionalRewriter(Box<dyn Rewriter>);

impl Rewriter for PhpConditionalRewriter {
  fn rewrite(
    &self,
    request: Request,
    docroot: &Path,
  ) -> std::result::Result<Request, RequestBuilderException> {
    self.0.rewrite(request, docroot)
  }
}

impl TryFrom<&PhpConditionalRewriterOptions> for Box<PhpConditionalRewriter> {
  type Error = Error;

  fn try_from(value: &PhpConditionalRewriterOptions) -> std::result::Result<Self, Self::Error> {
    let value = value.clone();

    let operation = value
      .operation
      .clone()
      .unwrap_or("and".into())
      .parse::<OperationType>()?;

    let rewriter = value
      .rewriters
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

    let condition = value.conditions.unwrap_or_default().iter().try_fold(
      None::<Box<dyn Condition>>,
      |state, next| {
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
      },
    )?;

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
  pub fn rewrite(&self, request: &PhpRequest, docroot: String) -> Result<PhpRequest> {
    let rewriter = self.into_rewriter()?;
    let docroot = Path::new(&docroot);
    Ok(PhpRequest {
      request: rewriter
        .rewrite(request.request.to_owned(), docroot)
        .map_err(|err| {
          Error::from_reason(format!("Failed to rewrite request: {}", err.to_string()))
        })?,
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
