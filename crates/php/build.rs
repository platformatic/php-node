use std::{
    collections::HashSet,
    env,
    ffi::OsStr,
    fmt::{Debug, Display},
    path::PathBuf,
    process::Command
};

// use autotools::Config;
use bindgen::Builder;
use downloader::{Download, Downloader};
use file_mode::ModePath;

fn maybe_windowsify<T>(path: T) -> String where T: Display {
    match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("windows") => format!("{}.exe", path),
        _ => path.to_string()
    }
}

fn spc_url() -> String {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let url = format!("https://dl.static-php.dev/static-php-cli/spc-bin/nightly/spc-{}-{}", os, arch);

    maybe_windowsify(url)
}

fn get_spc() -> PathBuf {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let filename = maybe_windowsify("spc");
    let spc = out_path.join(filename.clone());

    println!("cargo:rerun-if-changed={}", spc.to_str().unwrap());

    if spc.exists() {
        return spc;
    }
    println!("spc is not present");

    let mut downloader = Downloader::builder()
        .download_folder(&out_path)
        .build()
        .unwrap();

    let dl = Download::new(&spc_url())
        .file_name(filename.as_ref());

    downloader.download(&vec![dl]).unwrap();

    // Make the file executable
    spc.set_mode("a+x").unwrap();
    spc
}

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let current_dir = env::current_dir().unwrap();

    // Any commented out ones I was not able to get to build for some reason...
    let available_extensions = HashSet::from([
        "amqp",
        "apcu",
        "ast",
        "bcmath",
        "bz2",
        "calendar",
        "ctype",
        // "curl",
        // "dba",
        // "dio",
        // "dom",
        // "ds",
        // "event",
        // "exif",
        // "ffi",
        // "fileinfo",
        // "filter",
        // "ftp",
        // "gd",
        // "gettext",
        // "glfw",
        // "gmp",
        // "gmssl",
        // "grpc",
        // "iconv",
        // "igbinary",
        // "imagick",
        // "imap",
        // "inotify",
        // "intl",
        // "ldap",
        // "libxml",
        // "mbregex",
        // "mbstring",
        // "mcrypt",
        // "memcache",
        // "memcached",
        // "mongodb",
        // "msgpack",
        // "mysqli",
        // "mysqlnd",
        // "oci8",
        // "opcache",
        // "openssl",
        // "parallel", // Requires zts
        // "password-argon2",
        // "pcntl",
        // "pdo",
        // "pdo_mysql",
        // "pdo_pgsql",
        // "pdo_sqlite",
        // "pdo_sqlsrv",
        // "pgsql",
        // "phar",
        // "posix",
        // "protobuf",
        // "rar",
        // "rdkafka",
        // "readline",
        // "redis",
        // "session",
        // "shmop",
        // "simdjson",
        // "simplexml",
        // "snappy",
        // "soap",
        // "sockets",
        // "sodium",
        // "spx",
        // "sqlite3",
        // "sqlsrv",
        // "ssh2",
        // "swoole", // Mutually exclusive with pdo_*
        // "swoole-hook-mysql", // Mutually exclusive with pdo_*
        // "swoole-hook-pgsql", // Mutually exclusive with pdo_*
        // "swoole-hook-sqlite", // Mutually exclusive with pdo_*
        // "swow",
        // "sysvmsg",
        // "sysvsem",
        // "sysvshm",
        // "tidy",
        // "tokenizer",
        // "uuid",
        // "uv",
        // "xdebug",
        // "xhprof",
        // "xlswriter",
        // "xml",
        // "xmlreader",
        // "xmlwriter",
        // "xsl",
        // "yac",
        // "yaml",
        // "zip",
        // "zlib",
        // "zstd"
    ]);

    // TODO: Make an actually reasonable selection of default extensions
    let extensions = env::var("PHP_EXTENSIONS").unwrap_or_else(
        |_| available_extensions.iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
            .join(",")
    );

    let spc = get_spc();
    let spc_cmd = spc.to_str().unwrap();

    // Skip download and build of PHP if a build is already present
    // TODO: Probably need to detect modification date somehow...
    if !current_dir.join("buildroot/lib/libphp.a").exists() {
        // Download PHP and requested extensions
        execute_command(&[
            spc_cmd,
            "download",
            &format!("--for-extensions={}", extensions.clone()),
            "--retry=10",
            "--prefer-pre-built",
            "--with-php=8.4"
        ]);

        // Build in embed mode
        execute_command(&[
            spc_cmd,
            "build",
            &extensions,
            "--build-embed",
            // "--enable-zts"
        ]);
    }

    // Get the includes
    let includes = execute_command(&[
        spc_cmd,
        "spc-config",
        &extensions,
        "--includes"
    ]);

    // Get the libs
    let libs = execute_command(&[
        spc_cmd,
        "spc-config",
        &extensions,
        "--libs"
    ]);

    // Include main headers
    let includes = includes.split(' ').collect::<Vec<_>>();
    for dir in includes.iter() {
        println!("cargo:include={}", &dir[2..]);
    }

    // Include SAPI headers
    let sapi_include = current_dir
        .join("buildroot/include/php/sapi")
        .canonicalize()
        .unwrap();

    println!("cargo:include={}", sapi_include.display());

    // Link libraries
    let libs = libs.split(' ').collect::<Vec<_>>();
    for dir in libs.iter() {
        if dir.starts_with("-L") {
            println!("cargo:rustc-link-search={}", &dir[2..]);
        }
        if dir.starts_with("-l") {
            println!("cargo:rustc-link-lib={}", &dir[2..]);
        }
        if dir.starts_with("-framework") {
            println!("cargo:rustc-link-lib=framework={}", &dir[11..]);
        }
    }

    let mut builder = cc::Build::new();
    for include in &includes {
        builder.flag(include);
    }
    builder
        .file("src/php_wrapper.c")
        .compile("phpwrapper");

    println!("cargo:rerun-if-changed=src/php_wrapper.h");

    let mut builder = Builder::default()
        // .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .header("src/php_wrapper.c")
        .allowlist_file("src/php_wrapper\\.c")
        // Block the `zend_ini_parse_quantity` because it's document causes the doc test to fail.
        .blocklist_function("zend_ini_parse_quantity")
        // Block the `zend_startup` because it fails checks.
        .blocklist_function("zend_startup")
        // Block the `zend_random_bytes_insecure` because it fails checks.
        .blocklist_item("zend_random_bytes_insecure")
        .clang_args(&includes)
        .derive_default(true);

    // iterate over the php include directories, and update the builder
    // to only create bindings from the header files in those directories
    for dir in includes.iter() {
        let p = PathBuf::from(&dir[2..]).join(".*\\.h");
        builder = builder.allowlist_file(p.display().to_string());
    }

    let generated_path = out_path.join("bindings.rs");
    builder
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(generated_path)
        .expect("Unable to write output file");
}

fn execute_command<S: AsRef<OsStr> + Debug>(argv: &[S]) -> String {
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]);

    let result = command
        .output()
        .unwrap_or_else(|e| panic!("Execute command {:?} failed with: {:?}", &argv, e));

    if !result.status.success() {
        let err = String::from_utf8(result.stderr).unwrap();
        panic!("Execute command {:?} failed with output: {}", &argv, err);
    }

    String::from_utf8(result.stdout).unwrap().trim().to_owned()
}
