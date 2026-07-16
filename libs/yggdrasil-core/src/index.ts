import { initAnchorClick } from './anchor-click';
import { scrollToHash } from './hash-scroll';
import { initMermaid } from './mermaid';
import { initPostContent } from './post-content';
import { applyResolvedTheme, startThemeTransition } from './theme-transition';
import type { ThemeName } from '@yggdrasil/shared';
import './style.css';

declare global {
  interface Window {
    __initPostContent: (selector: string) => void;
    __initMermaid: (selector: string, theme: ThemeName) => void;
    __initAnchorClick: () => void;
    __scrollToHash: () => void;
    __startThemeTransition: (x: number, y: number) => void;
    __applyResolvedTheme: (isDark: boolean) => void;
  }
}

window.__initPostContent = initPostContent;
window.__initMermaid = initMermaid;
window.__initAnchorClick = initAnchorClick;
window.__scrollToHash = scrollToHash;
window.__startThemeTransition = startThemeTransition;
window.__applyResolvedTheme = applyResolvedTheme;
