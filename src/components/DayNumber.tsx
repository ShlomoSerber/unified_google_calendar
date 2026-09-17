import type { ReactNode } from 'react';
import './DayNumber.css';

// The one circular day button of the app (docs/11 section 7): the mini calendar and the date
// picker (`mini`), the year cards (`year`), the week and agenda headers (`day`) and the month
// cells (`month`, a pill that grows for "Sep 1"). Today is primary on on-primary, a selected
// day is primary-container, a muted day (another month) is on-surface-variant.

export type DayNumberSize = 'day' | 'mini' | 'year' | 'month';

export interface DayNumberProps {
  size: DayNumberSize;
  /** Typescale class of the label, e.g. `md-typescale-label-medium`. */
  typescale: string;
  label: string;
  today?: boolean;
  selected?: boolean;
  muted?: boolean;
  className?: string;
  onClick: () => void;
  children: ReactNode;
}

export function DayNumber({ size, typescale, label, today, selected, muted, className, onClick, children }: DayNumberProps) {
  const tone = today ? ' day-number-today' : selected ? ' day-number-selected' : muted ? ' day-number-muted' : '';
  return (
    <button className={`day-number ${typescale}${tone}${className ? ` ${className}` : ''}`} data-size={size} type="button" aria-label={label} onClick={onClick}>
      <md-ripple></md-ripple>
      <md-focus-ring></md-focus-ring>
      {children}
    </button>
  );
}
