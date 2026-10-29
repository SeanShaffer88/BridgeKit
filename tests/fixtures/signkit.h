/* BridgeKit test fixture - trimmed copy of the shipped SignKit
 * header (include/signkit.h in SeanShaffer88/SignKit, v0.1.0, MIT).
 * The full prototype set is kept: SignKit is the caller-owned char*
 * return shape (sg_inspect and kin) BridgeKit must wrap. The
 * documentation comments are trimmed; the declarations are
 * character-identical to the shipped header. */
#ifndef SIGNKIT_H
#define SIGNKIT_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
#if defined(_WIN32) && defined(SG_BUILD)
#define SG_API __declspec(dllexport)
#elif defined(_WIN32)
#define SG_API __declspec(dllimport)
#else
#define SG_API
#endif

/* Library version string. Static storage: do NOT free. */
SG_API const char *sg_version(void);

/* Thread-local message describing the most recent failure. */
SG_API const char *sg_last_error(void);

/* Thread-local code for the most recent failure. */
SG_API int sg_last_error_code(void);

/* Free a string returned by sg_inspect and kin. */
SG_API void sg_free_string(char *p);

/* Inspect a PE file and return a JSON report (caller-owned). */
SG_API char *sg_inspect(const char *path);

/* Compute the Authenticode image hash (caller-owned hex string). */
SG_API char *sg_authenticode_hash(const char *path);

/* Verify a file's Authenticode signature (caller-owned JSON). */
SG_API char *sg_verify(const char *path);

#ifdef __cplusplus
}
#endif
#endif
