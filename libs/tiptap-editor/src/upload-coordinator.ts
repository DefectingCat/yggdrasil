import type { Editor } from '@tiptap/core'

/** pending 上传条目：保留 File 供重试，blobUrl 供本地预览。 */
interface UploadEntry {
  file: File
  blobUrl: string
  fileName: string
}

/** coordinator 推给 Rust 的事件（通过 window.__tiptap_uploads.events）。 */
export interface UploadEvent {
  kind: 'error' | 'success' | 'removed'
  uploadId: string
  fileName: string
  errorMsg?: string
  ts: number
}

/** 生成 uploadId，兼容非安全上下文（无 crypto.randomUUID 时）。 */
function genUploadId(): string {
  return crypto.randomUUID?.() ?? Math.random().toString(36).slice(2)
}

/**
 * 上传协调器：集中管理图片上传生命周期。
 *
 * 职责：
 * - 生成 uploadId、创建 blob URL、插入占位符节点
 * - 发起上传，成功更新节点 src、失败转 error 态
 * - 按 upload-id 定位节点更新/删除（上传完成时光标早已移走）
 * - 维护 window.__tiptap_uploads 供 Rust 轮询
 *
 * pending Map 保留 File 对象直到上传成功或显式移除，支持无限重试。
 */
export class UploadCoordinator {
  private pending = new Map<string, UploadEntry>()

  constructor(
    private editor: Editor,
    private onImageUpload: (file: File) => Promise<string>,
  ) {}

  /**
   * 插入上传中占位符。
   * pos 省略时插入当前选区。
   * 注意：此版本只插入节点，不发起上传（runUpload 在后续任务加入）。
   */
  insertUploading(file: File, pos?: number): void {
    const uploadId = genUploadId()
    const blobUrl = URL.createObjectURL(file)
    this.pending.set(uploadId, { file, blobUrl, fileName: file.name })

    this.editor.chain().focus().insertContentAt(pos ?? this.editor.state.selection.head, {
      type: 'image',
      attrs: {
        src: blobUrl,
        'data-upload-state': 'uploading',
        'data-upload-id': uploadId,
      },
    }).run()
  }

  /** 按 uploadId 删除节点（revoke blob、清 pending）。NodeView 移除按钮 / Rust ×关闭 共用。 */
  removeUpload(uploadId: string): boolean {
    const entry = this.pending.get(uploadId)
    if (!entry) return false
    this.removeNodeByUploadId(uploadId)
    URL.revokeObjectURL(entry.blobUrl)
    this.pending.delete(uploadId)
    return true
  }

  /** 按 uploadId 在文档中定位节点并删除。 */
  private removeNodeByUploadId(uploadId: string): void {
    let targetPos: number | null = null
    let nodeSize = 0
    this.editor.state.doc.descendants((node, pos) => {
      if (node.type.name === 'image' && node.attrs['data-upload-id'] === uploadId) {
        targetPos = pos
        nodeSize = node.nodeSize
        return false
      }
      return true
    })
    if (targetPos !== null) {
      const tr = this.editor.state.tr.delete(targetPos, targetPos + nodeSize)
      this.editor.view.dispatch(tr)
    }
  }

  /** 按 uploadId 定位节点并合并更新属性。 */
  private updateNodeAttrs(uploadId: string, attrs: Record<string, unknown>): void {
    let targetPos: number | null = null
    let oldAttrs: Record<string, unknown> | null = null
    this.editor.state.doc.descendants((node, pos) => {
      if (node.type.name === 'image' && node.attrs['data-upload-id'] === uploadId) {
        targetPos = pos
        oldAttrs = node.attrs
        return false
      }
      return true
    })
    if (targetPos !== null && oldAttrs) {
      const tr = this.editor.state.tr.setNodeMarkup(
        targetPos,
        undefined,
        { ...oldAttrs, ...attrs },
      )
      this.editor.view.dispatch(tr)
    }
  }

  /** pending Map 查询（供内部/测试）。 */
  hasPending(uploadId: string): boolean {
    return this.pending.has(uploadId)
  }
}
