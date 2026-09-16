// JSX typings for the @material/web elements registered in src/m3/register.ts
// (docs/11-material3.md section 5). Each tag is typed with its element class: HTML attributes,
// `ref` to the class, the element's own properties (React 19 assigns them as properties) and
// the element's events as lowercase `on<event>` listeners. Hyphenated attributes
// (`touch-target`, `supporting-text`, `trailing-icon`, ...) need no declaration in JSX.
import type { MdFilledButton } from '@material/web/button/filled-button.js';
import type { MdFilledTonalButton } from '@material/web/button/filled-tonal-button.js';
import type { MdOutlinedButton } from '@material/web/button/outlined-button.js';
import type { MdTextButton } from '@material/web/button/text-button.js';
import type { MdCheckbox } from '@material/web/checkbox/checkbox.js';
import type { MdAssistChip } from '@material/web/chips/assist-chip.js';
import type { MdChipSet } from '@material/web/chips/chip-set.js';
import type { MdFilterChip } from '@material/web/chips/filter-chip.js';
import type { MdDialog } from '@material/web/dialog/dialog.js';
import type { MdDivider } from '@material/web/divider/divider.js';
import type { MdFab } from '@material/web/fab/fab.js';
import type { MdFocusRing } from '@material/web/focus/md-focus-ring.js';
import type { MdIcon } from '@material/web/icon/icon.js';
import type { MdIconButton } from '@material/web/iconbutton/icon-button.js';
import type { MdList } from '@material/web/list/list.js';
import type { MdListItem } from '@material/web/list/list-item.js';
import type { MdMenu } from '@material/web/menu/menu.js';
import type { MdMenuItem } from '@material/web/menu/menu-item.js';
import type { MdCircularProgress } from '@material/web/progress/circular-progress.js';
import type { MdRadio } from '@material/web/radio/radio.js';
import type { MdRipple } from '@material/web/ripple/ripple.js';
import type { MdOutlinedSelect } from '@material/web/select/outlined-select.js';
import type { MdSelectOption } from '@material/web/select/select-option.js';
import type { MdSwitch } from '@material/web/switch/switch.js';
import type { MdOutlinedTextField } from '@material/web/textfield/outlined-text-field.js';

type M3<T extends HTMLElement, E extends string = never> = React.DetailedHTMLProps<React.HTMLAttributes<T>, T> &
  Partial<Omit<T, keyof HTMLElement | 'children'>> & { slot?: string } & { [K in E as `on${K}`]?: (e: Event) => void };

type Clickable = 'click';
type Field = 'change' | 'input';

declare module 'react' {
  namespace JSX {
    interface IntrinsicElements {
      'md-filled-button': M3<MdFilledButton, Clickable>;
      'md-filled-tonal-button': M3<MdFilledTonalButton, Clickable>;
      'md-outlined-button': M3<MdOutlinedButton, Clickable>;
      'md-text-button': M3<MdTextButton, Clickable>;
      'md-checkbox': M3<MdCheckbox, Field>;
      'md-assist-chip': M3<MdAssistChip, Clickable>;
      'md-chip-set': M3<MdChipSet>;
      'md-filter-chip': M3<MdFilterChip, Clickable | 'change'>;
      'md-dialog': M3<MdDialog, 'open' | 'opened' | 'close' | 'closed' | 'cancel'>;
      'md-divider': M3<MdDivider>;
      'md-fab': M3<MdFab, Clickable>;
      'md-focus-ring': M3<MdFocusRing>;
      'md-icon': M3<MdIcon>;
      'md-icon-button': M3<MdIconButton, Clickable>;
      'md-list': M3<MdList>;
      'md-list-item': M3<MdListItem, Clickable>;
      'md-menu': M3<MdMenu, 'opening' | 'opened' | 'closing' | 'closed'>;
      'md-menu-item': M3<MdMenuItem, Clickable | 'close-menu'>;
      'md-circular-progress': M3<MdCircularProgress>;
      'md-radio': M3<MdRadio, Field>;
      'md-ripple': M3<MdRipple>;
      'md-outlined-select': M3<MdOutlinedSelect, Field | 'opening' | 'opened' | 'closing' | 'closed'>;
      'md-select-option': M3<MdSelectOption, Clickable>;
      'md-switch': M3<MdSwitch, Field>;
      'md-outlined-text-field': M3<MdOutlinedTextField, Field | 'select' | 'keydown'>;
    }
  }
}
