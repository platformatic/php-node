use std::{
  collections::HashMap,
  env::temp_dir,
  fs::{create_dir_all, File},
  io::{Error, ErrorKind, Write},
  ops::{Deref, DerefMut},
  path::{Path, PathBuf},
};

/// A mock document root for testing purposes.
pub struct MockRoot(PathBuf);

impl MockRoot {
  /// Create a new MockRoot with the given document root and files.
  ///
  /// # Examples
  ///
  /// ```
  /// # use std::{collections::HashMap, env::temp_dir, path::PathBuf};
  /// # use lang_handler::MockRoot;
  /// # let docroot = std::env::temp_dir().join("test");
  /// let files = HashMap::from([
  ///   (PathBuf::new().join("file1.txt"), "Hello, world!".to_string()),
  ///   (PathBuf::new().join("file2.txt"), "Goodbye, world!".to_string())
  /// ]);
  ///
  /// let mock_root = MockRoot::new(&docroot, files)
  ///   .expect("should create mock root");
  /// ```
  pub fn new<D, H>(docroot: D, files: H) -> Result<Self, Error>
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
    Ok(Self(
      docroot
        .canonicalize()
        .map_err(|err| Error::new(ErrorKind::Other, err))?,
    ))
  }

  /// Create a new MockRoot with the given document root and files.
  ///
  /// # Examples
  ///
  /// ```
  /// use lang_handler::MockRoot;
  ///
  /// let mock_root = MockRoot::builder()
  ///   .file("file1.txt", "Hello, world!")
  ///   .file("file2.txt", "Goodbye, world!")
  ///   .build()
  ///   .unwrap();
  /// ```
  pub fn builder() -> MockRootBuilder {
    MockRootBuilder::default()
  }
}

// TODO: Somehow this happens too early?
// impl Drop for MockRoot {
//   fn drop(&mut self) {
//     remove_dir_all(&self.0).ok();
//   }
// }

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

/// A builder for creating a MockRoot with specified files.
#[derive(Debug)]
pub struct MockRootBuilder(PathBuf, HashMap<PathBuf, String>);

impl MockRootBuilder {
  /// Create a new MockRootBuilder with the specified document root.
  ///
  /// # Examples
  ///
  /// ```rust
  /// # use lang_handler::MockRootBuilder;
  /// # let docroot = std::env::temp_dir().join("test");
  /// let builder = MockRootBuilder::new(&docroot);
  /// ```
  pub fn new<D>(docroot: D) -> Self
  where
    D: AsRef<Path>,
  {
    Self(docroot.as_ref().to_owned(), HashMap::new())
  }

  /// Add a file to the MockRootBuilder.
  ///
  /// # Examples
  ///
  /// ```rust
  /// # use lang_handler::MockRootBuilder;
  /// # let docroot = std::env::temp_dir().join("test");
  /// let builder = MockRootBuilder::new(&docroot)
  ///   .file("bar.txt", "Hello, world!");
  /// ```
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

  /// Build the MockRoot.
  ///
  /// # Examples
  ///
  /// ```rust
  /// # use lang_handler::MockRootBuilder;
  /// # let docroot = std::env::temp_dir().join("test");
  /// let root = MockRootBuilder::new(&docroot)
  ///   .file("bar.txt", "Hello, world!")
  ///   .build()
  ///   .expect("should create mock root");
  /// ```
  pub fn build(self) -> Result<MockRoot, Error> {
    MockRoot::new(self.0, self.1)
  }
}

impl Default for MockRootBuilder {
  fn default() -> Self {
    Self::new(temp_dir().join("php-temp-dir-base"))
  }
}
