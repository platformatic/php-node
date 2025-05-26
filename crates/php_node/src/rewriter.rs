// use napi::bindgen_prelude::*;
use napi::{Error, Result};

use php::rewrite::{
  Condition, ConditionOperation, ConditionalRewriter, HeaderCondition, HeaderRewriter,
  PathCondition, PathRewriter, Rewriter, RewriterSet,
};

use crate::PhpRequest;

#[napi(object)]
#[derive(Default, Debug)]
pub struct PhpRewriteCond {
  #[napi(js_name = "type")]
  pub cond_type: String,
  pub args: Vec<String>,
}

impl PhpRewriteCond {
  pub fn try_to_cond(&self) -> Result<Box<dyn Condition>> {
    let cond_type = self.cond_type.to_lowercase();
    match cond_type.as_str() {
      "path" => match self.args.len() {
        1 => Ok(Box::new(
          PathCondition::new(self.args[0].to_owned())
            .map_err(|err| Error::from_reason(err.to_string()))?,
        )),
        _ => Err(Error::from_reason("Wrong number of parameters")),
      },
      "header" => match self.args.len() {
        2 => {
          let name = self.args[0].to_owned();
          let pattern = self.args[1].to_owned();
          Ok(Box::new(
            HeaderCondition::new(name, pattern)
              .map_err(|err| Error::from_reason(err.to_string()))?,
          ))
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

#[napi(object)]
#[derive(Default, Debug)]
pub struct PhpRewriterInst {
  #[napi(js_name = "type")]
  pub rewriter_type: String,
  pub args: Vec<String>,
}

impl PhpRewriterInst {
  fn try_to_rewriter(&self) -> Result<Box<dyn Rewriter>> {
    let rewriter_type = self.rewriter_type.to_lowercase();
    match rewriter_type.as_str() {
      "path" => match self.args.len() {
        2 => {
          let pattern = self.args[0].to_owned();
          let replacement = self.args[1].to_owned();
          Ok(Box::new(
            PathRewriter::new(pattern, replacement)
              .map_err(|err| Error::from_reason(err.to_string()))?,
          ))
        }
        _ => Err(Error::from_reason("Wrong number of parameters")),
      },
      "header" => match self.args.len() {
        3 => {
          let name = self.args[0].to_owned();
          let pattern = self.args[1].to_owned();
          let replacement = self.args[2].to_owned();
          Ok(Box::new(
            HeaderRewriter::new(name, pattern, replacement)
              .map_err(|err| Error::from_reason(err.to_string()))?,
          ))
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

/// Options for creating a new PHP response.
#[napi(object)]
#[derive(Default)]
pub struct PhpRewriterOptions {
  pub operation: String,
  pub conditions: Vec<PhpRewriteCond>,
  pub rewriters: Vec<PhpRewriterInst>,
}

#[napi(js_name = "Rewriter")]
pub struct PhpRewriter {
  rewriter: Box<dyn Rewriter>,
}

#[napi]
impl PhpRewriter {
  #[napi(constructor)]
  pub fn constructor(options: Vec<PhpRewriterOptions>) -> Result<Self> {
    let mut rewriter = RewriterSet::default();

    for option in options.iter() {
      rewriter.add_rewriter({
        let operation = option
          .operation
          .parse::<ConditionOperation>()
          .map_err(|e| Error::from_reason(e))?;

        let mut rewriter = ConditionalRewriter::new(operation);

        for item in option.conditions.iter() {
          rewriter.add_condition(item.try_to_cond()?);
        }
        for item in option.rewriters.iter() {
          rewriter.add_rewriter(item.try_to_rewriter()?);
        }

        Box::new(rewriter)
      });
    }

    Ok(PhpRewriter {
      rewriter: Box::new(rewriter),
    })
  }

  #[napi]
  pub fn rewrite(&self, request: &PhpRequest) -> PhpRequest {
    PhpRequest {
      request: self.rewriter.rewrite(request.request.to_owned()),
    }
  }
}
