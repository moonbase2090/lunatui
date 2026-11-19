// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_runtime::{Change, ChangeContext, ChangeSource};

pub(crate) fn guarded<S: Clone, T>(
    state: &mut S,
    mut context: ChangeContext<'_>,
    operation: impl FnOnce(&mut S, ChangeSource) -> Change<T>,
) -> Change<T> {
    let source = context.source();
    if context.is_guarded() {
        let mut next = state.clone();
        let proposed = operation(&mut next, source);
        context.commit(proposed, || *state = next)
    } else {
        operation(state, source)
    }
}
