use std::{
  collections::HashMap,
  env::temp_dir,
  fs::{create_dir_all, remove_dir_all, File},
  io::{Error, Write},
  ops::{Deref, DerefMut},
  path::{Path, PathBuf},
};

pub struct MockRoot(PathBuf);

impl MockRoot {
  pub(crate) fn new<D, H>(docroot: D, files: H) -> Result<Self, Error>
  where
    D: AsRef<Path>,
    H: Into<HashMap<PathBuf, String>>,
  {
    let docroot = docroot.as_ref();
    create_dir_all(docroot)?;

    let map: HashMap<PathBuf, String> = files.into();
    for (path, contents) in map.iter() {
      let stripped = path.strip_prefix("/").unwrap_or(path);

      let file_path = docroot.join(stripped);
      if let Some(parent) = file_path.parent() {
        create_dir_all(parent)?;
      }

      let mut file = File::create(file_path)?;
      file.write_all(contents.as_bytes())?;
    }

    // This unwrap should be safe due to creating the docroot base dir above.
    Ok(Self(docroot.canonicalize().unwrap()))
  }

  pub fn builder() -> MockRootBuilder {
    MockRootBuilder::default()
  }
}

impl Drop for MockRoot {
  fn drop(&mut self) {
    remove_dir_all(&self.0).ok();
  }
}

impl Deref for MockRoot {
  type Target = PathBuf;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for MockRoot {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

#[derive(Debug)]
pub struct MockRootBuilder(PathBuf, HashMap<PathBuf, String>);

impl MockRootBuilder {
  pub(crate) fn new<D>(docroot: D) -> Self
  where
    D: AsRef<Path>,
  {
    Self(docroot.as_ref().to_owned(), HashMap::new())
  }

  pub fn file<P, C>(mut self, path: P, contents: C) -> MockRootBuilder
  where
    P: AsRef<Path>,
    C: Into<String>,
  {
    let path = path.as_ref().to_owned();
    let contents = contents.into();

    self.1.insert(path, contents);
    self
  }

  pub fn build(self) -> Result<MockRoot, Error> {
    MockRoot::new(self.0, self.1)
  }
}

impl Default for MockRootBuilder {
  fn default() -> Self {
    Self::new(temp_dir().join("php-temp-dir-base"))
  }
}
