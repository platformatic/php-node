# PHP-Node Source Code Documentation

This document provides a comprehensive guide to understanding the php-node Rust codebase, which embeds the PHP runtime within Node.js applications.

## Table of Contents

1. [Project Overview](#project-overview)
2. [Architecture Overview](#architecture-overview)
3. [Core Modules](#core-modules)
4. [Request Lifecycle](#request-lifecycle)
5. [PHP Embedding Details](#php-embedding-details)
6. [SAPI Implementation](#sapi-implementation)
7. [Thread Safety and Concurrency](#thread-safety-and-concurrency)
8. [Memory Management](#memory-management)
9. [Important Gotchas and Edge Cases](#important-gotchas-and-edge-cases)
10. [Development Guidelines](#development-guidelines)

---

## Project Overview

### What is php-node?

php-node is a Rust library that embeds the PHP runtime into Node.js applications, enabling PHP scripts to handle HTTP requests within the same process as Node.js. This eliminates network overhead and enables seamless interoperability between PHP and Node.js.

### Key Features

- **Zero network overhead**: PHP runs in the same process as Node.js
- **Request rewriting**: Apache mod_rewrite-like functionality built-in
- **Thread-safe**: Supports concurrent PHP request handling
- **NAPI bindings**: Exposes Rust functionality to Node.js via N-API
- **Custom SAPI**: Implements PHP's Server API for optimal performance
- **Reusable PHP environments**: Shares compiled code between requests

### Architecture Goals

1. **Performance**: Minimize PHP startup cost by reusing the PHP environment
2. **Safety**: Use Rust's type system to prevent common C/FFI errors
3. **Flexibility**: Support request rewriting and customization
4. **Compatibility**: Work with existing PHP applications (Laravel, WordPress, etc.)

---

## Architecture Overview

### High-Level Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                         Node.js                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              NAPI Layer (napi.rs)                      │  │
│  │  - PhpRuntime: JavaScript-facing PHP instance          │  │
│  │  - PhpRequestTask: Async request handler               │  │
│  └────────────────────────────────────────────────────────┘  │
│                            ↓                                 │
│  ┌────────────────────────────────────────────────────────┐  │
│  │            Core Embed Layer (embed.rs)                 │  │
│  │  - Embed: Main request handler                         │  │
│  │  - Request rewriting logic                             │  │
│  │  - HTTP Handler trait implementation                   │  │
│  └────────────────────────────────────────────────────────┘  │
│                            ↓                                 │
│  ┌────────────────────────────────────────────────────────┐  │
│  │          SAPI Implementation (sapi.rs)                 │  │
│  │  - Custom PHP SAPI module                              │  │
│  │  - SAPI lifecycle management                           │  │
│  │  - INI configuration                                   │  │
│  │  - Callbacks for I/O operations                        │  │
│  └────────────────────────────────────────────────────────┘  │
│                            ↓                                 │
│  ┌────────────────────────────────────────────────────────┐  │
│  │          Request Context (request_context.rs)          │  │
│  │  - Thread-local request state                          │  │
│  │  - Response builder accumulation                       │  │
│  │  - Access to request/response data                     │  │
│  └────────────────────────────────────────────────────────┘  │
│                            ↓                                 │
│  ┌────────────────────────────────────────────────────────┐  │
│  │         PHP Runtime (via ext-php-rs)                   │  │
│  │  - Zend Engine                                         │  │
│  │  - Script execution                                    │  │
│  │  - Exception handling                                  │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### Key Design Decisions

1. **Single-crate structure**: Consolidated from multi-crate workspace for simplicity
2. **Feature-gated NAPI**: The `napi-support` feature enables Node.js bindings
3. **Thread-safe by design**: Uses `ext-php-rs` with ZTS (Zend Thread Safety)
4. **Request context pattern**: Thread-local storage for request-specific data
5. **RAII scopes**: Automatic cleanup using Rust's Drop trait

---

## Core Modules

### lib.rs

**Purpose**: Library entry point and public API surface

**Key exports**:
- `Embed`: Main PHP runtime wrapper
- `Handler` trait: Async HTTP request handling
- `Request` / `Response`: HTTP types
- `EmbedStartError` / `EmbedRequestError`: Error types
- `napi` module (when `napi-support` feature enabled)

**Design notes**:
- Re-exports types from `http_handler` and `http_rewriter` crates
- Uses feature flags to conditionally compile NAPI support
- Documentation includes executable examples using doctests

### embed.rs

**Purpose**: Core PHP embedding logic and request handling

**Key types**:
- `Embed`: The main struct representing a PHP runtime instance
  - `docroot`: Document root directory
  - `args`: Command-line arguments passed to PHP
  - `sapi`: Arc-wrapped SAPI instance (keeps SAPI alive)
  - `rewriter`: Optional request rewriter

**Key trait implementations**:
- `Handler` trait: Async HTTP request handling
- `Send + Sync`: Enables sharing Embed across threads

**Request handling flow**:
1. **Startup**: Initialize SAPI module
2. **Preserve REQUEST_URI**: Capture pre-rewrite URI
3. **Rewriting**: Apply request rewriter rules
4. **Path translation**: Map URL path to filesystem path
5. **Setup context**: Create RequestContext with request data
6. **SAPI configuration**: Set request_info fields
7. **Execute**: Run PHP script in try_catch_first block
8. **Exception handling**: Capture and convert PHP exceptions
9. **Response building**: Build HTTP response from accumulated data

**Important details**:
- Uses `try_catch_first` to handle PHP bailouts (fatal errors, exit calls)
- Converts C strings using `estrdup` (must be manually freed)
- RequestContext is set up BEFORE try_catch_first to avoid RefUnwindSafe issues
- Nested scopes ensure proper cleanup even on bailout

### sapi.rs

**Purpose**: Custom PHP SAPI (Server API) implementation

**Key types**:
- `Sapi`: Wrapper around `SapiModule` with lifecycle management
- `SAPI_INIT`: Global singleton using OnceCell and Weak reference pattern

**SAPI Callbacks** (C FFI functions):

1. **sapi_cli_ini_defaults**: Sets hardcoded PHP INI values
2. **sapi_module_startup**: Calls `php_module_startup`
3. **sapi_module_shutdown**: Calls `php_module_shutdown`
4. **sapi_module_deactivate**: Frees request-specific C strings
5. **sapi_module_ub_write**: Writes output to response builder
6. **sapi_module_flush**: Sends headers (calls `sapi_send_headers`)
7. **sapi_module_send_header**: Adds header to response builder
8. **sapi_module_read_post**: Reads from request body
9. **sapi_module_read_cookies**: Returns Cookie header value
10. **sapi_module_register_server_variables**: Populates $_SERVER superglobal
11. **sapi_module_log_message**: Captures PHP error logs

**Singleton pattern**:
- Uses `OnceCell<RwLock<Weak<Sapi>>>` for thread-safe lazy initialization
- `ensure_sapi()` upgrades Weak reference or creates new Sapi
- Prevents multiple SAPI initializations which would crash PHP

**INI configuration**:
- Hardcoded INI settings for CLI-like behavior
- `max_execution_time=0`: No timeout for request execution
- `output_buffering=0`: Direct output (no buffering)
- `display_errors=0`: Don't send errors to output
- `log_errors=1`: Capture errors via log_message callback

**Important**:
- SAPI is initialized once per process and reused for all requests
- Each request gets its own request scope via `RequestScope`
- The `Sapi` Drop implementation ensures clean shutdown

### request_context.rs

**Purpose**: Thread-local request state management

**Key type**:
- `RequestContext`: Stores request-specific data
  - `request`: The HTTP request being processed
  - `response_builder`: Accumulates response data
  - `docroot`: Document root for this request

**Pattern**: Thread-local storage via `SapiGlobals::server_context`

**Lifecycle**:
1. **Creation**: `for_request()` boxes context and stores raw pointer
2. **Access**: `current()` retrieves mutable reference from pointer
3. **Reclaim**: `reclaim()` takes ownership back to Box for cleanup
4. **Build**: `build_response()` consumes context to produce Response

**Why this pattern?**:
- SAPI callbacks are C functions with no context parameter
- PHP stores a `void*` server_context we can use
- Allows SAPI callbacks to access request/response data
- Ensures memory safety through RAII

**Important**:
- Context MUST be reclaimed before request ends to prevent leaks
- Accessing context from wrong thread will return None
- Context is tied to PHP's request lifecycle

### scopes.rs

**Purpose**: RAII scopes for PHP lifecycle management

**Key types**:

1. **RequestScope**: Manages PHP request lifecycle
   - Constructor calls `php_request_startup()`
   - Drop calls `php_request_shutdown()`
   - Ensures proper cleanup even on bailout

2. **FileHandleScope**: Manages zend_file_handle lifecycle
   - Constructor initializes file handle with `zend_stream_init_filename()`
   - Sets `primary_script = true`
   - Drop calls `zend_destroy_file_handle()` and frees path string

**Design pattern**: RAII (Resource Acquisition Is Initialization)
- Leverage Rust's Drop trait for automatic cleanup
- Prevents resource leaks even when unwinding from bailouts
- Separates concerns (request scope vs file handle scope)

**Why nested scopes?**:
- Request scope wraps file handle scope
- Allows script execution to fail without skipping request shutdown
- Matches PHP's internal lifecycle requirements

### napi.rs

**Purpose**: Node.js N-API bindings (feature-gated)

**Key types**:

1. **PhpRuntime**: JavaScript-facing PHP class
   - Wraps `Arc<Embed>` for thread-safe sharing
   - `throw_request_errors`: Controls error handling behavior
   - Methods: `new()`, `handle_request()`, `handle_request_sync()`

2. **PhpOptions**: Configuration object
   - `argv`: Command-line arguments
   - `docroot`: Document root path
   - `throw_request_errors`: Error handling mode
   - `rewriter`: Optional request rewriter

3. **PhpRequestTask**: Async task for worker thread
   - Implements NAPI `Task` trait
   - `compute()`: Runs in worker thread
   - `resolve()`: Converts result in main thread

**Design decisions**:
- Async by default (uses NAPI worker pool)
- Sync method provided but discouraged
- Error translation: Rust errors → HTTP responses or thrown exceptions
- Thread-safe sharing via Arc<Embed>

### exception.rs

**Purpose**: Error type definitions

**Key error types**:

1. **EmbedStartError**: Errors during Embed construction
   - `DocRootNotFound`: Invalid or missing document root
   - `ExeLocationNotFound`: Can't determine executable path
   - `SapiNotInitialized`: SAPI initialization failed

2. **EmbedRequestError**: Errors during request handling
   - `SapiNotStarted`: SAPI startup failed
   - `Exception`: PHP exception thrown
   - `Bailout`: PHP bailout (fatal error or exit)
   - `ScriptNotFound`: PHP file doesn't exist
   - `ResponseBuildError`: Failed to build response
   - `RequestRewriteError`: Request rewriting failed

**Display implementations**: Provide human-readable error messages

### strings.rs

**Purpose**: Path translation utilities

**Key function**: `translate_path(docroot, request_uri)`
- Converts URL path to filesystem path
- Handles trailing slash → index.php conversion
- Validates path exists and is a file
- Returns canonicalized absolute path

**Edge cases**:
- `/foo/` → `{docroot}/foo/index.php` (if exists) or `{docroot}/foo`
- `/foo` → `{docroot}/foo` (exact match only)
- Requires absolute URI (must start with `/`)

### test.rs

**Purpose**: Testing utilities

**Key types**:
- `MockRoot`: Temporary directory with PHP files
- `MockRootBuilder`: Builder pattern for test setup

**Usage**: Create temporary document roots for integration tests

---

## Request Lifecycle

### Complete Request Flow

```
1. JavaScript: php.handleRequest(request)
   ↓
2. NAPI: PhpRequestTask created and scheduled on worker pool
   ↓
3. Worker thread: PhpRequestTask.compute()
   ↓
4. Embed::handle(request)
   ├─ 4.1: sapi.startup()
   ├─ 4.2: Capture REQUEST_URI (pre-rewrite)
   ├─ 4.3: Apply rewriter (if configured)
   ├─ 4.4: translate_path() to get script path
   ├─ 4.5: Convert strings to C (estrdup)
   ├─ 4.6: RequestContext::for_request()
   ├─ 4.7: try_catch_first {
   │    ├─ Set SapiGlobals::request_info
   │    ├─ RequestScope::new()
   │    ├─ try_catch {
   │    │    ├─ FileHandleScope::new()
   │    │    └─ php_execute_script()
   │    │  }
   │    ├─ Check for exception
   │    ├─ Get mimetype and status code
   │    └─ RequestContext::reclaim()
   │  }
   └─ 4.8: Return Response or Error
   ↓
5. Worker thread → Main thread: Convert to PhpResponse
   ↓
6. JavaScript: Promise resolves with response
```

### SAPI Callback Invocations During Request

**During php_request_startup()**:
- `sapi_module_read_cookies()` → populates $_COOKIE
- `sapi_module_register_server_variables()` → populates $_SERVER

**During php_execute_script()**:
- `sapi_module_ub_write()` → called for each echo/print
- `sapi_module_send_header()` → called for each header()
- `sapi_module_flush()` → called for flush()
- `sapi_module_read_post()` → called when reading php://input
- `sapi_module_log_message()` → called for error_log() or PHP errors

**During php_request_shutdown()**:
- `sapi_module_deactivate()` → frees C strings

### Thread Safety Model

**PHP side (via ext-php-rs)**:
- Compiled with ZTS (Zend Thread Safety)
- Uses TSRM (Thread Safe Resource Management)
- Each thread has its own executor globals

**Rust side**:
- `Embed` is `Send + Sync` (can be shared across threads)
- `Arc<Sapi>` Shares SAPI across threads
- `RequestContext` uses thread-local storage pattern
- Each request executes in its own worker thread

**Important**: Multiple requests can execute concurrently on different threads

---

## PHP Embedding Details

### ext-php-rs Library

**What it provides**:
- Safe Rust bindings to PHP/Zend C APIs
- `try_catch` / `try_catch_first`: Exception handling
- `SapiBuilder`: Construct SAPI modules
- Memory management: `estrdup`, `efree`
- Globals access: `SapiGlobals`, `ExecutorGlobals`

**Key concepts**:
- `try_catch_first`: First unwinding pass (for bailouts)
- `try_catch`: Second unwinding pass (for normal exceptions)
- Bailout: PHP's longjmp mechanism for fatal errors

### PHP SAPI Architecture

**What is SAPI?**
- Server API: Interface between PHP and web server
- Defined by struct with function pointers
- Handles I/O between PHP and host application

**Our SAPI design**:
- Name: "php_lang_handler"
- Based on PHP's CLI SAPI
- Modified for request/response handling
- Reusable across multiple requests

**SAPI vs Zend API**:
- SAPI is high-level, opinionated
- We use SAPI structure but manage lifecycle directly
- Allows request scope reuse without full teardown

### Memory Management

**C string handling**:
- `estrdup(str)`: Allocate C string (PHP's allocator)
- `efree(ptr)`: Free C string
- All estrdup'd strings must be freed
- `maybe_efree()`: Safe wrapper checking for null

**Memory ownership**:
- Request data: Created per request, freed in deactivate
- Response data: Accumulated in RequestContext
- SAPI module: Singleton, freed on process exit
- File handles: RAII via FileHandleScope

**Important**:
- Never mix libc malloc/free with PHP's emalloc/efree
- Always free estrdup'd strings in deactivate callback
- Use RAII patterns to prevent leaks on bailout
- DO NOT use strings from anywhere but estrdup in PHP types or functions

### PHP Superglobals

**How they work**:
- Global variables available in all PHP scopes
- Populated via SAPI callbacks
- Stored in executor globals
- All frameworks and higher-level server abstractions hang off these

**Key superglobals**:
- `$_SERVER`: Set via register_server_variables callback
- `$_GET`: Parsed from query string by PHP
- `$_POST`: Parsed from body by PHP (via read_post)
- `$_COOKIE`: Parsed from Cookie header (via read_cookies)
- `$_FILES`: Parsed from multipart body by PHP

**Our implementation**:
- `sapi_module_register_server_variables()`: Sets all $_SERVER vars
- Includes CGI-style variables (HTTP_*, REQUEST_METHOD, etc.)
- Sets custom variables (DOCUMENT_ROOT, SCRIPT_FILENAME, etc.)

---

## SAPI Implementation

### SAPI Lifecycle

```
Process startup:
  Sapi::new()
    ├─ Create SapiModule with SapiBuilder
    ├─ ext_php_rs_sapi_startup()
    ├─ sapi_startup(&module)
    └─ module.startup(&module)  [sapi_module_startup]
         └─ php_module_startup()

Per request:
  Sapi::startup()
    └─ ext_php_rs_sapi_per_thread_init()

Process shutdown:
  Drop for Sapi
    ├─ Sapi::shutdown()
    │   └─ module.shutdown(&module)  [sapi_module_shutdown]
    │        └─ php_module_shutdown()
    ├─ sapi_shutdown()
    └─ ext_php_rs_sapi_shutdown()
```

### Critical SAPI Callbacks

**ub_write (unbuffered write)**:
```rust
fn sapi_module_ub_write(str: *const c_char, str_length: usize) -> usize
```
- Called when PHP outputs data (echo, print, etc.)
- Appends bytes to response builder
- Returns number of bytes written
- Must handle null pointer

**read_post**:
```rust
fn sapi_module_read_post(buffer: *mut c_char, length: usize) -> usize
```
- Called when PHP reads from php://input
- Reads from request body buffer
- Uses BytesMut::split_to() to consume data
- Returns actual bytes read (may be less than requested)

**register_server_variables**:
```rust
fn sapi_module_register_server_variables(vars: *mut Zval)
```
- Populates $_SERVER superglobal
- Sets CGI-style variables (HTTP_*, REQUEST_METHOD, etc.)
- Sets PHP-specific variables (SCRIPT_FILENAME, etc.)
- Called during php_request_startup()

**send_header**:
```rust
fn sapi_module_send_header(header: *mut SapiHeader, _context: *mut c_void)
```
- Called when PHP calls header() function
- Adds header to response builder
- Null header means status line (HTTP/1.1 200 OK)

### INI Configuration

**Hardcoded settings**:
```ini
error_reporting=4343  # E_ERROR | E_WARNING | E_PARSE | ...
ignore_repeated_errors=1
display_errors=0       # Don't output errors to response
log_errors=1           # Capture via log_message callback
memory_limit=128M
output_buffering=0     # Direct output, no buffering
max_execution_time=0   # No timeout
max_input_time=-1      # No timeout
```

**Why these settings?**:
- `display_errors=0`: Errors via log_message instead
- `output_buffering=0`: Immediate output streaming
- `max_execution_time=0`: Let Node.js handle timeouts
- `register_argc_argv=1`: Enable $argc and $argv

---

## Thread Safety and Concurrency

### Thread Safety Guarantees

**Rust side**:
- `Embed` implements `Send + Sync` (explicitly via unsafe impl)
- `Arc<Sapi>` provides shared ownership
- `RequestContext` uses thread-local pattern
- No shared mutable state between requests

**PHP side**:
- Compiled with `--enable-zts` (Zend Thread Safety)
- TSRM provides thread-local storage
- Executor globals are per-thread
- Each thread has independent PHP state

**Important**: Concurrent requests are safe and encouraged

### Concurrency Model

**Request handling**:
- Each request runs in NAPI worker thread
- Multiple requests can execute concurrently, but one per-thread
- Each request has its own RequestContext
- SAPI is shared but thread-safe

**Limitations**:
- PHP extensions may not be thread-safe
- File operations use global file descriptors
- Some PHP functions may have global state

### Weak Reference Pattern

**Why Weak<Sapi>?**
```rust
pub(crate) static SAPI_INIT: OnceCell<RwLock<Weak<Sapi>>> = OnceCell::new();
```
- Allows SAPI to be dropped when last Embed is dropped
- Multiple threads share same SAPI, last to shutdown cleans up (only one live SAPI possible per-process)
- `ensure_sapi()` upgrades Weak or creates new Sapi
- Prevents SAPI singleton from living forever
- Enables clean shutdown

**Pattern**:
1. First `ensure_sapi()`: Creates Arc<Sapi>, stores Weak
2. Subsequent calls: Upgrade Weak to Arc (cheap)
3. Last Arc dropped: Weak upgrade fails, new Sapi created if needed

---

## Memory Management

### C String Lifecycle

**Creation**:
```rust
let c_str = estrdup("some string");  // PHP's allocator
```

**Usage**:
```rust
globals.request_info.request_method = c_str;
```

**Cleanup**:
```rust
maybe_efree(globals.request_info.request_method as *mut u8);
globals.request_info.request_method = std::ptr::null_mut();
```

**Important rules**:
- Always pair estrdup with efree
- Use maybe_efree to check for null
- Free in sapi_module_deactivate callback
- Never use after free
- Only use strings from estrdup in PHP types and functions

### Request Context Lifecycle

The `RequestContext` type is stored in the PHP SAPI `server_context` global.
This will be set per-request, for the lifetime of the request on that thread.
Uses Box<T> into_raw and from_raw to give ownership to the PHP SAPI and then
take it back when the request is done so the request data may live through
all the PHP SAPI callbacks.

**Allocation**:
```rust
let context = Box::new(RequestContext { ... });
let raw_ptr = Box::into_raw(context) as *mut c_void;
globals.server_context = raw_ptr;
```

**Access**:
```rust
let ctx = unsafe { &mut *(ptr as *mut RequestContext) };
```

**Deallocation**:
```rust
let boxed = unsafe { Box::from_raw(ptr as *mut RequestContext) };
// Box is dropped, freeing memory
```

**Critical**: Must reclaim Box before request ends to prevent leak

### RAII Patterns

**FileHandleScope**:
- Owns zend_file_handle and path C string
- Drop destroys handle and frees path
- Ensures cleanup even on bailout

**RequestScope**:
- Calls php_request_startup in constructor
- Calls php_request_shutdown in Drop
- Wraps entire request execution
- Triggers request variable cleanup when dropped

**Benefits**:
- Automatic cleanup on unwinding
- No resource leaks on panic/bailout
- Clear ownership semantics

---

## Important Gotchas and Edge Cases

### 1. Bailout Handling

**Problem**: PHP uses longjmp for fatal errors
**Solution**: `try_catch_first` wraps script execution
**Gotcha**: Code after bailout doesn't run
**Mitigation**: Use RAII scopes for cleanup

This one is very important to understand for stability. PHP often fails by
panicking. If it is allowed to unwind out of PHP and into Node.js this will
segfault the process. It's _extremely_ important not only that the try catch
logic remain intact, but also that all Rust types which enter the scope of the
PHP execution properly handle bailouts and be cleaned up accordinaly. Otherwise
you may get segfaults from the Rust code being handled improperly, or memory
leaks from things not being cleaned up as expected.

### 2. REQUEST_URI Preservation

**Problem**: Rewriting changes URI before PHP sees it
**Solution**: Capture REQUEST_URI before rewriting
**Impact**: PHP sees original URI in $_SERVER['REQUEST_URI']

### 3. Path Translation with Trailing Slash

**Problem**: `/foo/` vs `/foo` behave differently
**Current behavior**:
- `/foo/` → tries `foo/index.php`, falls back to `foo`
- `/foo` → tries `foo` (exact match)

### 4. RefUnwindSafe Issue

**Problem**: try_catch_first requires FnOnce: RefUnwindSafe
**Solution**: Set up RequestContext BEFORE try_catch_first
**Why**: Avoids capturing mutable references in closure

Basically, you just can't pass anything into the try_catch_first scope that may
be mutated non-atomically as failure partway through a write and then resuming
at the inner catch_unwind of the try_catch_first would be unsafe.

### 5. SAPI Singleton

**Problem**: Multiple SAPI initializations crash PHP
**Solution**: OnceCell with Weak reference pattern
**Gotcha**: Must use ensure_sapi(), never Sapi::new() directly

### 6. Thread-Local Context Access

**Problem**: RequestContext is thread-local
**Gotcha**: Accessing from wrong thread returns None
**Impact**: SAPI callbacks must run on same thread as request

### 7. Error Observer Crashes

**Problem**: Registering error observers crashes Laravel
**Current status**: Error observers disabled (commented out)
**Workaround**: Errors captured via log_message callback

Error observers are the only way to correctly capture all possible exception
paths, however registering an error observer in ZTS (multi-threaded) code
breaks how ZTS tracks exceptions itself and so will crash. This seems to be
an unsupported feature in PHP at present.

### 8. Header Line Merging

**Problem**: Some headers (Set-Cookie) require separate lines
**Current**: Headers class has getLine() but may be incorrect
**Impact**: Multiple Set-Cookie headers might be merged

### 9. Output Buffering

**Problem**: PHP can buffer output internally
**Solution**: `output_buffering=0` in INI
**Impact**: Ensures immediate ub_write callbacks

### 10. Script Execution Scope

**Problem**: Bailout can skip request shutdown
**Solution**: Nested try_catch blocks
**Pattern**:
```rust
let _request_scope = RequestScope::new()?;
{
  let mut file_handle = FileHandleScope::new(path);
  try_catch(|| php_execute_script(&mut file_handle))?;
}
// RequestScope drops here, ensuring shutdown
```

---

## Development Guidelines

### Adding New SAPI Callbacks

1. Define extern "C" function with correct signature
2. Access RequestContext via `RequestContext::current()`
3. Handle null pointers and edge cases
4. Update response_builder or request state
5. Return appropriate value (bytes written, etc.)
6. Test with various PHP scripts

### Modifying Request Handling

1. Consider impact on REQUEST_URI preservation
2. Ensure C strings are properly freed
3. Update exception handling if needed
4. Test with Laravel, WordPress, Symfony
5. Verify thread safety

### Memory Safety Checklist

- [ ] All estrdup calls paired with efree
- [ ] RAII scopes used for cleanup
- [ ] Raw pointers are valid when dereferenced
- [ ] RequestContext reclaimed before request ends
- [ ] No use-after-free in SAPI callbacks
- [ ] Proper null checks before dereferencing

### Testing Considerations

**Unit tests**:
- Use MockRoot for filesystem setup
- Test path translation edge cases
- Verify error handling

**Integration tests**:
- Test with real PHP frameworks
- Verify concurrency (multiple simultaneous requests)
- Test bailout handling
- Check memory leaks with valgrind

**Performance tests**:
- Measure request throughput
- Compare to mod_php, PHP-FPM
- Profile with perf, flamegraph

### Debugging Tips

**Enable debug build**:
```bash
npm run build:debug
```

**Check SAPI callbacks**:
```rust
eprintln!("ub_write called with {} bytes", str_length);
```

**Inspect RequestContext**:
```rust
if let Some(ctx) = RequestContext::current() {
  eprintln!("Request: {:?}", ctx.request());
}
```

**Use valgrind**:
```bash
valgrind --leak-check=full ./target/debug/php-main
```

**PHP errors**:
- Check `response.log` for captured errors
- Enable `display_errors=1` in INI for testing
- Use `error_log()` in PHP to capture messages

### Common Patterns

**Safe C string handling**:
```rust
let c_str = estrdup("value");
// Use c_str...
maybe_efree(c_str.cast::<u8>());
```

**Accessing globals**:
```rust
let mut globals = SapiGlobals::get_mut();
globals.request_info.request_method = c_str;
```

**RAII scope**:
```rust
{
  let _scope = SomeScope::new()?;
  // Do work...
  // _scope dropped here
}
```

**Error conversion**:
```rust
.map_err(|e| EmbedRequestError::CustomError(e.to_string()))
```

---

## Further Reading

- [PHP Internals Book](https://www.phpinternalsbook.com/)
- [ext-php-rs documentation](https://docs.rs/ext-php-rs/)
- [PHP SAPI documentation](https://www.php.net/manual/en/internals2.buildsys.phpsapi.php)
- [Zend Engine documentation](https://www.phpinternalsbook.com/php7/zend_engine.html)
- [NAPI documentation](https://nodejs.org/api/n-api.html)
- [Project INTERNALS.md](../INTERNALS.md)
- [Project CLAUDE.md](../CLAUDE.md)

---

## Contributing

When contributing to this codebase:

1. Read this README thoroughly
2. Understand the request lifecycle
3. Follow memory safety guidelines
4. Test with multiple PHP frameworks
5. Document any new gotchas or edge cases
6. Update this README with architectural changes

This codebase is complex due to the nature of embedding PHP, but following these guidelines will help maintain safety and correctness.
