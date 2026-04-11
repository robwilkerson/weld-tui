/// Manages scroll position and dimensions for the visible content area.
#[derive(Debug, Default)]
pub struct Viewport {
    /// Vertical scroll offset (0 = top of content).
    pub scroll_y: u16,
    /// Horizontal scroll offset (0 = left of content).
    pub scroll_x: u16,
    /// Visible rows in the content area.
    pub height: u16,
    /// Visible columns in the content area.
    pub width: u16,
}

impl Viewport {
    /// Maximum vertical scroll offset that keeps content bottom at or below viewport bottom.
    pub fn scroll_y_max(&self, total_rows: usize) -> u16 {
        let max_y = total_rows.saturating_sub(1) as u16;
        max_y.saturating_sub(self.height.saturating_sub(1))
    }

    /// Clamp scroll position to valid bounds after a resize or content change.
    pub fn clamp(&mut self, total_rows: usize, max_content_width: u16) {
        self.scroll_y = self.scroll_y.min(self.scroll_y_max(total_rows));
        self.scroll_x = self
            .scroll_x
            .min(max_content_width.saturating_sub(self.width));
    }

    /// Scroll down by one line, clamped.
    pub fn scroll_down(&mut self, total_rows: usize) {
        self.scroll_y = self
            .scroll_y
            .saturating_add(1)
            .min(self.scroll_y_max(total_rows));
    }

    /// Scroll up by one line, clamped.
    pub fn scroll_up(&mut self) {
        self.scroll_y = self.scroll_y.saturating_sub(1);
    }

    /// Jump to top.
    pub fn scroll_to_top(&mut self) {
        self.scroll_y = 0;
    }

    /// Jump to bottom, clamped.
    pub fn scroll_to_bottom(&mut self, total_rows: usize) {
        self.scroll_y = self.scroll_y_max(total_rows);
    }

    /// Scroll right by `cols` columns, clamped.
    pub fn scroll_right(&mut self, cols: u16, max_content_width: u16) {
        self.scroll_x = self
            .scroll_x
            .saturating_add(cols)
            .min(max_content_width.saturating_sub(self.width));
    }

    /// Scroll left by `cols` columns, clamped.
    pub fn scroll_left(&mut self, cols: u16) {
        self.scroll_x = self.scroll_x.saturating_sub(cols);
    }

    /// Jump to left edge.
    pub fn scroll_to_left(&mut self) {
        self.scroll_x = 0;
    }

    /// Jump to right edge, clamped.
    pub fn scroll_to_right(&mut self, max_content_width: u16) {
        self.scroll_x = max_content_width.saturating_sub(self.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vp(height: u16, width: u16) -> Viewport {
        Viewport {
            scroll_y: 0,
            scroll_x: 0,
            height,
            width,
        }
    }

    #[test]
    fn scroll_y_max_basic() {
        let v = vp(10, 40);
        assert_eq!(v.scroll_y_max(20), 10);
    }

    #[test]
    fn scroll_y_max_content_fits() {
        let v = vp(10, 40);
        assert_eq!(v.scroll_y_max(5), 0);
    }

    #[test]
    fn scroll_down_clamps() {
        let mut v = vp(10, 40);
        for _ in 0..25 {
            v.scroll_down(20);
        }
        assert_eq!(v.scroll_y, 10);
    }

    #[test]
    fn scroll_up_clamps_at_zero() {
        let mut v = vp(10, 40);
        v.scroll_up();
        assert_eq!(v.scroll_y, 0);
    }

    #[test]
    fn scroll_to_bottom_matches_repeated_down() {
        let mut v1 = vp(20, 40);
        v1.scroll_to_bottom(50);

        let mut v2 = vp(20, 40);
        for _ in 0..100 {
            v2.scroll_down(50);
        }

        assert_eq!(v1.scroll_y, v2.scroll_y);
    }

    #[test]
    fn scroll_right_clamps() {
        let mut v = vp(10, 40);
        v.scroll_right(200, 100);
        assert_eq!(v.scroll_x, 60);
    }

    #[test]
    fn scroll_to_right_matches_repeated_right() {
        let mut v1 = vp(10, 40);
        v1.scroll_to_right(100);

        let mut v2 = vp(10, 40);
        for _ in 0..200 {
            v2.scroll_right(2, 100);
        }

        assert_eq!(v1.scroll_x, v2.scroll_x);
    }

    #[test]
    fn clamp_after_resize_shrink() {
        let mut v = vp(20, 40);
        v.scroll_y = 30;
        v.scroll_x = 50;
        // Shrink viewport — scroll_y 30 is still valid (max = 49 - 9 = 40)
        v.height = 10;
        v.width = 20;
        v.clamp(50, 100);
        assert_eq!(v.scroll_y, 30);
        assert_eq!(v.scroll_x, 50);
    }

    #[test]
    fn clamp_pulls_back_overscrolled() {
        let mut v = vp(10, 40);
        v.scroll_y = 45;
        v.scroll_x = 90;
        // 50 rows, height 10 → max scroll_y = 40; width 40, content 100 → max scroll_x = 60
        v.clamp(50, 100);
        assert_eq!(v.scroll_y, 40);
        assert_eq!(v.scroll_x, 60);
    }

    #[test]
    fn clamp_after_resize_grow() {
        let mut v = vp(10, 40);
        v.scroll_y = 10;
        v.scroll_x = 30;
        // Grow viewport to fit all content
        v.height = 50;
        v.width = 100;
        v.clamp(20, 80);
        assert_eq!(v.scroll_y, 0); // all content fits
        assert_eq!(v.scroll_x, 0); // all content fits
    }
}
