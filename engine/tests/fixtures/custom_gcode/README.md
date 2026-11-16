# Changements de couleur / pause / G-code perso par couche
cube.3mf : export 3MF d'OrcaSlicer 2.4.1 (cube20, presets make_refs, arrangé).
cube_custom.3mf : idem + Metadata/custom_gcode_per_layer.xml (z=5 couleur, z=10 pause, z=15 M117).
*.orca.gcode : orca-slicer --slice 0 sur chaque 3MF. Régénérer : nix develop -c python3 make_ref.py
