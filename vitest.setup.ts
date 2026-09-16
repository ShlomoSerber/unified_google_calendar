// jsdom has no ElementInternals, so the @material/web registrations are replaced by nothing:
// every <md-*> renders as an inert element without shadow DOM and tests query the app's own
// roles and labels only (docs/11-material3.md section 5).
import { vi } from 'vitest';

vi.mock('./src/m3/register', () => ({}));
