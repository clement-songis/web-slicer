// Calibration du débit (menu Calibration › Flow rate, passes 1 et 2, YOLO) :
// choix du dialogue (FlowRateCalibrationDialog d'Orca). Le projet est construit
// par le serveur (Plater::calib_flowrate porté ; cf. server.ts).

export type FlowPass = 1 | 2;
export type FlowPattern = 'archimedeanchords' | 'monotonic';
