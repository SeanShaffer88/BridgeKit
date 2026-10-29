/* BridgeKit test fixture - trimmed copy of the shipped NotifyKit
 * header (include/notifykit.h in SeanShaffer88/NotifyKit, v0.2.0,
 * MIT). Only a representative subset of the prototypes is kept: the
 * lifecycle, out-pointer, handle, and tray shapes BridgeKit must
 * map. Documentation comments are trimmed; the declarations are
 * character-identical to the shipped header. */
#ifndef NOTIFYKIT_H
#define NOTIFYKIT_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
#if defined(_WIN32) && defined(NO_BUILD)
#define NO_API __declspec(dllexport)
#elif defined(_WIN32)
#define NO_API __declspec(dllimport)
#else
#define NO_API
#endif

typedef struct NoCenter NoCenter;
typedef struct NoTray NoTray;

/* Library version, e.g. "0.2.0". Borrowed string; do not free. */
NO_API const char *no_version(void);

/* Last error message on this thread ("" when none). Borrowed. */
NO_API const char *no_last_error(void);

/* Last NO_ERR_* code on this thread (NO_OK when none). */
NO_API int no_last_error_code(void);

/* Free a caller-owned string returned by this library. */
NO_API void no_free_string(char *p);

/* Create a notification center from a JSON config. */
NO_API int no_create(const char *config_json, NoCenter **out_handle);

/* Enqueue one notification; *out_json receives caller-owned JSON. */
NO_API int no_enqueue(NoCenter *handle, const char *item_json,
                      char **out_json);

/* Free a center handle. */
NO_API int no_close(NoCenter *handle);

/* Register a tray icon; *out_handle receives the tray handle. */
NO_API int no_tray_create(const char *config_json, NoTray **out_handle);

/* Select the tray backend: "balloon" or "toast". */
NO_API int no_tray_set_backend(NoTray *handle, const char *name);

/* Show a notification on the tray icon. */
NO_API int no_tray_show(NoTray *handle, const char *show_json,
                        char **out_json);

#ifdef __cplusplus
}
#endif
#endif
