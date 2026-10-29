/* BridgeKit test fixture - trimmed copy of the shipped WatchKit
 * header (include/watchkit.h in SeanShaffer88/WatchKit, v0.2.0,
 * MIT). Only a representative subset of the prototypes is kept: the
 * lifecycle, uint64_t/uint32_t parameters, and void-return shapes
 * BridgeKit must map. Documentation comments are trimmed; the
 * declarations are character-identical to the shipped header. */
#ifndef WATCHKIT_H
#define WATCHKIT_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32) && defined(WA_BUILD)
#define WA_API __declspec(dllexport)
#elif defined(_WIN32)
#define WA_API __declspec(dllimport)
#else
#define WA_API
#endif

enum {
    WA_OK = 0,
    WA_ERR_INVALID_ARG = 1,
    WA_ERR_NOT_SUPPORTED = 2,
    WA_ERR_NOT_FOUND = 3,
    WA_ERR_ACCESS_DENIED = 4,
    WA_ERR_PLATFORM = 5,
    WA_ERR_INTERNAL = 6,
    WA_ERR_PARSE = 7
};

typedef struct WaWatcher WaWatcher;

WA_API const char *wa_version(void);
WA_API const char *wa_last_error(void);
WA_API int wa_last_error_code(void);
WA_API void wa_free_string(char *p);

/* Create a watcher from a config JSON object. */
WA_API int wa_create(const char *config_json, WaWatcher **out_watcher);

/* Consume pending events with seq > since_seq, up to max_events. */
WA_API int wa_poll_events(WaWatcher *watcher, uint64_t since_seq,
                          uint32_t max_events, char **out_json);

/* Counters, the running backend, and cache statistics as JSON. */
WA_API int wa_status(WaWatcher *watcher, char **out_json);

/* Close and free the handle, joining the background thread. */
WA_API void wa_close(WaWatcher *watcher);

#ifdef __cplusplus
}
#endif
#endif /* WATCHKIT_H */
