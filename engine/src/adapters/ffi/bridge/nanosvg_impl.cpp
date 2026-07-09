// Implémentation nanosvg : dans OrcaSlicer elle vit dans la GUI
// (BitmapCache.cpp) ; le build headless la référence (EmbossShape/SVG)
// sans la définir — fournie ici pour le link du bridge.
#define NANOSVG_IMPLEMENTATION
#include "nanosvg/nanosvg.h"
#define NANOSVGRAST_IMPLEMENTATION
#include "nanosvg/nanosvgrast.h"
