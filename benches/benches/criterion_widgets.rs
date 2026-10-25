// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use criterion::{Criterion, criterion_group, criterion_main};
use lunatui_bench::widget_suite::{CASES, LunatuiRun, RatatuiRun, SIZES};

fn widgets(c: &mut Criterion) {
    for case in CASES {
        for (width, height) in SIZES {
            let mut group = c.benchmark_group(format!("widget_{}_{width}x{height}", case.name));
            let mut lunatui = LunatuiRun::new(case, width, height);
            group.bench_function("render/lunatui", |b| b.iter(|| lunatui.render()));
            let mut lunatui = LunatuiRun::new(case, width, height);
            group.bench_function("frame/lunatui", |b| {
                b.iter(|| {
                    let _ = lunatui.frame();
                });
            });
            if let Ok(mut ratatui) = RatatuiRun::new(case, width, height) {
                group.bench_function("render/ratatui", |b| b.iter(|| ratatui.render()));
            }
            if let Ok(mut ratatui) = RatatuiRun::new(case, width, height) {
                group.bench_function("frame/ratatui", |b| {
                    b.iter(|| {
                        let _ = ratatui.frame();
                    });
                });
            }
            group.finish();
        }
    }
}

criterion_group!(benches, widgets);
criterion_main!(benches);
