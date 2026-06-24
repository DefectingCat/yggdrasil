import { describe, it, expect } from 'vitest'
import { isValidUrl } from '../slash-command'

/**
 * isValidUrl 纯函数测试。
 *
 * 只允许 http(s):// 和 data:image/ 开头，拒绝 javascript: 等危险或非图片 scheme。
 */

describe('isValidUrl', () => {
  describe('接受', () => {
    it.each([
      ['http://example.com', 'http 协议'],
      ['https://example.com', 'https 协议'],
      ['https://example.com/path?q=1#frag', '带路径/查询/片段'],
      ['HTTP://EXAMPLE.COM', 'http 大写（大小写不敏感）'],
      ['Https://Example.Com', '混合大小写'],
      ['http://localhost:3000/img.png', 'localhost + 端口'],
      ['data:image/png;base64,iVBORw0KGgo=', 'data URL png'],
      ['data:image/jpeg;base64,/9j/4AAQ', 'data URL jpeg'],
      ['data:image/svg+xml,<svg/>', 'data URL svg'],
      ['data:image/', 'data:image 前缀（最小匹配）'],
    ])('%s (%s)', (url) => {
      expect(isValidUrl(url)).toBe(true)
    })
  })

  describe('拒绝', () => {
    it.each([
      ['javascript:alert(1)', 'javascript scheme（XSS 风险）'],
      ['JavaScript:alert(1)', 'javascript 大写'],
      ['ftp://example.com/file', 'ftp scheme'],
      ['file:///etc/passwd', 'file scheme'],
      ['mailto:foo@bar.com', 'mailto scheme'],
      ['data:text/html,<script>', '非 image 的 data URL'],
      ['data:application/octet-stream,', '非 image 的 data URL'],
      ['//example.com/img.png', '协议相对 URL（无 scheme）'],
      ['/absolute/path/img.png', '绝对路径'],
      ['relative/path/img.png', '相对路径'],
      ['./img.png', './ 相对'],
      ['example.com/img.png', '无 scheme 的域名'],
      ['httpfoo://x', 'http 前缀但非 http scheme'],
      [' https://example.com', '前导空格（^ 锚定，不匹配）'],
      ['', '空字符串'],
    ])('%s (%s)', (url) => {
      expect(isValidUrl(url)).toBe(false)
    })
  })

  describe('正则边界（scheme 前缀校验，不锚定结尾）', () => {
    it('https:// 后含末尾空格仍被接受（只校验前缀）', () => {
      // 有意为之：isValidUrl 只校验 scheme 前缀，URL 其余部分由浏览器/服务端校验
      expect(isValidUrl('https://example.com ')).toBe(true)
    })
  })
})
