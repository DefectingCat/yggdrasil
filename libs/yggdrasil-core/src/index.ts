import { initPostContent } from './post-content';
import { applyResolvedTheme, startThemeTransition } from './theme-transition';
import './style.css';

declare global {
  interface Window {
    __initPostContent: (selector: string) => void;
    __startThemeTransition: (x: number, y: number) => void;
    __applyResolvedTheme: (isDark: boolean) => void;
  }
}

window.__initPostContent = initPostContent;
window.__startThemeTransition = startThemeTransition;
window.__applyResolvedTheme = applyResolvedTheme;
