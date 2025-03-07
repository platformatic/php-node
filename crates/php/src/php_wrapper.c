// TODO: Review these dependencies. They were all auto-added by IDE completion.
//       There's likely consolidated headers to get things from.
#include "SAPI.h"
#include "php.h"
#include "php_main.h"
#include "zend.h"
#include "zend_alloc.h"
#include "zend_execute.h"
#include "zend_frameless_function.h"
#include "zend_globals_macros.h"
#include "zend_property_hooks.h"
#include "zend_types.h"
#include "zend_variables.h"
#include "zend_exceptions.h"
#include "lang_handler.h"
#include "php_ini_builder.h"

#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>

#include <sapi/embed/php_embed.h>

#include <ext/standard/head.h>
#include <ext/standard/info.h>

typedef struct php_server_context_s {
    int foo;
} php_server_context_t;

int php_sapi_module_startup(sapi_module_struct* sapi_module) {
  php_server_context_t* context = (php_server_context_t*)SG(server_context);
  printf("Startup from %d\n", context->foo);
  return SUCCESS;
}

int php_sapi_activate() {
  php_server_context_t* context = (php_server_context_t*)SG(server_context);
  printf("Activate from %d\n", context->foo);
  return SUCCESS;
}

int php_sapi_deactivate() {
  php_server_context_t* context = (php_server_context_t*)SG(server_context);
  printf("Deactivate from %d\n", context->foo);
  return SUCCESS;
}

size_t sapi_ub_write(const char *str, size_t str_length) {
  php_server_context_t* context = (php_server_context_t*)SG(server_context);
  printf("%.*s from %d", (int)str_length, str, context->foo);
  return str_length;
}

void sapi_node_flush() {
  php_server_context_t* context = (php_server_context_t*)SG(server_context);
  printf("Flush occurred from %d\n", context->foo);
  if (!SG(headers_sent)) {
    sapi_send_headers();
    SG(headers_sent) = 1;
  }
}

// void sapi_send_header(sapi_header_struct *sapi_header, void *server_context) {
//   // Not sure _why_ this is necessary, but it is.
//   if (sapi_header == NULL) return;
//   php_server_context_t* context = (php_server_context_t*)server_context;
//   printf("Header: %s from %d\n", sapi_header->header, context->foo);
// }

int php_sapi_send_headers(sapi_headers_struct *sapi_headers) {
  php_server_context_t* context = (php_server_context_t*)SG(server_context);
  printf("Headers sent from %d\n", context->foo);

  sapi_header_struct  *h;
  zend_llist_position pos;

  h = zend_llist_get_first_ex(&sapi_headers->headers, &pos);
  while (h) {
    if ( h->header_len > 0 ) {
      printf("Header: %s\n", h->header);
    }
    h = zend_llist_get_next_ex(&sapi_headers->headers, &pos);
  }
  return 0;
}

// TODO: Read n bytes from request body in ctx, memcpy to buffer, return remaining bytes.
size_t php_sapi_read_post(char *buffer, size_t count_bytes) {
  php_server_context_t* context = (php_server_context_t*)SG(server_context);
  printf("Read post from %d\n", context->foo);
  return 0;
}

char* php_sapi_read_cookies() {
  return SG(request_info).cookie_data;
}

void php_register_server_variables(zval *track_vars_array) {
  php_server_context_t* context = (php_server_context_t*)SG(server_context);
  printf("Register server variables from %d\n", context->foo);
}

void php_sapi_log_message(const char *message, int syslog_type_int) {
  php_server_context_t* context = (php_server_context_t*)SG(server_context);
  printf("Log message: %s from %d\n", message, context->foo);
}

// static const char HARDCODED_INI[] =
	// "log_errors=1\n"
	// "implicit_flush=1\n"
	// "memory_limit=128MB\n"
	// "output_buffering=0\n";

void php_http_setup() {
  sapi_module.startup = php_sapi_module_startup;

  sapi_module.activate = php_sapi_activate;
  sapi_module.deactivate = php_sapi_deactivate;

  sapi_module.ub_write = sapi_ub_write;
  sapi_module.flush = sapi_node_flush;

  sapi_module.sapi_error = php_error;

  // sapi_module.send_header = sapi_send_header;
  sapi_module.send_headers = php_sapi_send_headers;

  sapi_module.read_post = php_sapi_read_post;
  sapi_module.read_cookies = php_sapi_read_cookies;

  sapi_module.register_server_variables = php_register_server_variables;
  sapi_module.log_message = php_sapi_log_message;

  // struct php_ini_builder ini_builder;
  // php_ini_builder_init(&ini_builder);
  // php_ini_builder_prepend_literal(&ini_builder, HARDCODED_INI);
}

void clean_superglobals() {
    // request
    if (SG(request_info).request_method != NULL) {
        lh_reclaim_str(SG(request_info).request_method);
    }

    // url
    if (SG(request_info).path_translated != NULL) {
        lh_reclaim_str(SG(request_info).path_translated);
    }
    if (SG(request_info).query_string != NULL) {
        lh_reclaim_str(SG(request_info).query_string);
    }
    if (SG(request_info).request_uri != NULL) {
        lh_reclaim_str(SG(request_info).request_uri);
    }

    // headers
    if (SG(request_info).content_type != NULL) {
        lh_reclaim_str(SG(request_info).content_type);
    }
    if (SG(request_info).cookie_data != NULL) {
        lh_reclaim_str(SG(request_info).cookie_data);
    }
}

/**
 * TODO:
 * - Learn how php_stream works and adapt to tokio streams?
 * - Provide high-performance PSR-7 implementation on libuv?
 * - Use thread pool, or let wattpm handle it?
 *   - Don't need multiple instances of the native module and PHP runtime.
 *
 * NOTES:
 *
 * Most PHP frameworks use superglobals ($_SERVER, $_GET, $_POST, $_COOKIE, etc)
 * to access request data which have been injected into the environment.
 * These are populated by the SAPI (Server API) when the request is received.
 * A typical request is simply read via stdin (php://input) and responded to
 * by writing to stdout (php://output).
 *
 * Each SAPI request is handled in an isolated PHP context, but code compilation
 * can be shared making spin up quick. Each of these contexts is single-threaded.
 */
lh_response_t* php_http_handle_request(const char* code, const char* filename, lh_request_t* request) {
  // This is where we store the stuff for associating callbacks with this request.
  // TODO: This should probably contain the request and response objects.
  php_server_context_t* context = malloc(sizeof(php_server_context_t));
  context->foo = 555;
  SG(server_context) = context;

  SG(options) |= SAPI_OPTION_NO_CHDIR;
  SG(headers_sent) = 0;

  SG(request_info).argc = 0;
  SG(request_info).argv = NULL;

  // Reset state
  SG(sapi_headers).http_response_code = 200;

  // Set up superglobals
  SG(request_info).request_method = lh_request_method(request);

  lh_url_t* url = lh_request_url(request);
  SG(request_info).path_translated = (char*) lh_url_path(url);
  SG(request_info).query_string = (char*) lh_url_query(url);
  SG(request_info).request_uri = (char*) lh_url_uri(url);
  // TODO: Add auth fields

  // Could implement a PHP stream to do this?
  // SG(request_info).request_body = lh_request_body(request);

  lh_headers_t* headers = lh_request_headers(request);

  const char* content_type = lh_headers_get(headers, "Content-Type");
  if (content_type == NULL) {
    SG(request_info).content_type = content_type;
  }

  const char* content_length = lh_headers_get(headers, "Content-Length");
  if (content_length != NULL) {
    SG(request_info).content_length = strtoll(content_length, NULL, 10);
  }

  const char* cookie = lh_headers_get(headers, "Cookie");
  SG(request_info).cookie_data = (char*) cookie;

  // Start new request now that we've setup the environment fully.
  if (php_request_startup() == FAILURE) {
    return NULL;
  }

  // Needs to be set _after_ php_request_startup, also because reasons.
  SG(request_info).proto_num = 110;

  zval retval;
  zend_first_try {
    printf("code: %s\n", code);
    zend_eval_string_ex((char*)code, &retval, filename, false);

    if (EG(exception)) {
      zval rv;
      // TODO: Figure out why this fails.
      zval *msg = zend_read_property_ex(zend_ce_exception, EG(exception), ZSTR_KNOWN(ZEND_STR_MESSAGE), /* silent */ false, &rv);
      zend_printf("Exception: %s\n", Z_STRVAL_P(msg));
      zend_object_release(EG(exception));
      EG(exception) = NULL;
    }
  } zend_catch {
    return NULL;
  } zend_end_try();

  zend_try {
    php_request_shutdown(NULL);
  } zend_end_try();
  clean_superglobals();

  // Reset headers to reuse for response object
  lh_headers_free(headers);
  headers = lh_headers_new();

  const char* mime = SG(sapi_headers).mimetype;
  if (mime == NULL) {
    mime = "text/plain";
  }
  lh_headers_set(headers, "Content-Type", mime);

  int status = SG(sapi_headers).http_response_code;
  return lh_response_new(status, headers, "Hello, World!");
}
