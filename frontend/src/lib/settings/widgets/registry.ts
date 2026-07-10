// Table widget → composant Svelte (T041). Séparée de `resolve.ts` (pur) pour
// que la résolution reste testable sans compiler de composant.

import type { Component } from 'svelte';

import type { ParamDef } from '../../../generated/params';
import BoolWidget from './BoolWidget.svelte';
import ColorWidget from './ColorWidget.svelte';
import EnumWidget from './EnumWidget.svelte';
import FloatOrPercentWidget from './FloatOrPercentWidget.svelte';
import FloatWidget from './FloatWidget.svelte';
import IntWidget from './IntWidget.svelte';
import MultilineWidget from './MultilineWidget.svelte';
import PercentWidget from './PercentWidget.svelte';
import PointWidget from './PointWidget.svelte';
import { resolveWidgetKind, type WidgetKind } from './resolve';
import StringWidget from './StringWidget.svelte';
import StringsWidget from './StringsWidget.svelte';
import type { WidgetProps } from './types';

/** Composant d'édition générique (props communes, valeur de type variable). */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WidgetComponent = Component<WidgetProps<any>>;

/** Composant associé à chaque `WidgetKind`. */
export const WIDGETS: Record<WidgetKind, WidgetComponent> = {
	bool: BoolWidget,
	int: IntWidget,
	float: FloatWidget,
	percent: PercentWidget,
	floatOrPercent: FloatOrPercentWidget,
	enum: EnumWidget,
	string: StringWidget,
	strings: StringsWidget,
	multiline: MultilineWidget,
	point: PointWidget,
	color: ColorWidget
};

/** Composant d'édition d'un paramètre, résolu par son type de registre. */
export function widgetFor(def: ParamDef): WidgetComponent {
	return WIDGETS[resolveWidgetKind(def)];
}
