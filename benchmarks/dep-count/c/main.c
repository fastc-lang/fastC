#include <curl/curl.h>
#include <stdio.h>
#include <stdlib.h>

// Toy HTTP-fetch program for the dep-count benchmark. C's "deps"
// are libcurl + libc; the harness counts libcurl as a non-libc
// include.

int main(void) {
    CURL *curl = curl_easy_init();
    if (!curl) return 1;
    curl_easy_setopt(curl, CURLOPT_URL, "http://example.com");
    long status = 0;
    CURLcode res = curl_easy_perform(curl);
    if (res == CURLE_OK) {
        curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &status);
    }
    curl_easy_cleanup(curl);
    return (int)status;
}
