#[cfg(test)]
thread_local! {
    static INCREMENTAL_PARSE_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_PARSE_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
struct IncrementalParseTracker;

#[cfg(test)]
impl IncrementalParseTracker {
    fn new() -> Self {
        INCREMENTAL_PARSE_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_PARSE_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_PARSE_COUNT.with(|count| count.set(0));
    }

    fn count(&self) -> usize {
        INCREMENTAL_PARSE_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalParseTracker {
    fn drop(&mut self) {
        INCREMENTAL_PARSE_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_PARSE_COUNT.with(|count| count.set(0));
    }
}

#[cfg(test)]
thread_local! {
    static INCREMENTAL_TYPE_INFER_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_TYPE_INFER_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static INCREMENTAL_SCC_INFER_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_SCC_INFER_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_SCC_MERGED_FAST_PATH_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
struct IncrementalTypeInferTracker;

#[cfg(test)]
impl IncrementalTypeInferTracker {
    fn new() -> Self {
        INCREMENTAL_TYPE_INFER_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_TYPE_INFER_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_TYPE_INFER_COUNT.with(|count| count.set(0));
    }

    fn count(&self) -> usize {
        INCREMENTAL_TYPE_INFER_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalTypeInferTracker {
    fn drop(&mut self) {
        INCREMENTAL_TYPE_INFER_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_TYPE_INFER_COUNT.with(|count| count.set(0));
    }
}

#[cfg(test)]
struct IncrementalSccInferTracker;

#[cfg(test)]
impl IncrementalSccInferTracker {
    fn new() -> Self {
        INCREMENTAL_SCC_INFER_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_SCC_INFER_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_SCC_INFER_COUNT.with(|count| count.set(0));
    }

    fn count(&self) -> usize {
        INCREMENTAL_SCC_INFER_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalSccInferTracker {
    fn drop(&mut self) {
        INCREMENTAL_SCC_INFER_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_SCC_INFER_COUNT.with(|count| count.set(0));
    }
}

#[cfg(test)]
struct IncrementalSccMergedFastPathTracker;

#[cfg(test)]
impl IncrementalSccMergedFastPathTracker {
    fn new() -> Self {
        INCREMENTAL_SCC_MERGED_FAST_PATH_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT.with(|count| count.set(0));
    }

    fn count(&self) -> usize {
        INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalSccMergedFastPathTracker {
    fn drop(&mut self) {
        INCREMENTAL_SCC_MERGED_FAST_PATH_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT.with(|count| count.set(0));
    }
}

#[cfg(test)]
thread_local! {
    static INCREMENTAL_LOWER_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_LOWER_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
struct IncrementalLowerTracker;

#[cfg(test)]
impl IncrementalLowerTracker {
    fn new() -> Self {
        INCREMENTAL_LOWER_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_LOWER_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_LOWER_COUNT.with(|count| count.set(0));
    }

    fn count(&self) -> usize {
        INCREMENTAL_LOWER_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalLowerTracker {
    fn drop(&mut self) {
        INCREMENTAL_LOWER_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_LOWER_COUNT.with(|count| count.set(0));
    }
}

#[cfg(test)]
thread_local! {
    static INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_MODULE_SEGMENT_LOWER_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
struct IncrementalModuleSegmentLowerTracker;

#[cfg(test)]
impl IncrementalModuleSegmentLowerTracker {
    fn new() -> Self {
        INCREMENTAL_MODULE_SEGMENT_LOWER_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT.with(|count| count.set(0));
    }

    fn count(&self) -> usize {
        INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalModuleSegmentLowerTracker {
    fn drop(&mut self) {
        INCREMENTAL_MODULE_SEGMENT_LOWER_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT.with(|count| count.set(0));
    }
}

#[cfg(test)]
thread_local! {
    static INCREMENTAL_LINK_FULL_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_LINK_FULL_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static INCREMENTAL_LINK_CACHE_HIT_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INCREMENTAL_LINK_CACHE_HIT_TRACKING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
struct IncrementalLinkTracker;

#[cfg(test)]
impl IncrementalLinkTracker {
    fn new() -> Self {
        INCREMENTAL_LINK_FULL_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_LINK_CACHE_HIT_TRACKING_ENABLED.with(|enabled| enabled.set(true));
        INCREMENTAL_LINK_FULL_COUNT.with(|count| count.set(0));
        INCREMENTAL_LINK_CACHE_HIT_COUNT.with(|count| count.set(0));
        Self
    }

    fn reset(&self) {
        INCREMENTAL_LINK_FULL_COUNT.with(|count| count.set(0));
        INCREMENTAL_LINK_CACHE_HIT_COUNT.with(|count| count.set(0));
    }

    fn full_count(&self) -> usize {
        INCREMENTAL_LINK_FULL_COUNT.with(|count| count.get())
    }

    fn cache_hit_count(&self) -> usize {
        INCREMENTAL_LINK_CACHE_HIT_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for IncrementalLinkTracker {
    fn drop(&mut self) {
        INCREMENTAL_LINK_FULL_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_LINK_CACHE_HIT_TRACKING_ENABLED.with(|enabled| enabled.set(false));
        INCREMENTAL_LINK_FULL_COUNT.with(|count| count.set(0));
        INCREMENTAL_LINK_CACHE_HIT_COUNT.with(|count| count.set(0));
    }
}
