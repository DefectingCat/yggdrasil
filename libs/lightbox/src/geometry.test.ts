// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { fitCentered, originalUrl, type Rect, transformFor } from './geometry';

describe('fitCentered', () => {
  // 大图受 maxW=vw*0.92 / maxH=vh*0.88 约束,scale 取较小者
  it('宽图被视口宽度卡住(横向受限)', () => {
    // 2000x1000 in 1000x800: maxW=920, maxH=704
    // scale = min(920/2000, 704/1000, 1) = min(0.46, 0.704, 1) = 0.46
    const r = fitCentered(2000, 1000, 1000, 800);
    expect(r.w).toBeCloseTo(920, 5);
    expect(r.h).toBeCloseTo(460, 5);
    expect(r.x).toBeCloseTo((1000 - 920) / 2, 5); // 40
    expect(r.y).toBeCloseTo((800 - 460) / 2, 5); // 170
  });

  it('高图被视口高度卡住(纵向受限)', () => {
    // 1000x2000 in 1000x800: maxW=920, maxH=704
    // scale = min(920/1000, 704/2000, 1) = min(0.92, 0.352, 1) = 0.352
    const r = fitCentered(1000, 2000, 1000, 800);
    expect(r.w).toBeCloseTo(352, 5);
    expect(r.h).toBeCloseTo(704, 5);
  });

  it('小图不被放大(scale 钳到 1)', () => {
    // 300x200 in 1000x800: scale = min(920/300, 704/200, 1) = 1
    const r = fitCentered(300, 200, 1000, 800);
    expect(r.w).toBe(300);
    expect(r.h).toBe(200);
    expect(r.x).toBeCloseTo((1000 - 300) / 2, 5); // 350
    expect(r.y).toBeCloseTo((800 - 200) / 2, 5); // 300
  });

  it('正方形图按 maxW/maxH 中较小者缩放', () => {
    // 1000x1000 in 1000x800: maxW=920, maxH=704
    // scale = min(0.92, 0.704, 1) = 0.704
    const r = fitCentered(1000, 1000, 1000, 800);
    expect(r.w).toBeCloseTo(704, 5);
    expect(r.h).toBeCloseTo(704, 5);
  });
});

describe('transformFor', () => {
  it('居中态(base=target):scale=1,translate 到目标左上角', () => {
    const target: Rect = { x: 40, y: 170, w: 920, h: 460 };
    const t = transformFor(target, 920, 460);
    expect(t).toBe('translate(40px,170px) scale(1,1)');
  });

  it('缩小态(rect 比 base 小):scale<1', () => {
    // 原图位置小,base=居中尺寸 920x460
    const origin: Rect = { x: 100, y: 500, w: 400, h: 200 };
    const t = transformFor(origin, 920, 460);
    // 400/920 = 0.43478260869565216（JS 浮点完整精度）
    expect(t).toBe('translate(100px,500px) scale(0.43478260869565216,0.43478260869565216)');
  });

  it('baseW=0 守卫:scale 守卫为 1(不产生 NaN/Infinity)', () => {
    const rect: Rect = { x: 0, y: 0, w: 0, h: 0 };
    const t = transformFor(rect, 0, 0);
    expect(t).toBe('translate(0px,0px) scale(1,1)');
  });

  it('字符串格式为 translate(Xpx,Ypx) scale(SX,SY)', () => {
    const rect: Rect = { x: 10, y: 20, w: 100, h: 50 };
    const t = transformFor(rect, 200, 100);
    expect(t).toMatch(
      /^translate\(\d+(\.\d+)?px,\d+(\.\d+)?px\) scale\(\d+(\.\d+)?,\d+(\.\d+)?\)$/,
    );
  });
});

describe('originalUrl', () => {
  it('去 query string', () => {
    expect(originalUrl('/uploads/x.webp?w=800')).toBe('/uploads/x.webp');
  });

  it('无 query 原样返回', () => {
    expect(originalUrl('/uploads/x.webp')).toBe('/uploads/x.webp');
  });

  it('null 输入返回空串', () => {
    expect(originalUrl(null)).toBe('');
  });

  it('空串输入返回空串', () => {
    expect(originalUrl('')).toBe('');
  });
});
