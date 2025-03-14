use std::{
    collections::HashMap,
    env,
    ffi::OsStr,
    fmt::Debug,
    path::PathBuf,
    process::Command
};

// use autotools::Config;
use bindgen::Builder;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let project_root = crate_dir.join("../..");

    let spc = project_root.join("spc");
    let spc_cmd = spc.to_str().unwrap();

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let extensions_sh = project_root.join("scripts/extensions.sh");
    let extensions_cmd = extensions_sh.to_str().unwrap();
    let extensions = execute_command(&[extensions_cmd], None);

    // Get the includes
    let includes = execute_command(&[
        spc_cmd,
        "spc-config",
        &extensions,
        "--includes"
    ], None);

    // Get the libs
    let libs = execute_command(&[
        spc_cmd,
        "spc-config",
        &extensions,
        "--libs"
    ], None);

    // Include main headers
    let mut includes = includes.split(' ').collect::<Vec<_>>();
    for dir in includes.iter() {
        println!("cargo:include={}", &dir[2..]);
    }

    // Include SAPI headers
    let sapi_include = project_root
        .join("buildroot/include/php/sapi")
        .canonicalize()
        .unwrap();

    println!("cargo:include={}", sapi_include.display());

    // Link libraries
    let libs = libs.split(' ').collect::<Vec<_>>();
    let mut is_framework = false;
    for dir in libs.iter() {
        if is_framework {
            is_framework = false;
            println!("cargo:rustc-link-lib=framework={}", dir);
        }
        if dir.starts_with("-L") {
            println!("cargo:rustc-link-search={}", &dir[2..]);
        }
        if dir.starts_with("-l") {
            println!("cargo:rustc-link-lib={}", &dir[2..]);
        }
        if dir.starts_with("-framework") {
            is_framework = true;
        }
    }

    // Locate lang_handler header and lib
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap())
        .join("../../..")
        .canonicalize()
        .unwrap();

    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rustc-link-lib=lang_handler");
    println!("cargo:include={}", out_dir.display());

    let lang_handler_include_flag = format!("-I{}", out_dir.display());
    includes.push(lang_handler_include_flag.as_str());

    let mut builder = cc::Build::new();
    for include in &includes {
        builder.flag(include);
    }
    builder
        .file("src/php_wrapper.c")
        .compile("phpwrapper");

    println!("cargo:rerun-if-changed=src/php_wrapper.h");

    let sw_vers = execute_command(&[
        "sw_vers",
        "-productVersion"
    ], None);

    println!("cargo:env=MACOSX_DEPLOYMENT_TARGET={}", sw_vers);

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
        .opaque_type("lh_request_t")
        .opaque_type("lh_response_t")
        .opaque_type("lh_request_builder_t")
        .opaque_type("lh_response_builder_t")
        .opaque_type("lh_headers_t")
        .opaque_type("lh_url_t")
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

fn execute_command<S: AsRef<OsStr> + Debug>(argv: &[S], env: Option<HashMap<String, String>>) -> String {
    let mut command = Command::new(&argv[0]);
    if let Some(env) = env {
        command.envs(env);
    }
    command.args(&argv[1..]);

    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let project_root = crate_dir.join("../..");
    command.current_dir(project_root);

    let result = command
        .output()
        .unwrap_or_else(|e| panic!("Execute command {:?} failed with: {:?}", &argv, e));

    if !result.status.success() {
        let out = String::from_utf8(result.stdout).unwrap();
        let err = String::from_utf8(result.stderr).unwrap();
        panic!("Execute command {:?} failed with stdout: {}, stderr: {}", &argv, out, err);
    }

    String::from_utf8(result.stdout).unwrap().trim().to_owned()
}
