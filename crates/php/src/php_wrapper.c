// TODO: Review these dependencies. They were all auto-added by IDE completion.
//       There's likely consolidated headers to get things from.
#include "SAPI.h"
#include "php_main.h"
#include "zend.h"
#include "zend_alloc.h"
#include "zend_execute.h"
#include "zend_frameless_function.h"
#include "zend_globals_macros.h"
#include "zend_property_hooks.h"
#include "zend_types.h"
#include "zend_variables.h"

#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>

#include <sapi/embed/php_embed.h>

#include <ext/standard/head.h>
#include <ext/standard/info.h>

typedef struct string_array {
  size_t buffer_size;
  size_t used_size;
  size_t count;
  char* buffer;
  size_t* offsets;
} string_array;

string_array* string_array_new(size_t initial_size) {
  string_array* arr = (string_array*)malloc(sizeof(string_array));
  if (!arr) return NULL;

  arr->buffer_size = initial_size;
  arr->used_size = 0;
  arr->count = 0;
  arr->buffer = (char*)malloc(initial_size * sizeof(char));
  arr->offsets = (size_t*)malloc(initial_size * sizeof(size_t));

  if (!arr->buffer || !arr->offsets) {
    free(arr->buffer);
    free(arr->offsets);
    free(arr);
    return NULL;
  }

  return arr;
}
bool string_array_grow(string_array* arr, size_t new_size) {
  char* new_buffer = (char*)realloc(arr->buffer, new_size * sizeof(char));
  size_t* new_offsets = (size_t*)realloc(arr->offsets, new_size * sizeof(size_t));

  if (!new_buffer || !new_offsets) {
    free(new_buffer);
    free(new_offsets);
    return false;
  }

  arr->buffer = new_buffer;
  arr->offsets = new_offsets;
  arr->buffer_size = new_size;

  return true;
}
bool string_array_add(string_array* arr, const char* str) {
  size_t str_len = strlen(str) + 1; // include the null terminator

  if (arr->used_size + str_len > arr->buffer_size) {
    if (!string_array_grow(arr, (arr->buffer_size + str_len) * 2)) {
      return false;
    }
  }

  arr->offsets[arr->count] = arr->used_size;
  strcpy(&arr->buffer[arr->used_size], str);
  arr->used_size += str_len;
  arr->count++;

  return true;
}
const char* string_array_get(string_array* arr, size_t index) {
  if (index >= arr->count) {
    return NULL;
  }
  return &arr->buffer[arr->offsets[index]];
}
bool string_array_remove(string_array* arr, size_t index) {
  if (index >= arr->count) {
    return false;
  }

  size_t start_offset = arr->offsets[index];
  size_t next_offset = (index + 1 < arr->count) ? arr->offsets[index + 1] : arr->used_size;
  size_t length_to_move = arr->used_size - next_offset;

  memmove(&arr->buffer[start_offset], &arr->buffer[next_offset], length_to_move);
  arr->used_size -= (next_offset - start_offset);

  for (size_t i = index; i < arr->count - 1; ++i) {
    arr->offsets[i] = arr->offsets[i + 1] - (next_offset - start_offset);
  }
  arr->count--;

  return true;
}
void string_array_free(string_array* arr) {
  free(arr->buffer);
  free(arr->offsets);
  free(arr);
}


/**
 * A header key/value pair.
 */
typedef struct php_http_header {
  const char* key;
  string_array* values;
} php_http_header;

php_http_header* php_http_header_init(php_http_header* self, const char* key, string_array* values) {
  self->key = key;
  self->values = values;
  return self;
}
php_http_header* php_http_header_new(const char* key) {
  php_http_header* self = (php_http_header*)malloc(sizeof(php_http_header));
  if (!self) return NULL;

  string_array* values = string_array_new(1);
  if (!values) return NULL;

  return php_http_header_init(self, key, values);
}
const char* php_http_header_key(php_http_header* self) {
  return self->key;
}
string_array* php_http_header_values(php_http_header* self) {
  return self->values;
}
bool php_http_header_add_value(php_http_header* self, const char* value) {
  return string_array_add(self->values, value);
}
const char* php_http_header_get_value(php_http_header* self, size_t index) {
  return string_array_get(self->values, index);
}
size_t php_http_header_value_count(php_http_header* self) {
  return self->values->count;
}
bool php_http_header_remove_value(php_http_header* self, size_t index) {
  return string_array_remove(self->values, index);
}
void php_http_header_free(php_http_header* self) {
  free(self);
}

/**
 * A collection of headers.
 */
typedef struct php_http_headers {
  size_t allocated;
  size_t count;
  php_http_header* headers;
} php_http_headers;

bool php_http_headers_grow(php_http_headers* self, size_t count) {
  size_t new_size = (self->allocated + count) * sizeof(php_http_header);
  self->headers = (php_http_header*)realloc(self->headers, new_size);
  if (self->headers == NULL) return false;
  self->allocated += count;
  return true;
}
php_http_headers* php_http_headers_init(php_http_headers* self) {
  self->allocated = 0;
  self->count = 0;
  self->headers = NULL;
  return self;
}
php_http_headers* php_http_headers_new(size_t count) {
  php_http_headers* self = (php_http_headers*)malloc(sizeof(php_http_headers));
  if (self == NULL) return NULL;

  php_http_headers_init(self);
  if (count > 0 && !php_http_headers_grow(self, count)) {
    free(self);
    return NULL;
  }

  return self;
}
void php_http_headers_free(php_http_headers* self) {
  free(self->headers);
  free(self);
}
bool php_http_headers_has_room(php_http_headers* self, size_t count) {
  return self->allocated - self->count >= count;
}
size_t php_http_headers_count(php_http_headers* self) {
  return self->count;
}
php_http_header* php_http_headers_get(php_http_headers* self, size_t index) {
  return &self->headers[index];
}
int php_http_headers_find_index(php_http_headers* self, const char* key) {
  for (int i = 0; i < (int)self->count; i++) {
    if (strcmp(self->headers[i].key, key) == 0) {
      return i;
    }
  }
  return -1;
}
php_http_header* php_http_headers_find(php_http_headers* self, const char* key) {
  int index = php_http_headers_find_index(self, key);
  if (index == -1) return NULL;
  return &self->headers[index];
}
php_http_header* php_http_headers_remove(php_http_headers* self, const char* key) {
  int i = php_http_headers_find_index(self, key);
  if (i == -1) return NULL;

  php_http_header* header = &self->headers[i];
  for (int j = i; j < (int)self->count - 1; j++) {
    self->headers[j] = self->headers[j + 1];
  }
  self->count--;

  return header;
}
php_http_headers* php_http_headers_push(php_http_headers* self, const char* key, const char* value) {
  php_http_header* header = php_http_headers_find(self, key);
  if (header != NULL) {
    if (!php_http_header_add_value(header, value)) {
      return NULL;
    }
    return self;
  }

  if (!php_http_headers_has_room(self, 1) && !php_http_headers_grow(self, 1)) {
    return NULL;
  }

  string_array* values = string_array_new(1);
  if (!values) return NULL;

  string_array_add(values, value);
  header = &self->headers[self->count++];
  php_http_header_init(header, key, values);

  return self;
}

/**
 * An incoming request.
 */
typedef struct php_http_request {
  const char* method;
  const char* path;
  php_http_headers headers;
  const char* body;
} php_http_request;

php_http_request* php_http_request_init(php_http_request* self) {
  self->method = NULL;
  self->path = NULL;
  php_http_headers_init(&self->headers);
  self->body = NULL;
  return self;
}
php_http_request* php_http_request_new() {
  php_http_request* self = (php_http_request*)malloc(sizeof(php_http_request));
  if (self == NULL) return NULL;

  return php_http_request_init(self);
}
void php_http_request_free(php_http_request* self) {
  free((void*)self->method);
  free((void*)self->path);
  php_http_headers_free(&self->headers);
  free((void*)self->body);
  free(self);
}
bool php_http_request_set_method(php_http_request* self, const char* method) {
  self->method = strdup(method);
  return true;
}
const char* php_http_request_get_method(php_http_request* self) {
  return self->method;
}
bool php_http_request_set_path(php_http_request* self, const char* path) {
  self->path = strdup(path);
  return true;
}
const char* php_http_request_get_path(php_http_request* self) {
  return self->path;
}
bool php_http_request_set_body(php_http_request* self, const char* body) {
  self->body = strdup(body);
  return true;
}
const char* php_http_request_get_body(php_http_request* self) {
  return self->body;
}

/**
 * An outgoing response.
 */
typedef struct php_http_response {
  int status;
  php_http_headers headers;
  const char* body;
} php_http_response;

php_http_response* php_http_response_init(php_http_response* self) {
  self->status = 0;
  php_http_headers_init(&self->headers);
  self->body = NULL;
  return self;
}
php_http_response* php_http_response_new() {
  php_http_response* self = (php_http_response*)malloc(sizeof(php_http_response));
  if (self == NULL) return NULL;

  return php_http_response_init(self);
}
void php_http_response_free(php_http_response* self) {
  php_http_headers_free(&self->headers);
  free((void*)self->body);
  free(self);
}
bool php_http_response_set_status(php_http_response* self, int status) {
  self->status = status;
  return true;
}
int php_http_response_get_status(php_http_response* self) {
  return self->status;
}
bool php_http_response_set_body(php_http_response* self, const char* body) {
  self->body = strdup(body);
  return true;
}
const char* php_http_response_get_body(php_http_response* self) {
  return self->body;
}
php_http_headers* php_http_response_get_headers(php_http_response* self) {
  return &self->headers;
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
php_http_response* php_http_handle_request(const char* code, const char* filename, php_http_request* request) {
  php_http_response* response = php_http_response_new();
  if (response == NULL) return NULL;

  // Teardown initial request.
  php_request_shutdown((void*) 0);

  // Set up $_SERVER values.
  // SG(request_info).script_filename = estrdup(filename);
  // SG(request_info).php_self = estrdup(request->path);
  SG(request_info).request_method = estrdup(request->method);
  // SG(request_info).request_body =
  SG(request_info).path_translated = estrdup(request->path);
  // SG(request_info).query_string = estrdup(request->query_string);
  // SG(request_info).request_uri = estrdup(request->request_uri);
  // SG(request_info).cookie_data = estrdup(request->cookie_data);
  // SG(request_info).content_type = estrdup(request->content_type);
  // SG(request_info).content_length = estrdup(request->content_length);
  // SG(request_info).server_software = estrdup(request->server_software); // wattpm?
  // SG(request_info).gateway_interface = estrdup(request->gateway_interface); // CGI/1.1
  // SG(request_info).request_time_float = request->request_time_float;
  // SG(request_info).document_root = estrdup(request->document_root);
  // SG(request_info).remote_addr = estrdup(request->remote_addr);
  // SG(request_info).remote_host = estrdup(request->remote_host);
  // SG(request_info).remote_port = estrdup(request->remote_port);
  // SG(request_info).remote_user = estrdup(request->remote_user);
  // SG(request_info).server_port = estrdup(request->server_port);
  // SG(request_info).script_name = estrdup(request->script_name);
  // SG(request_info).php_auth_digest = estrdup(request->php_auth_digest);
  // SG(request_info).php_auth_user = estrdup(request->php_auth_user);
  // SG(request_info).php_auth_pw = estrdup(request->php_auth_pw);
  // SG(request_info).auth_type = estrdup(request->auth_type);
  // SG(request_info).path_info = estrdup(request->path_info);

  // SG(server_context) needs to be non-zero size because...reasons. ¯\_(ツ)_/¯
  SG(server_context) = (void*)(1);  // Sigh.

  // Needs to be set _after_ php_request_startup, also because reasons.
  SG(request_info).proto_num = 110;

  // Start new request now that we've setup the environment fully.
  if (php_request_startup() == FAILURE) {
    return NULL;
  }

  // php_embed_module.flush =

  zval retval;
  zend_try {
    // size_t len = strlen(code);
    // zend_eval_stringl_ex((char*)code, len, &retval, filename, true);

    zend_eval_string((char*)code, NULL, filename);

    if (EG(exception)) {
      // Can't call zend_clear_exception because there isn't a current
      // execution stack (ie, `EG(current_execute_data)`)
      zend_object* e = EG(exception);
      EG(exception) = NULL;
      // TODO: do something with the error...
      zval_ptr_dtor((zval*)e);
    }
  } zend_catch {
    return NULL;
  } zend_end_try();

  // Populate response fields
  php_http_headers_push(&response->headers, "Content-Type", "text/plain");
  php_http_response_set_status(response, SG(sapi_headers).http_response_code);

  // TODO: Need to figure out how php://output works.
  response->body = "Hello, World!";

  return response;
}
