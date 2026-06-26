import { initPostContent } from './post-content';

declare global {
  interface Window {
    __initPostContent: (selector: string) => void;
  }
}

window.__initPostContent = initPostContent;

export {};
