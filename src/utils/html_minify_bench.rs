//! 临时 bench：测量 minify_html 对真实大小 HTML 的耗时。
//! 用完即删。

#[cfg(all(test, feature = "server"))]
mod bench {
    use std::time::Instant;

    fn synthetic_html(size_hint: usize) -> String {
        // 构造一个带空白、注释、嵌套标签的模拟 HTML，体积接近真实 SSR 输出。
        let unit = "<div class=\"x\">\n  <p>hello world</p>\n  <!-- comment -->\n  <a href=\"/p\">link</a>\n</div>\n";
        let unit_len = unit.len();
        let repeats = size_hint / unit_len + 1;
        unit.repeat(repeats)
    }

    #[test]
    fn bench_minify_throughput() {
        for &kb in &[50usize, 200, 568] {
            let html = synthetic_html(kb * 1024);
            // 预热
            let _ = crate::utils::html_minify::minify_html(&html);

            let n = 20;
            let start = Instant::now();
            for _ in 0..n {
                let _ = crate::utils::html_minify::minify_html(&html);
            }
            let elapsed = start.elapsed() / n;
            let input_kb = html.len() as f64 / 1024.0;
            let mbps = (html.len() as f64 / elapsed.as_secs_f64()) / (1024.0 * 1024.0);
            println!(
                "input={:.0}KB avg minify={:?} (~{:.0} MB/s)",
                input_kb,
                elapsed,
                mbps
            );
        }
    }
}
