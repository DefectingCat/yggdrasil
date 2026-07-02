import { initPostContent } from './post-content';
import { startThemeTransition } from './theme-transition';
import './style.css';

declare global {
  interface Window {
    __initPostContent: (selector: string) => void;
    __startThemeTransition: (x: number, y: number) => void;
  }
}

window.__initPostContent = initPostContent;
window.__startThemeTransition = startThemeTransition;
