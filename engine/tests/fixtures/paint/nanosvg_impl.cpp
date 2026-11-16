// nanosvg : implémentation absente du build headless (dans la GUI d'Orca,
// BitmapCache.cpp) — requise pour linker liblibslic3r.a.
#define NANOSVG_IMPLEMENTATION
#include "nanosvg/nanosvg.h"
#define NANOSVGRAST_IMPLEMENTATION
#include "nanosvg/nanosvgrast.h"
