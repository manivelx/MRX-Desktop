//! Pure window-layout calculations shared by the X11 window manager and tests.
//! Coordinates are root-window coordinates; dimensions are always non-zero.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// ICCCM WM_NORMAL_HINTS constraints used when fitting a client into a tile.
/// Aspect constraints are kept for a later pass; these fields cover the common
/// minimum, maximum, base-size and resize-increment hints.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SizeHints {
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub base_width: Option<u32>,
    pub base_height: Option<u32>,
    pub width_inc: Option<u32>,
    pub height_inc: Option<u32>,
}

/// Constrain a tiled rectangle to the client's ICCCM size limits. When a
/// client requests a smaller size than its tile, center it in the available
/// rectangle; minimum sizes may exceed the tile when the screen is too small.
pub fn constrain_size(rect: Rect, hints: &SizeHints) -> Rect {
    fn dimension(value: u32, min: Option<u32>, max: Option<u32>, base: Option<u32>, inc: Option<u32>) -> u32 {
        let minimum = min.unwrap_or(1).max(1);
        let maximum = max.filter(|v| *v >= minimum).unwrap_or(u32::MAX);
        let mut result = value.clamp(minimum, maximum);
        if let Some(step) = inc.filter(|step| *step > 0) {
            let origin = base.or(min).unwrap_or(0);
            if result >= origin {
                result = origin.saturating_add(((result - origin) / step) * step);
            }
            result = result.clamp(minimum, maximum);
        }
        result.max(1)
    }

    let width = dimension(rect.width, hints.min_width, hints.max_width, hints.base_width, hints.width_inc);
    let height = dimension(rect.height, hints.min_height, hints.max_height, hints.base_height, hints.height_inc);
    let x = rect.x.saturating_add(((rect.width as i64 - width as i64) / 2) as i32);
    let y = rect.y.saturating_add(((rect.height as i64 - height as i64) / 2) as i32);
    Rect { x, y, width, height }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    MasterStack,
    Columns,
    Monocle,
}

impl LayoutKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "master-stack" => Some(Self::MasterStack),
            "columns" => Some(Self::Columns),
            "monocle" => Some(Self::Monocle),
            _ => None,
        }
    }
}

/// Compute deterministic client rectangles inside a monitor work area.
/// Floating and fullscreen windows must be filtered by the caller.
pub fn arrange(
    area: Rect,
    count: usize,
    layout: LayoutKind,
    master_ratio: f32,
    gap: u32,
) -> Vec<Rect> {
    if count == 0 || area.width == 0 || area.height == 0 {
        return Vec::new();
    }

    let g = gap.min(area.width.saturating_sub(1) / 2).min(area.height.saturating_sub(1) / 2);
    let inner_x = area.x.saturating_add(g as i32);
    let inner_y = area.y.saturating_add(g as i32);
    let inner_w = area.width.saturating_sub(g.saturating_mul(2)).max(1);
    let inner_h = area.height.saturating_sub(g.saturating_mul(2)).max(1);

    if layout == LayoutKind::Monocle || count == 1 {
        return vec![Rect { x: inner_x, y: inner_y, width: inner_w, height: inner_h }; count];
    }

    match layout {
        LayoutKind::MasterStack => {
            let ratio = if master_ratio.is_finite() { master_ratio.clamp(0.2, 0.8) } else { 0.58 };
            let column_gap = g.min(inner_w.saturating_sub(1));
            let usable = inner_w.saturating_sub(column_gap).max(1);
            let master_w = ((usable as f32) * ratio).round() as u32;
            let master_w = master_w.clamp(1, usable.saturating_sub(1).max(1));
            let stack_w = usable.saturating_sub(master_w).max(1);
            let mut out = Vec::with_capacity(count);
            out.push(Rect { x: inner_x, y: inner_y, width: master_w, height: inner_h });
            let n = (count - 1) as u32;
            let total_gaps = g.saturating_mul(n.saturating_sub(1));
            let cell_h = inner_h.saturating_sub(total_gaps).checked_div(n).unwrap_or(1).max(1);
            for i in 0..n {
                let y = inner_y.saturating_add((i.saturating_mul(cell_h.saturating_add(g))) as i32);
                let height = if i == n - 1 { (inner_y as i64 + inner_h as i64 - y as i64).max(1) as u32 } else { cell_h };
                out.push(Rect { x: inner_x.saturating_add((master_w + column_gap) as i32), y, width: stack_w, height });
            }
            out
        }
        LayoutKind::Columns => {
            let n = count as u32;
            let total_gaps = g.saturating_mul(n.saturating_sub(1));
            let cell_w = inner_w.saturating_sub(total_gaps).checked_div(n).unwrap_or(1).max(1);
            (0..n).map(|i| {
                let x = inner_x.saturating_add((i.saturating_mul(cell_w.saturating_add(g))) as i32);
                let width = if i == n - 1 { (inner_x as i64 + inner_w as i64 - x as i64).max(1) as u32 } else { cell_w };
                Rect { x, y: inner_y, width, height: inner_h }
            }).collect()
        }
        LayoutKind::Monocle => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn area() -> Rect { Rect { x: 0, y: 0, width: 1920, height: 1080 } }

    #[test]
    fn empty_workspace_has_no_rectangles() {
        assert!(arrange(area(), 0, LayoutKind::MasterStack, 0.58, 8).is_empty());
    }

    #[test]
    fn monocle_gives_every_client_same_inset_area() {
        let rects = arrange(area(), 3, LayoutKind::Monocle, 0.58, 10);
        assert_eq!(rects.len(), 3);
        assert!(rects.iter().all(|r| *r == Rect { x: 10, y: 10, width: 1900, height: 1060 }));
    }

    #[test]
    fn master_stack_fills_reasonable_non_overlapping_columns() {
        let rects = arrange(area(), 3, LayoutKind::MasterStack, 0.6, 8);
        assert_eq!(rects.len(), 3);
        assert!(rects.iter().all(|r| r.width > 0 && r.height > 0));
        assert_eq!(rects[0].x, 8);
        assert!(rects[1].x > rects[0].x);
        assert!(rects[2].y > rects[1].y);
    }

    #[test]
    fn columns_distribute_clients_across_width() {
        let rects = arrange(area(), 4, LayoutKind::Columns, 0.58, 8);
        assert_eq!(rects.len(), 4);
        assert!(rects.windows(2).all(|w| w[1].x > w[0].x));
        assert!(rects.iter().all(|r| r.height == 1064));
    }

    #[test]
    fn pathological_inputs_never_create_zero_dimensions() {
        let tiny = Rect { x: -10, y: -10, width: 1, height: 1 };
        let rects = arrange(tiny, 8, LayoutKind::MasterStack, f32::NAN, u32::MAX);
        assert!(rects.iter().all(|r| r.width >= 1 && r.height >= 1));
    }

    #[test]
    fn size_hints_enforce_minimum_and_maximum_dimensions() {
        let rect = Rect { x: 0, y: 0, width: 900, height: 700 };
        let hints = SizeHints {
            min_width: Some(320), min_height: Some(240),
            max_width: Some(800), max_height: Some(600), ..Default::default()
        };
        assert_eq!(constrain_size(rect, &hints), Rect { x: 50, y: 50, width: 800, height: 600 });
    }

    #[test]
    fn resize_increments_are_applied_from_base_size() {
        let rect = Rect { x: 0, y: 0, width: 503, height: 407 };
        let hints = SizeHints {
            base_width: Some(100), base_height: Some(80),
            width_inc: Some(16), height_inc: Some(12), ..Default::default()
        };
        assert_eq!(constrain_size(rect, &hints), Rect { x: 1, y: 1, width: 500, height: 404 });
    }

    #[test]
    fn size_hints_never_return_zero_for_pathological_values() {
        let rect = Rect { x: -4, y: 7, width: 0, height: 0 };
        let hints = SizeHints { min_width: Some(0), min_height: Some(0), width_inc: Some(0), height_inc: Some(0), ..Default::default() };
        let result = constrain_size(rect, &hints);
        assert!(result.width >= 1 && result.height >= 1);
    }
}
