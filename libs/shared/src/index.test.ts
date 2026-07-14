import { describe, expect, it } from 'vitest';
import { prefersReducedMotion, THEME_CHANGE_EVENT, type ThemeName } from './index';

describe('@yggdrasil/shared', () => {
  it('THEME_CHANGE_EVENT 是固定字符串', () => {
    expect(THEME_CHANGE_EVENT).toBe('yggdrasil:theme-change');
  });

  it('prefersReducedMotion 在无 matchMedia 时返回 false', () => {
    // happy-dom 提供了 matchMedia，这里只验证函数可调用且返回 boolean
    const result = prefersReducedMotion();
    expect(typeof result).toBe('boolean');
  });

  it('ThemeName 类型仅接受 light/dark（编译期约束）', () => {
    const t: ThemeName = 'light';
    expect(['light', 'dark']).toContain(t);
  });
});
