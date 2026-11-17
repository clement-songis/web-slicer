// Généré par scripts/gen-gcode-placeholders.mjs depuis OrcaSlicer PrintConfig.cpp — ne pas éditer.
export interface GcodePlaceholder { key: string; type: string; label: string; tooltip: string }
export const GCODE_PLACEHOLDER_GROUPS: { title: string; items: GcodePlaceholder[] }[] = [
	{
		"title": "Slicing State (lecture seule)",
		"items": [
			{
				"key": "zhop",
				"type": "coFloat",
				"label": "Current Z-hop",
				"tooltip": "Contains Z-hop present at the beginning of the custom G-code block."
			}
		]
	},
	{
		"title": "Slicing State (lecture et écriture)",
		"items": [
			{
				"key": "position",
				"type": "coFloats",
				"label": "Position",
				"tooltip": "Position of the extruder at the beginning of the custom G-code block. If the custom G-code travels somewhere else, it should write to this variable so OrcaSlicer knows where it travels from when it gets control back."
			},
			{
				"key": "e_retracted",
				"type": "coFloats",
				"label": "Retraction",
				"tooltip": "Retraction state at the beginning of the custom G-code block. If the custom G-code moves the extruder axis, it should write to this variable so OrcaSlicer de-retracts correctly when it gets control back."
			},
			{
				"key": "e_restart_extra",
				"type": "coFloats",
				"label": "Extra de-retraction",
				"tooltip": "Currently planned extra extruder priming after de-retraction."
			},
			{
				"key": "e_position",
				"type": "coFloats",
				"label": "Absolute E position",
				"tooltip": "Current position of the extruder axis. Only used with absolute extruder addressing."
			}
		]
	},
	{
		"title": "Slicing State",
		"items": [
			{
				"key": "current_extruder",
				"type": "coInt",
				"label": "Current extruder",
				"tooltip": "Zero-based index of currently used extruder."
			},
			{
				"key": "current_object_idx",
				"type": "coInt",
				"label": "Current object index",
				"tooltip": "Specific for sequential printing. Zero-based index of currently printed object."
			},
			{
				"key": "has_wipe_tower",
				"type": "coBool",
				"label": "Has wipe tower",
				"tooltip": "Whether or not wipe tower is being generated in the print."
			},
			{
				"key": "initial_extruder",
				"type": "coInt",
				"label": "Initial extruder",
				"tooltip": "Zero-based index of the first extruder used in the print. Same as initial_tool."
			},
			{
				"key": "initial_tool",
				"type": "coInt",
				"label": "Initial tool",
				"tooltip": "Zero-based index of the first extruder used in the print. Same as initial_extruder."
			},
			{
				"key": "is_extruder_used",
				"type": "coBools",
				"label": "Is extruder used?",
				"tooltip": "Vector of booleans stating whether a given extruder is used in the print."
			},
			{
				"key": "num_extruders",
				"type": "coInt",
				"label": "Number of extruders",
				"tooltip": "Total number of extruders, regardless of whether they are used in the current print."
			},
			{
				"key": "initial_filament_type",
				"type": "coString",
				"label": "Initial filament type",
				"tooltip": "String containing filament type of the first used extruder."
			},
			{
				"key": "has_single_extruder_multi_material_priming",
				"type": "coBool",
				"label": "Has single extruder MM priming",
				"tooltip": "Are the extra multi-material priming regions used in this print?"
			},
			{
				"key": "initial_no_support_extruder",
				"type": "coInt",
				"label": "Initial no support extruder",
				"tooltip": "Zero-based index of the first extruder used for printing without support. Same as initial_no_support_tool."
			},
			{
				"key": "in_head_wrap_detect_zone",
				"type": "coBool",
				"label": "In head wrap detect zone",
				"tooltip": "Indicates if the first layer overlaps with the head wrap zone."
			}
		]
	},
	{
		"title": "Print Statistics",
		"items": [
			{
				"key": "extruded_volume",
				"type": "coFloats",
				"label": "Volume per extruder",
				"tooltip": "Total filament volume extruded per extruder during the entire print."
			},
			{
				"key": "total_toolchanges",
				"type": "coInt",
				"label": "Total tool changes",
				"tooltip": "Number of tool changes during the print."
			},
			{
				"key": "extruded_volume_total",
				"type": "coFloat",
				"label": "Total volume",
				"tooltip": "Total volume of filament used during the entire print."
			},
			{
				"key": "extruded_weight",
				"type": "coFloats",
				"label": "Weight per extruder",
				"tooltip": "Weight per extruder extruded during the entire print. Calculated from filament_density value in Filament Settings."
			},
			{
				"key": "extruded_weight_total",
				"type": "coFloat",
				"label": "Total weight",
				"tooltip": "Total weight of the print. Calculated from filament_density value in Filament Settings."
			},
			{
				"key": "total_layer_count",
				"type": "coInt",
				"label": "Total layer count",
				"tooltip": "Number of layers in the entire print."
			},
			{
				"key": "normal_print_time",
				"type": "coString",
				"label": "Print time (normal mode)",
				"tooltip": "Estimated print time when printed in normal mode (i.e. not in silent mode). Same as print_time."
			},
			{
				"key": "num_printing_extruders",
				"type": "coInt",
				"label": "Number of printing extruders",
				"tooltip": "Number of extruders used during the print."
			},
			{
				"key": "print_time",
				"type": "coString",
				"label": "Print time (normal mode)",
				"tooltip": "Estimated print time when printed in normal mode (i.e. not in silent mode). Same as normal_print_time."
			},
			{
				"key": "printing_filament_types",
				"type": "coString",
				"label": "Used filament types",
				"tooltip": "Comma-separated list of all filament types used during the print."
			},
			{
				"key": "silent_print_time",
				"type": "coString",
				"label": "Print time (silent mode)",
				"tooltip": "Estimated print time when printed in silent mode."
			},
			{
				"key": "total_cost",
				"type": "coFloat",
				"label": "Total cost",
				"tooltip": "Total cost of all material used in the print. Calculated from filament_cost value in Filament Settings."
			},
			{
				"key": "total_weight",
				"type": "coFloat",
				"label": "Total weight",
				"tooltip": "Total weight of the print. Calculated from filament_density value in Filament Settings."
			},
			{
				"key": "total_wipe_tower_cost",
				"type": "coFloat",
				"label": "Total wipe tower cost",
				"tooltip": "Total cost of the material wasted on the wipe tower. Calculated from filament_cost value in Filament Settings."
			},
			{
				"key": "total_wipe_tower_filament",
				"type": "coFloat",
				"label": "Wipe tower volume",
				"tooltip": "Total filament volume extruded on the wipe tower."
			},
			{
				"key": "used_filament",
				"type": "coFloat",
				"label": "Used filament",
				"tooltip": "Total length of filament used in the print."
			},
			{
				"key": "print_time_sec",
				"type": "coString",
				"label": "Print time (seconds)",
				"tooltip": "Total estimated print time in seconds. Replaced with actual value during post-processing."
			},
			{
				"key": "used_filament_length",
				"type": "coString",
				"label": "Filament length (meters)",
				"tooltip": "Total filament length used in meters. Replaced with actual value during post-processing."
			}
		]
	},
	{
		"title": "Objects Info",
		"items": [
			{
				"key": "num_objects",
				"type": "coInt",
				"label": "Number of objects",
				"tooltip": "Total number of objects in the print."
			},
			{
				"key": "num_instances",
				"type": "coInt",
				"label": "Number of instances",
				"tooltip": "Total number of object instances in the print, summed over all objects."
			},
			{
				"key": "scale",
				"type": "coStrings",
				"label": "Scale per object",
				"tooltip": "Contains a string with the information about what scaling was applied to the individual objects. Indexing of the objects is zero-based (first object has index 0).\nExample: 'x:100% y:50% z:100%'."
			},
			{
				"key": "input_filename_base",
				"type": "coString",
				"label": "Input filename without extension",
				"tooltip": "Source filename of the first object, without extension."
			},
			{
				"key": "input_filename",
				"type": "coString",
				"label": "Full input filename",
				"tooltip": "Source filename of the first object."
			},
			{
				"key": "plate_name",
				"type": "coString",
				"label": "Plate name",
				"tooltip": "Name of the plate sliced."
			}
		]
	},
	{
		"title": "Dimensions",
		"items": [
			{
				"key": "first_layer_print_convex_hull",
				"type": "coPoints",
				"label": "First layer convex hull",
				"tooltip": "Vector of points of the first layer convex hull. Each element has the following format:'[x, y]' (x and y are floating-point numbers in mm)."
			},
			{
				"key": "first_layer_print_min",
				"type": "coFloats",
				"label": "Bottom-left corner of the first layer bounding box",
				"tooltip": ""
			},
			{
				"key": "first_layer_print_max",
				"type": "coFloats",
				"label": "Top-right corner of the first layer bounding box",
				"tooltip": ""
			},
			{
				"key": "first_layer_print_size",
				"type": "coFloats",
				"label": "Size of the first layer bounding box",
				"tooltip": ""
			},
			{
				"key": "print_bed_min",
				"type": "coFloats",
				"label": "Bottom-left corner of print bed bounding box",
				"tooltip": ""
			},
			{
				"key": "print_bed_max",
				"type": "coFloats",
				"label": "Top-right corner of print bed bounding box",
				"tooltip": ""
			},
			{
				"key": "print_bed_size",
				"type": "coFloats",
				"label": "Size of the print bed bounding box",
				"tooltip": ""
			},
			{
				"key": "first_layer_center_no_wipe_tower",
				"type": "coFloats",
				"label": "First layer center without wipe tower",
				"tooltip": ""
			},
			{
				"key": "first_layer_height",
				"type": "coFloat",
				"label": "First layer height",
				"tooltip": "Height of the first layer."
			}
		]
	},
	{
		"title": "Temperatures",
		"items": [
			{
				"key": "bed_temperature",
				"type": "coInts",
				"label": "Bed temperature",
				"tooltip": "Vector of bed temperatures for each extruder/filament."
			},
			{
				"key": "bed_temperature_initial_layer",
				"type": "coInts",
				"label": "First layer bed temperature",
				"tooltip": "Vector of first layer bed temperatures for each extruder/filament. Provides the same value as first_layer_bed_temperature."
			},
			{
				"key": "bed_temperature_initial_layer_single",
				"type": "coInt",
				"label": "First layer bed temperature (initial extruder)",
				"tooltip": "First layer bed temperature for the initial extruder. Same as bed_temperature_initial_layer[initial_extruder]"
			},
			{
				"key": "chamber_temperature",
				"type": "coInts",
				"label": "Chamber temperature",
				"tooltip": "Vector of chamber temperatures for each extruder/filament."
			},
			{
				"key": "chamber_minimal_temperature",
				"type": "coInts",
				"label": "Chamber minimal temperature",
				"tooltip": "Vector of minimal chamber temperatures for each extruder/filament."
			},
			{
				"key": "overall_chamber_temperature",
				"type": "coInt",
				"label": "Overall chamber temperature",
				"tooltip": "Overall chamber temperature. This value is the maximum chamber temperature of any extruder/filament used."
			},
			{
				"key": "first_layer_bed_temperature",
				"type": "coInts",
				"label": "First layer bed temperature",
				"tooltip": "Vector of first layer bed temperatures for each extruder/filament. Provides the same value as bed_temperature_initial_layer."
			},
			{
				"key": "first_layer_temperature",
				"type": "coInts",
				"label": "First layer temperature",
				"tooltip": "Vector of first layer temperatures for each extruder/filament."
			}
		]
	},
	{
		"title": "Timestamps",
		"items": [
			{
				"key": "timestamp",
				"type": "coString",
				"label": "Timestamp",
				"tooltip": "String containing current time in yyyyMMdd-hhmmss format."
			},
			{
				"key": "year",
				"type": "coInt",
				"label": "Year",
				"tooltip": ""
			},
			{
				"key": "month",
				"type": "coInt",
				"label": "Month",
				"tooltip": ""
			},
			{
				"key": "day",
				"type": "coInt",
				"label": "Day",
				"tooltip": ""
			},
			{
				"key": "hour",
				"type": "coInt",
				"label": "Hour",
				"tooltip": ""
			},
			{
				"key": "minute",
				"type": "coInt",
				"label": "Minute",
				"tooltip": ""
			},
			{
				"key": "second",
				"type": "coInt",
				"label": "Second",
				"tooltip": ""
			}
		]
	}
];
export const GCODE_SPECIFIC_DEFS: Record<string, GcodePlaceholder> = {
	"layer_num": {
		"key": "layer_num",
		"type": "coInt",
		"label": "Layer number",
		"tooltip": "Index of the current layer. One-based (i.e. first layer is number 1)."
	},
	"layer_z": {
		"key": "layer_z",
		"type": "coFloat",
		"label": "Layer Z",
		"tooltip": "Height of the current layer above the print bed, measured to the top of the layer."
	},
	"max_layer_z": {
		"key": "max_layer_z",
		"type": "coFloat",
		"label": "Maximal layer Z",
		"tooltip": "Height of the last layer above the print bed."
	},
	"filament_extruder_id": {
		"key": "filament_extruder_id",
		"type": "coInt",
		"label": "Filament extruder ID",
		"tooltip": "The current extruder ID. The same as current_extruder."
	},
	"previous_extruder": {
		"key": "previous_extruder",
		"type": "coInt",
		"label": "Previous extruder",
		"tooltip": "Index of the extruder that is being unloaded. The index is zero based (first extruder has index 0)."
	},
	"next_extruder": {
		"key": "next_extruder",
		"type": "coInt",
		"label": "Next extruder",
		"tooltip": "Index of the extruder that is being loaded. The index is zero based (first extruder has index 0)."
	},
	"relative_e_axis": {
		"key": "relative_e_axis",
		"type": "coBool",
		"label": "Relative e-axis",
		"tooltip": "Indicates if relative positioning is being used."
	},
	"toolchange_count": {
		"key": "toolchange_count",
		"type": "coInt",
		"label": "Toolchange count",
		"tooltip": "The number of toolchanges throught the print."
	},
	"old_retract_length": {
		"key": "old_retract_length",
		"type": "coFloat",
		"label": "Old retract length",
		"tooltip": "The retraction length of the previous filament."
	},
	"new_retract_length": {
		"key": "new_retract_length",
		"type": "coFloat",
		"label": "New retract length",
		"tooltip": "The retraction lenght of the new filament."
	},
	"old_retract_length_toolchange": {
		"key": "old_retract_length_toolchange",
		"type": "coFloat",
		"label": "Old retract length toolchange",
		"tooltip": "The toolchange retraction length of the previous filament."
	},
	"new_retract_length_toolchange": {
		"key": "new_retract_length_toolchange",
		"type": "coFloat",
		"label": "New retract length toolchange",
		"tooltip": "The toolchange retraction length of the new filament."
	},
	"old_filament_temp": {
		"key": "old_filament_temp",
		"type": "coInt",
		"label": "Old filament temp",
		"tooltip": "The old filament temp."
	},
	"new_filament_temp": {
		"key": "new_filament_temp",
		"type": "coInt",
		"label": "New filament temp",
		"tooltip": "The new filament temp."
	},
	"x_after_toolchange": {
		"key": "x_after_toolchange",
		"type": "coFloat",
		"label": "X after toolchange",
		"tooltip": "The X pos after toolchange."
	},
	"y_after_toolchange": {
		"key": "y_after_toolchange",
		"type": "coFloat",
		"label": "Y after toolchange",
		"tooltip": "The Y pos after toolchange."
	},
	"z_after_toolchange": {
		"key": "z_after_toolchange",
		"type": "coFloat",
		"label": "Z after toolchange",
		"tooltip": "The Z pos after toolchange."
	},
	"first_flush_volume": {
		"key": "first_flush_volume",
		"type": "coFloat",
		"label": "First flush volume",
		"tooltip": "The first flush volume."
	},
	"second_flush_volume": {
		"key": "second_flush_volume",
		"type": "coFloat",
		"label": "Second flush volume",
		"tooltip": "The second flush volume."
	},
	"old_filament_e_feedrate": {
		"key": "old_filament_e_feedrate",
		"type": "coInt",
		"label": "Old filament e feedrate",
		"tooltip": "The old filament extruder feedrate."
	},
	"new_filament_e_feedrate": {
		"key": "new_filament_e_feedrate",
		"type": "coInt",
		"label": "New filament e feedrate",
		"tooltip": "The new filament extruder feedrate."
	},
	"travel_point_1_x": {
		"key": "travel_point_1_x",
		"type": "coFloat",
		"label": "Travel point 1 X",
		"tooltip": "The travel point 1 X."
	},
	"travel_point_1_y": {
		"key": "travel_point_1_y",
		"type": "coFloat",
		"label": "Travel point 1 Y",
		"tooltip": "The travel point 1 Y."
	},
	"travel_point_2_x": {
		"key": "travel_point_2_x",
		"type": "coFloat",
		"label": "Travel point 2 X",
		"tooltip": "The travel point 2 X."
	},
	"travel_point_2_y": {
		"key": "travel_point_2_y",
		"type": "coFloat",
		"label": "Travel point 2 Y",
		"tooltip": "The travel point 2 Y."
	},
	"travel_point_3_x": {
		"key": "travel_point_3_x",
		"type": "coFloat",
		"label": "Travel point 3 X",
		"tooltip": "The travel point 3 X."
	},
	"travel_point_3_y": {
		"key": "travel_point_3_y",
		"type": "coFloat",
		"label": "Travel point 3 Y",
		"tooltip": "The travel point 3 Y."
	},
	"flush_length_1": {
		"key": "flush_length_1",
		"type": "coFloat",
		"label": "Flush Length 1",
		"tooltip": "The first flush length."
	},
	"flush_length_2": {
		"key": "flush_length_2",
		"type": "coFloat",
		"label": "Flush Length 2",
		"tooltip": "The second flush length."
	},
	"flush_length_3": {
		"key": "flush_length_3",
		"type": "coFloat",
		"label": "Flush Length 3",
		"tooltip": "The third flush length."
	},
	"flush_length_4": {
		"key": "flush_length_4",
		"type": "coFloat",
		"label": "Flush Length 4",
		"tooltip": "The fourth flush length."
	},
	"extrusion_role": {
		"key": "extrusion_role",
		"type": "coString",
		"label": "Extrusion role",
		"tooltip": "The new extrusion role/type that is going to be used\n"
	},
	"last_extrusion_role": {
		"key": "last_extrusion_role",
		"type": "coString",
		"label": "Last extrusion role",
		"tooltip": "The previously used extrusion role/type\nPossible Values:\n"
	}
};
export const GCODE_SPECIFIC_PLACEHOLDERS: Record<string, string[]> = {
	"file_start_gcode": [],
	"machine_start_gcode": [],
	"machine_end_gcode": [
		"layer_num",
		"layer_z",
		"max_layer_z",
		"filament_extruder_id"
	],
	"before_layer_change_gcode": [
		"layer_num",
		"layer_z",
		"max_layer_z"
	],
	"layer_change_gcode": [
		"layer_num",
		"layer_z",
		"max_layer_z"
	],
	"timelapse_gcode": [
		"layer_num",
		"layer_z",
		"max_layer_z"
	],
	"change_filament_gcode": [
		"layer_num",
		"layer_z",
		"max_layer_z",
		"next_extruder",
		"previous_extruder",
		"first_flush_volume",
		"flush_length_1",
		"flush_length_2",
		"flush_length_3",
		"flush_length_4",
		"new_filament_e_feedrate",
		"new_filament_temp",
		"new_retract_length",
		"new_retract_length_toolchange",
		"old_filament_e_feedrate",
		"old_filament_temp",
		"old_retract_length",
		"old_retract_length_toolchange",
		"relative_e_axis",
		"second_flush_volume",
		"toolchange_count",
		"travel_point_1_x",
		"travel_point_1_y",
		"travel_point_2_x",
		"travel_point_2_y",
		"travel_point_3_x",
		"travel_point_3_y",
		"x_after_toolchange",
		"y_after_toolchange",
		"z_after_toolchange"
	],
	"change_extrusion_role_gcode": [
		"layer_num",
		"layer_z",
		"extrusion_role",
		"last_extrusion_role"
	],
	"filament_change_extrusion_role_gcode": [
		"layer_num",
		"layer_z",
		"extrusion_role",
		"last_extrusion_role"
	],
	"process_change_extrusion_role_gcode": [
		"layer_num",
		"layer_z",
		"extrusion_role",
		"last_extrusion_role"
	],
	"printing_by_object_gcode": [],
	"machine_pause_gcode": [],
	"template_custom_gcode": [],
	"filament_start_gcode": [
		"filament_extruder_id"
	],
	"filament_end_gcode": [
		"layer_num",
		"layer_z",
		"max_layer_z",
		"filament_extruder_id"
	]
};
