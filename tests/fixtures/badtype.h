/* BridgeKit test fixture - a header using types OUTSIDE the pinned
 * grammar on purpose. Generation from this file must fail with a
 * hard error naming the offending token (float first) - BridgeKit
 * never guesses a mapping. Invented fixture; not a shipped kit. */
#ifndef BADTYPE_H
#define BADTYPE_H

typedef struct BkThing BkThing;

BK_API const char *bk_version(void);
BK_API void bk_free_string(char *p);
BK_API int bk_scale(BkThing *thing, float factor);
BK_API double bk_measure(BkThing *thing);

#endif
