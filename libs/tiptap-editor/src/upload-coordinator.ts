import type { Editor } from '@tiptap/core'

/** pending 上传条目：保留 File 供重试，blobUrl 供本地预览，state 跟踪当前态。 */
interface UploadEntry {
  file: File
  blobUrl: string
  fileName: string
  /** 当前态：uploading（进行中）或 error（失败待重试）。成功后整个 entry 删除。 */
  state: 'uploading' | 'error'
}

/** counts 快照：随事件一起传给宿主，替代 window 全局。 */
export interface UploadCounts {
  uploading: number
  error: number
}

/** coordinator 推给宿主的事件（通过 emit 回调，替代 window 全局轮询）。 */
export interface UploadEvent {
  kind: 'error' | 'success' | 'removed'
  uploadId: string
  fileName: string
  errorMsg?: string
  ts: number
  /** 当前编辑器内 uploading/error 占位符计数（emit 时由 JS 遍历文档算出）。 */
  counts: UploadCounts
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
 * - 通过 emit 回调把事件推给宿主（替代旧版 window 全局轮询）
 *
 * pending Map 保留 File 对象直到上传成功或显式移除，支持无限重试。
 */
export class UploadCoordinator {
  private pending = new Map<string, UploadEntry>()
  /** 内部计数：替代每次事件全量遍历文档。 */
  private uploadingCount = 0
  private errorCount = 0

  constructor(
    private editor: Editor,
    private onImageUpload: (file: File) => Promise<string>,
    private emit: (event: UploadEvent) => void,
  ) {}

  /**
   * 插入上传中占位符。
   * pos 省略时插入当前选区。
   * 注意：此版本只插入节点，不发起上传（runUpload 在后续任务加入）。
   */
  insertUploading(file: File, pos?: number): void {
    const uploadId = genUploadId()
    const blobUrl = URL.createObjectURL(file)
    this.pending.set(uploadId, { file, blobUrl, fileName: file.name, state: 'uploading' })
    this.uploadingCount++

    this.editor.chain().focus().insertContentAt(pos ?? this.editor.state.selection.head, {
      type: 'image',
      attrs: {
        src: blobUrl,
        'data-upload-state': 'uploading',
        'data-upload-id': uploadId,
      },
    }).run()

    this.runUpload(uploadId)
  }

  /** 按 uploadId 删除节点（revoke blob、清 pending）。NodeView 移除按钮 / Rust ×关闭 共用。 */
  removeUpload(uploadId: string): boolean {
    const entry = this.pending.get(uploadId)
    if (!entry) return false
    this.removeNodeByUploadId(uploadId)
    URL.revokeObjectURL(entry.blobUrl)
    // 按当前态减对应计数
    if (entry.state === 'uploading') this.uploadingCount--
    else this.errorCount--
    this.pending.delete(uploadId)
    this.notifyRust({ kind: 'removed', uploadId, fileName: entry.fileName })
    return true
  }

  /**
   * NodeView.destroy 兜底：节点被 ProseMirror 删除（如退格）时调用。
   * 与 removeUpload 区别：节点已被 PM 删，这里只清 pending + revoke + 减计数，
   * 不再调 removeNodeByUploadId，也不 emit 'removed'（节点已不在文档，counts 即时反映）。
   *
   * 成功态的 entry 不在 pending 里（已 delete），此方法对它们是 no-op。
   */
  handleNodeDestroyed(uploadId: string): void {
    const entry = this.pending.get(uploadId)
    if (!entry) return
    URL.revokeObjectURL(entry.blobUrl)
    if (entry.state === 'uploading') this.uploadingCount--
    else this.errorCount--
    this.pending.delete(uploadId)
    // emit removed 让宿主顶部提示同步移除该 id
    this.notifyRust({ kind: 'removed', uploadId, fileName: entry.fileName })
  }

  /** 核心上传逻辑：成功更新 src + 清上传属性，失败转 error 态。 */
  private async runUpload(uploadId: string): Promise<void> {
    const entry = this.pending.get(uploadId)
    if (!entry) return
    try {
      const url = await this.onImageUpload(entry.file)
      // 成功：替换 src，清除上传状态属性
      this.updateNodeAttrs(uploadId, {
        src: url,
        'data-upload-state': null,
        'data-upload-id': null,
        'data-error-msg': null,
      })
      URL.revokeObjectURL(entry.blobUrl)
      this.pending.delete(uploadId)
      this.uploadingCount--
      this.notifyRust({ kind: 'success', uploadId, fileName: entry.fileName })
    } catch (err) {
      const msg = this.extractErrorMessage(err)
      this.updateNodeAttrs(uploadId, {
        'data-upload-state': 'error',
        'data-error-msg': msg,
      })
      // 失败：从 uploading 转 error 态
      entry.state = 'error'
      this.uploadingCount--
      this.errorCount++
      this.notifyRust({ kind: 'error', uploadId, fileName: entry.fileName, errorMsg: msg })
    }
  }

  /** 重试：从 pending 取回原 File，节点转回 uploading，重跑上传。 */
  retryUpload(uploadId: string): void {
    const entry = this.pending.get(uploadId)
    if (!entry) return
    // 重试前节点处于 error 态，转回 uploading
    this.errorCount--
    this.uploadingCount++
    this.updateNodeAttrs(uploadId, {
      'data-upload-state': 'uploading',
      'data-error-msg': null,
    })
    this.runUpload(uploadId)
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

  /** 从错误对象提取消息。改造后的 fetch 直接抛服务端中文（如"文件超过大小限制"）。 */
  private extractErrorMessage(err: unknown): string {
    if (err instanceof Error) return err.message
    return String(err)
  }

  /**
   * 通过注入的 emit 回调把事件（含 counts 快照）输出给宿主，
   * counts 直接读内部维护的计数，不再每次遍历文档。
   */
  private notifyRust(event: Omit<UploadEvent, 'ts' | 'counts'>): void {
    this.emit({
      ...event,
      ts: Date.now(),
      counts: { uploading: this.uploadingCount, error: this.errorCount },
    })
  }

  /** pending Map 查询（供内部/测试）。 */
  hasPending(uploadId: string): boolean {
    return this.pending.has(uploadId)
  }
}
