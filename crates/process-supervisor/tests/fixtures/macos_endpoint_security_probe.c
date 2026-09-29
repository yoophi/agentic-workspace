#include <EndpointSecurity/EndpointSecurity.h>
#include <stdio.h>

static const char *result_name(es_new_client_result_t result) {
    switch (result) {
    case ES_NEW_CLIENT_RESULT_SUCCESS:
        return "success";
    case ES_NEW_CLIENT_RESULT_ERR_INVALID_ARGUMENT:
        return "invalid_argument";
    case ES_NEW_CLIENT_RESULT_ERR_INTERNAL:
        return "internal";
    case ES_NEW_CLIENT_RESULT_ERR_NOT_ENTITLED:
        return "not_entitled";
    case ES_NEW_CLIENT_RESULT_ERR_NOT_PERMITTED:
        return "not_permitted";
    case ES_NEW_CLIENT_RESULT_ERR_NOT_PRIVILEGED:
        return "not_privileged";
    case ES_NEW_CLIENT_RESULT_ERR_TOO_MANY_CLIENTS:
        return "too_many_clients";
    }
    return "unknown";
}

int main(void) {
    es_client_t *client = NULL;
    es_new_client_result_t result = es_new_client(
        &client, ^(es_client_t *ignored_client, const es_message_t *ignored_message) {
          (void)ignored_client;
          (void)ignored_message;
        });
    printf("es_new_client_result=%d name=%s\n", result, result_name(result));

    if (result == ES_NEW_CLIENT_RESULT_SUCCESS) {
        es_return_t deleted = es_delete_client(client);
        printf("es_delete_client_result=%d\n", deleted);
        return deleted == ES_RETURN_SUCCESS ? 0 : 1;
    }

    switch (result) {
    case ES_NEW_CLIENT_RESULT_ERR_NOT_ENTITLED:
    case ES_NEW_CLIENT_RESULT_ERR_NOT_PERMITTED:
    case ES_NEW_CLIENT_RESULT_ERR_NOT_PRIVILEGED:
        return 0;
    default:
        return 1;
    }
}
