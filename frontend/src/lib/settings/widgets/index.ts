// Bibliothèque de widgets de réglage (T041) : résolution type → contrôle et
// composants d'édition, consommés par `OptionLine` (T042).

export { resolveWidgetKind, isGcodeParam, type WidgetKind } from './resolve';
export { WIDGETS, widgetFor, type WidgetComponent } from './registry';
export { FIELD_CLASS, type WidgetProps } from './types';
