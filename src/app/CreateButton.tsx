import { useUi } from '../state/ui';
import { CreateMenu } from './CreateMenu';
import './CreateButton.css';

// Component 3 of docs/04 section 4. Google places this button outside the drawer, absolutely
// positioned on the page (docs/design/measurements/create_button-light.json rootRect), so the
// shell renders it next to the sidebar, not inside it. Classes mirror the dump nodes.
export function CreateButton() {
  const dialog = useUi((s) => s.dialog);
  const open = dialog.kind === 'create-menu';
  const toggle = () => {
    const ui = useUi.getState();
    if (open) ui.closeDialog();
    else ui.openDialog({ kind: 'create-menu' });
  };
  return (
    <div className="create-button-anchor">
      <button className="create-button-root" type="button" aria-haspopup="menu" aria-expanded={open} onClick={toggle}>
        <span className="create-button-ripple"></span>
        <div className="create-button-box">
          <div className="create-button-label">
            <i className="create-button-icon">add</i>
            {'Create'}
            <i className="create-button-arrow">arrow_drop_down</i>
          </div>
        </div>
      </button>
      {open ? <CreateMenu /> : null}
    </div>
  );
}
