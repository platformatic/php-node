// TODO: Review these dependencies. They were all auto-added by IDE completion.
//       There's likely consolidated headers to get things from.
#include "SAPI.h"
#include "TSRM.h"
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
  lh_request_t* request;
  lh_response_builder_t* response_builder;
} php_server_context_t;

static const char HARDCODED_INI[] =
  "display_errors=0\n"
  "register_argc_argv=1\n"
	"log_errors=1\n"
	"implicit_flush=1\n"
	"memory_limit=128MB\n"
	"output_buffering=0\n";

int php_http_startup(sapi_module_struct* sapi_module) {
  struct php_ini_builder ini_builder;
  php_ini_builder_init(&ini_builder);
  php_ini_builder_prepend_literal(&ini_builder, HARDCODED_INI);
  sapi_module->ini_entries = php_ini_builder_finish(&ini_builder);
  php_ini_builder_deinit(&ini_builder);

  return php_module_startup(sapi_module, NULL);
}

int php_http_deactivate() {
  php_server_context_t* context = SG(server_context);
  if (!context) return SUCCESS;

  SG(server_context) = NULL;

  SG(request_info).argc = 0;
  SG(request_info).argv = NULL;

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

  return SUCCESS;
}

size_t php_http_ub_write(const char* str, size_t len) {
  php_server_context_t* context = SG(server_context);
  return lh_response_builder_body_write(context->response_builder, str, len);
}

void php_http_flush() {
  if (!SG(headers_sent)) {
    sapi_send_headers();
    SG(headers_sent) = 1;
  }
}

void php_http_send_header(
  sapi_header_struct* sapi_header,
  __attribute__((unused)) void* server_context
) {
  // Not sure _why_ this is necessary, but it is.
  if (sapi_header == NULL) return;
  // printf("Header: %s\n", sapi_header->header);
}

int php_http_send_headers(sapi_headers_struct* sapi_headers) {
  // php_server_context_t* context = SG(server_context);
  sapi_header_struct* h;
  zend_llist_position pos;

  h = zend_llist_get_first_ex(&sapi_headers->headers, &pos);
  while (h) {
    if ( h->header_len > 0 ) {
      // printf("Header: %s\n", h->header);
    }
    h = zend_llist_get_next_ex(&sapi_headers->headers, &pos);
  }
  return SAPI_HEADER_SENT_SUCCESSFULLY;
}

size_t php_http_read_post(char* buffer, size_t count_bytes) {
  php_server_context_t* context = SG(server_context);
  return lh_request_body_read(context->request, buffer, count_bytes);
}

char* php_http_read_cookies() {
  return SG(request_info).cookie_data;
}

void php_http_register_server_variables(zval* track_vars_array) {
  php_import_environment_variables(track_vars_array);
}

void php_http_log_message(
  const char* message,
  __attribute__((unused)) int syslog_type_int
) {
  php_server_context_t* context = SG(server_context);
  size_t len = strlen(message);
  lh_response_builder_log_write(context->response_builder, message, len);
}

static sapi_module_struct php_http_sapi_module = {
	"php-http",						              /* name */
	"PHP/HTTP",					                /* pretty name */

	php_http_startup,				            /* startup */
	php_module_shutdown_wrapper,      	/* shutdown */

	NULL,				                        /* activate */
	php_http_deactivate,			          /* deactivate */

	php_http_ub_write,				          /* unbuffered write */
	php_http_flush,					            /* flush */

	NULL,							                  /* get uid */
	NULL,				                        /* getenv */

	php_error,						              /* error handler */

	NULL,							                  /* header handler */
	php_http_send_headers,			        /* send headers handler */
	php_http_send_header,							  /* send header handler */

	php_http_read_post,				          /* read POST data */
	php_http_read_cookies,			        /* read Cookies */

	php_http_register_server_variables,	/* register server variables */
	php_http_log_message,			          /* Log message */

	NULL,							                  /* Get request time */
	NULL,							                  /* Child terminate */

	STANDARD_SAPI_MODULE_PROPERTIES
};

zend_result php_http_init(int argc, char** argv) {
#ifdef ZTS
  php_tsrm_startup();
#endif

  if (argc > 0) {
    php_http_sapi_module.executable_location = argv[0];
  }
  sapi_startup(&php_http_sapi_module);

  php_module_startup(&php_http_sapi_module, NULL);

  return SUCCESS;
}

zend_result php_http_destruct() {
  // Why is this needed???
  // sapi_module.flush = NULL;

  // php_http_sapi_module.shutdown(&php_http_sapi_module);
  php_module_shutdown();

  sapi_shutdown();

#ifdef ZTS
  tsrm_shutdown();
#endif

  return SUCCESS;
}

/**
 * TODO:
 * - Provide high-performance PSR-7 implementation on libuv?
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
  lh_response_builder_t* response_builder = lh_response_builder_new();

  if (php_http_sapi_module.startup(&php_http_sapi_module) == FAILURE) {
#ifdef ZTS
    tsrm_shutdown();
#endif
    return lh_response_builder_build(response_builder);
  }

  zend_first_try {
    // This is where we store the stuff for associating callbacks with this request.
    php_server_context_t context = {
      .request = request,
      .response_builder = response_builder
    };

    SG(server_context) = &context;

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
      return lh_response_builder_build(response_builder);
    }

    // Needs to be set _after_ php_request_startup, also because reasons.
    SG(request_info).proto_num = 110;

    size_t len = strlen(code);
    zend_eval_stringl_ex((char*)code, len, NULL, filename, false);

    if (EG(exception)) {
      zval rv;
      zend_class_entry* exception_ce = zend_get_exception_base(EG(exception));
      zval *msg = zend_read_property_ex(exception_ce, EG(exception), ZSTR_KNOWN(ZEND_STR_MESSAGE), /* silent */ false, &rv);

      SG(sapi_headers).http_response_code = 500;
      lh_response_builder_exception(response_builder, Z_STRVAL_P(msg));

      zend_object_release(EG(exception));
      EG(exception) = NULL;
      EG(exit_status) = 1;
    }

    const char* mime = SG(sapi_headers).mimetype;
    if (mime == NULL) {
      mime = "text/plain";
    }
    lh_response_builder_header(response_builder, "Content-Type", mime);
    lh_response_builder_status_code(response_builder, SG(sapi_headers).http_response_code);

    php_request_shutdown(NULL);
    lh_headers_free(headers);

    php_header();
    php_output_flush_all();
  } zend_end_try();

  return lh_response_builder_build(response_builder);
}
