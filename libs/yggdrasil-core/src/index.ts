declare global {
  interface Window {
    __initPostContent: (selector: string) => void;
  }
}

window.__initPostContent = (_selector: string): void => {
  // Task 2 实现
};

export {};
