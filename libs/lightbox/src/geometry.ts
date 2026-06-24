export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

// 计算图片在视口居中、contain 适配后的目标 rect。
// naturalW/H: 图片真实像素尺寸；vw/vh: 视口尺寸。
export function fitCentered(naturalW: number, naturalH: number, vw: number, vh: number): Rect {
  var maxW = vw * 0.92;
  var maxH = vh * 0.88;
  var scale = Math.min(maxW / naturalW, maxH / naturalH, 1);
  var w = naturalW * scale;
  var h = naturalH * scale;
  return {
    x: (vw - w) / 2,
    y: (vh - h) / 2,
    w: w,
    h: h,
  };
}

// 把目标 rect 转成 transform 字符串。
// baseW/baseH 是 img 元素的布局尺寸（=居中目标尺寸），scale 相对它缩放。
// transform-origin 为 top left（见 CSS），translate 到 rect 左上角后 scale。
// - 居中态：scale=1（base 就是居中尺寸）
// - originRect 态：scale = originRect.w / base.w（缩小）
export function transformFor(rect: Rect, baseW: number, baseH: number): string {
  var sx = baseW > 0 ? rect.w / baseW : 1;
  var sy = baseH > 0 ? rect.h / baseH : 1;
  return (
    "translate(" +
    rect.x +
    "px," +
    rect.y +
    "px) scale(" +
    sx +
    "," +
    sy +
    ")"
  );
}

// 原图 URL = data-src 去 query。data-src 形如 "/uploads/x.webp?w=800"。
export function originalUrl(dataSrc: string | null): string {
  return (dataSrc || "").split("?")[0];
}
