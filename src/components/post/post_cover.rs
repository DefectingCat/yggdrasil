//! 文章封面组件
//!
//! 在文章详情页渲染封面大图，使用图片查看器支持点击放大。

use dioxus::prelude::*;

use crate::components::image_viewer::ImageViewer;

/// 文章封面组件。
///
/// Props：
/// - `src`：封面原图 URL
///
/// 使用 1200px 宽度的缩略图作为默认封面展示，点击可放大查看。
#[component]
pub fn PostCover(src: String) -> Element {
    rsx! {
        figure { class: "entry-cover",
            ImageViewer {
                src: src.clone(),
                thumb_params: "?w=1200",
                alt: "封面图片".to_string(),
                lazy_load: false,
            }
        }
    }
}
