//! 렌더링 경로의 자원 회귀 가드.
//!
//! "리소스를 더 쓰게 된다면 보류" 기준의 기계적 게이트: 태그 라이터 도입(2026-07) 전
//! 측정한 기준선을 상한으로 할당 횟수·동시 상주(peak) 메모리를 검증한다.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAllocator;

static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        let live = LIVE_BYTES.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
        PEAK_LIVE_BYTES.fetch_max(live, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn render_resources_do_not_exceed_baseline() {
    use namumark_backend_namuwiki::NamuwikiMarkup;
    use namumark_ir::RenderBackend;
    use namumark_render::EmptyContext;

    // (픽스처, 태그 라이터 도입 전 렌더 중 할당 횟수 기준선)
    const BASELINES: [(&str, usize); 3] = [
        ("타카하시_미나미", 264),
        ("iPhone_6s_Plus", 323),
        ("정암사", 94),
    ];

    for (name, maximum_allocation_count) in BASELINES {
        let path = format!("{}/../../fixtures/{name}.namu", env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(&path).expect("픽스처");
        let document = namumark_parser::parse(&source);
        let tree = namumark_render::build_render_tree(&document, &EmptyContext);

        let count_before = ALLOCATION_COUNT.load(Ordering::Relaxed);
        let live_before = LIVE_BYTES.load(Ordering::Relaxed);
        PEAK_LIVE_BYTES.store(live_before, Ordering::Relaxed);
        let markup = NamuwikiMarkup.render(&tree);
        let count = ALLOCATION_COUNT.load(Ordering::Relaxed) - count_before;
        let peak = PEAK_LIVE_BYTES.load(Ordering::Relaxed) - live_before;

        println!(
            "{name}: 마크업 {} bytes | 렌더 중 할당 {count}회, peak {peak} bytes",
            markup.len(),
        );
        assert!(
            count <= maximum_allocation_count,
            "{name}: 렌더 중 할당 {count}회 > 기준선 {maximum_allocation_count}회"
        );
        // peak 상한: 출력 버퍼의 배증 재할당 순간(구용량+신용량 ≤ 출력 3배) + 소액 여유.
        // 두 번째 전체 크기 버퍼·트리가 생기면 이 상한을 넘게 되어 회귀가 잡힌다.
        let peak_ceiling = markup.len() * 3 + 4096;
        assert!(
            peak <= peak_ceiling,
            "{name}: peak {peak} bytes > 상한 {peak_ceiling} bytes"
        );
    }
}
