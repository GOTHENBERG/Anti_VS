use macroquad::prelude::Vec2;

pub const CELL_SIZE: f32 = 128.0;

/// Магический множитель K* (результат эмпирического поиска на 15 000 кандидатов).
/// Дисперсия корзин: Var = 0.259 (строго 3-5 ячеек на бакет, 0 пустых бакетов).
/// 0 коллизий в окрестностях вплоть до 15x15 ячеек на всей карте 25920x17280!
pub const K_STAR: u64 = 0xd805d526169aa741;
pub const SPATIAL_TABLE_BITS: u32 = 13;
pub const SPATIAL_TABLE_SIZE: usize = 1 << SPATIAL_TABLE_BITS; // 8192
/// Вмещает плотную сетку 207x139 = 28 773 ячейки в стековой битовой маске без кучных аллокаций.
const MAX_STACK_MASK_WORDS: usize = 512; // 512 * 64 = 32 768 битов

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialGridBackend {
    /// Базовая хешированная связанная сетка (8192 бакета через K_STAR).
    HashedLinked,
    /// Прямая плотная связанная сетка (cy * cols + cx без хеш-коллизий).
    DenseLinked,
    /// Прямая плотная CSR-сетка с непрерывными диапазонами индексов [offsets[c]..offsets[c+1]].
    DenseCsr,
}

pub const DEFAULT_SPATIAL_BACKEND: SpatialGridBackend = SpatialGridBackend::DenseCsr;

/// Стековая битовая маска посещённых ячеек/бакетов для обходов с перекрытием шагов.
pub struct SpatialVisitedMask {
    stack_bits: [u64; MAX_STACK_MASK_WORDS],
    heap_bits: Vec<u64>,
    word_count: usize,
}

impl SpatialVisitedMask {
    #[inline]
    fn new(total_slots: usize) -> Self {
        let word_count = total_slots.div_ceil(64).max(1);
        let heap_bits = if word_count > MAX_STACK_MASK_WORDS {
            vec![0u64; word_count]
        } else {
            Vec::new()
        };
        Self {
            stack_bits: [0u64; MAX_STACK_MASK_WORDS],
            heap_bits,
            word_count,
        }
    }

    #[inline(always)]
    pub fn test_and_set(&mut self, slot: usize) -> bool {
        let word = slot >> 6;
        let bit = 1u64 << (slot & 63);
        let words = if self.heap_bits.is_empty() {
            &mut self.stack_bits[..self.word_count]
        } else {
            &mut self.heap_bits[..self.word_count]
        };
        if (words[word] & bit) != 0 {
            return false;
        }
        words[word] |= bit;
        true
    }
}

/// Вычисление индекса бакета за 2 такта процессора: imul + shr (без деления %)
#[inline(always)]
pub fn bucket_idx(cx: usize, cy: usize) -> usize {
    let key: u64 = ((cx as u64) << 32) | (cy as u64);
    ((key.wrapping_mul(K_STAR)) >> (64 - SPATIAL_TABLE_BITS)) as usize
}

/// Пространственный индекс мобов с поддержкой 3 бэкендов (HashedLinked, DenseLinked, DenseCsr)
/// для сравнительного бенчмаркинга на полном тике и нулевых кучных аллокаций в кадре.
#[derive(Debug, Clone)]
pub struct SpatialGrid {
    backend: SpatialGridBackend,
    cell_size: f32,
    inv_cell_size: f32,
    offset: f32,
    cols: usize,
    rows: usize,
    dense_len: usize,
    /// Головы списков (8192 для HashedLinked, cols*rows для DenseLinked).
    heads: Vec<i32>,
    entities: Vec<usize>,
    next: Vec<i32>,
    active_buckets: Vec<usize>,
    raw_active_coords: Vec<(usize, usize)>,
    /// Прямые плотные активные ячейки (дедуплицированы при вставке/prefix-sum).
    active_dense_cells: Vec<u32>,
    /// Поля бэкенда DenseCsr.
    csr_counts: Vec<u32>,
    csr_offsets: Vec<u32>,
    csr_cursors: Vec<u32>,
    csr_cell_cache: Vec<u32>,
    csr_pending: Vec<(u32, usize)>,
}

impl SpatialGrid {
    pub fn new(width: f32, height: f32) -> Self {
        Self::new_with_backend(width, height, DEFAULT_SPATIAL_BACKEND)
    }

    pub fn new_with_backend(width: f32, height: f32, backend: SpatialGridBackend) -> Self {
        let cols = ((width / CELL_SIZE).ceil() as usize) + 4;
        let rows = ((height / CELL_SIZE).ceil() as usize) + 4;
        let dense_len = cols * rows;

        let heads_len = match backend {
            SpatialGridBackend::HashedLinked => SPATIAL_TABLE_SIZE,
            SpatialGridBackend::DenseLinked => dense_len,
            SpatialGridBackend::DenseCsr => 0,
        };
        let csr_len = if backend == SpatialGridBackend::DenseCsr { dense_len } else { 0 };
        let csr_off_len = if backend == SpatialGridBackend::DenseCsr { dense_len + 1 } else { 0 };

        Self {
            backend,
            cell_size: CELL_SIZE,
            inv_cell_size: 1.0 / CELL_SIZE,
            offset: CELL_SIZE * 2.0,
            cols,
            rows,
            dense_len,
            heads: vec![-1; heads_len],
            entities: Vec::with_capacity(32768),
            next: Vec::with_capacity(match backend {
                SpatialGridBackend::DenseCsr => 0,
                _ => 32768,
            }),
            active_buckets: Vec::with_capacity(1024),
            raw_active_coords: Vec::with_capacity(if backend == SpatialGridBackend::HashedLinked { 1024 } else { 0 }),
            active_dense_cells: Vec::with_capacity(if backend != SpatialGridBackend::HashedLinked { 2048 } else { 0 }),
            csr_counts: vec![0; csr_len],
            csr_offsets: vec![0; csr_off_len],
            csr_cursors: vec![0; csr_len],
            csr_cell_cache: Vec::with_capacity(if backend == SpatialGridBackend::DenseCsr { 32768 } else { 0 }),
            csr_pending: Vec::new(),
        }
    }

    #[inline]
    pub fn backend(&self) -> SpatialGridBackend {
        self.backend
    }

    /// Меняет бэкенд переиндексации (используется для A/B замеров на полном тике).
    pub fn set_backend(&mut self, backend: SpatialGridBackend) {
        if self.backend == backend {
            return;
        }
        let w = (self.cols.saturating_sub(4)) as f32 * self.cell_size;
        let h = (self.rows.saturating_sub(4)) as f32 * self.cell_size;
        *self = Self::new_with_backend(w, h, backend);
    }

    #[inline]
    pub fn cols(&self) -> usize {
        self.cols
    }

    #[inline]
    pub fn rows(&self) -> usize {
        self.rows
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    #[inline]
    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    #[inline]
    pub fn active_cells(&self) -> &[usize] {
        &self.active_buckets
    }

    #[inline]
    pub fn new_visited_mask(&self) -> SpatialVisitedMask {
        let total = match self.backend {
            SpatialGridBackend::HashedLinked => SPATIAL_TABLE_SIZE,
            SpatialGridBackend::DenseLinked | SpatialGridBackend::DenseCsr => self.dense_len,
        };
        SpatialVisitedMask::new(total)
    }

    pub fn active_coords(&self) -> Vec<(usize, usize)> {
        match self.backend {
            SpatialGridBackend::HashedLinked => {
                let mut coords = self.raw_active_coords.clone();
                coords.sort_unstable();
                coords.dedup();
                coords
            }
            SpatialGridBackend::DenseLinked | SpatialGridBackend::DenseCsr => self
                .active_dense_cells
                .iter()
                .map(|&c| {
                    let ci = c as usize;
                    (ci % self.cols, ci / self.cols)
                })
                .collect(),
        }
    }

    /// Мгновенная очистка активных корзин/ячеек без сброса ёмкости буферов.
    pub fn clear(&mut self) {
        match self.backend {
            SpatialGridBackend::HashedLinked => {
                for &b in &self.active_buckets {
                    if b < self.heads.len() {
                        self.heads[b] = -1;
                    }
                }
                self.active_buckets.clear();
                self.next.clear();
                self.raw_active_coords.clear();
            }
            SpatialGridBackend::DenseLinked => {
                for &c in &self.active_dense_cells {
                    let ci = c as usize;
                    if ci < self.heads.len() {
                        self.heads[ci] = -1;
                    }
                }
                self.active_dense_cells.clear();
                self.next.clear();
            }
            SpatialGridBackend::DenseCsr => {
                for &c in &self.active_dense_cells {
                    let ci = c as usize;
                    if ci < self.csr_counts.len() {
                        self.csr_counts[ci] = 0;
                    }
                }
                self.active_dense_cells.clear();
                self.csr_cell_cache.clear();
                self.csr_pending.clear();
            }
        }
        self.entities.clear();
    }

    #[inline(always)]
    pub fn get_cell_coords(&self, pos: Vec2) -> Option<(usize, usize)> {
        let ox = pos.x + self.offset;
        let oy = pos.y + self.offset;
        if ox < 0.0 || oy < 0.0 {
            return None;
        }
        let cx = (ox * self.inv_cell_size) as usize;
        let cy = (oy * self.inv_cell_size) as usize;
        if cx < self.cols && cy < self.rows {
            Some((cx, cy))
        } else {
            None
        }
    }

    #[inline(always)]
    fn query_cell_bounds(&self, min_pos: Vec2, max_pos: Vec2) -> Option<(usize, usize, usize, usize)> {
        if self.entities.is_empty() || self.cols == 0 || self.rows == 0 {
            return None;
        }
        let f_min_x = (min_pos.x + self.offset) * self.inv_cell_size;
        let f_max_x = (max_pos.x + self.offset) * self.inv_cell_size;
        let f_min_y = (min_pos.y + self.offset) * self.inv_cell_size;
        let f_max_y = (max_pos.y + self.offset) * self.inv_cell_size;

        if f_max_x < 0.0 || f_max_y < 0.0 || f_min_x >= self.cols as f32 || f_min_y >= self.rows as f32 {
            return None;
        }
        let min_cx = (f_min_x.floor().max(0.0) as usize).min(self.cols - 1);
        let max_cx = (f_max_x.floor().max(0.0) as usize).min(self.cols - 1);
        let min_cy = (f_min_y.floor().max(0.0) as usize).min(self.rows - 1);
        let max_cy = (f_max_y.floor().max(0.0) as usize).min(self.rows - 1);
        Some((min_cx, max_cx, min_cy, max_cy))
    }

    /// Перестраивает весь индекс напрямую из среза сущностей без повторных проверок границ.
    pub fn rebuild_from_slice<T>(&mut self, items: &[T], mut pos_of: impl FnMut(&T) -> Vec2) {
        self.clear();
        if items.is_empty() {
            return;
        }
        match self.backend {
            SpatialGridBackend::HashedLinked | SpatialGridBackend::DenseLinked => {
                self.entities.reserve(items.len().saturating_sub(self.entities.capacity()));
                self.next.reserve(items.len().saturating_sub(self.next.capacity()));
                for (idx, item) in items.iter().enumerate() {
                    self.insert_linked(idx, pos_of(item));
                }
            }
            SpatialGridBackend::DenseCsr => {
                self.csr_cell_cache.reserve(items.len().saturating_sub(self.csr_cell_cache.capacity()));
                let cols = self.cols;
                let rows = self.rows;
                let offset = self.offset;
                let inv_h = self.inv_cell_size;

                for item in items {
                    let pos = pos_of(item);
                    let ox = pos.x + offset;
                    let oy = pos.y + offset;
                    if ox >= 0.0 && oy >= 0.0 {
                        let cx = (ox * inv_h) as usize;
                        let cy = (oy * inv_h) as usize;
                        if cx < cols && cy < rows {
                            let ci = cy * cols + cx;
                            self.csr_counts[ci] += 1;
                            self.csr_cell_cache.push(ci as u32);
                            continue;
                        }
                    }
                    self.csr_cell_cache.push(u32::MAX);
                }

                let mut total = 0u32;
                for ci in 0..self.dense_len {
                    self.csr_offsets[ci] = total;
                    let cnt = self.csr_counts[ci];
                    if cnt > 0 {
                        self.csr_cursors[ci] = total;
                        self.active_dense_cells.push(ci as u32);
                        total += cnt;
                    }
                }
                self.csr_offsets[self.dense_len] = total;
                self.entities.resize(total as usize, 0);

                let csr_cursors = &mut self.csr_cursors;
                let entities = &mut self.entities;
                for (idx, &cell) in self.csr_cell_cache.iter().enumerate() {
                    if cell != u32::MAX {
                        let ci = cell as usize;
                        let dst = csr_cursors[ci] as usize;
                        csr_cursors[ci] = (dst + 1) as u32;
                        entities[dst] = idx;
                    }
                }
            }
        }
    }

    #[inline(always)]
    fn cursors_advance(&mut self, ci: usize) -> usize {
        let dst = self.csr_cursors[ci] as usize;
        self.csr_cursors[ci] = (dst + 1) as u32;
        dst
    }

    /// Прямой пересчёт из готового слайса позиций (например `world.mob_positions_scratch`).
    pub fn rebuild_from_positions(&mut self, positions: &[Vec2]) {
        self.rebuild_from_slice(positions, |&p| p);
    }

    #[inline(always)]
    fn insert_linked(&mut self, entity_idx: usize, pos: Vec2) {
        let Some((cx, cy)) = self.get_cell_coords(pos) else {
            return;
        };
        match self.backend {
            SpatialGridBackend::HashedLinked => {
                let b_idx = bucket_idx(cx, cy);
                let head = self.heads[b_idx];
                if head == -1 {
                    self.active_buckets.push(b_idx);
                }
                let curr_id = self.entities.len() as i32;
                self.entities.push(entity_idx);
                self.next.push(head);
                self.heads[b_idx] = curr_id;
                self.raw_active_coords.push((cx, cy));
            }
            SpatialGridBackend::DenseLinked => {
                let ci = cy * self.cols + cx;
                let head = self.heads[ci];
                if head == -1 {
                    self.active_dense_cells.push(ci as u32);
                }
                let curr_id = self.entities.len() as i32;
                self.entities.push(entity_idx);
                self.next.push(head);
                self.heads[ci] = curr_id;
            }
            SpatialGridBackend::DenseCsr => {}
        }
    }

    /// Вставка одной сущности (для инкрементальных тестов; при DenseCsr завершается `finish()`).
    pub fn insert(&mut self, entity_idx: usize, pos: Vec2) {
        if self.backend != SpatialGridBackend::DenseCsr {
            self.insert_linked(entity_idx, pos);
            return;
        }
        if let Some((cx, cy)) = self.get_cell_coords(pos) {
            let ci = (cy * self.cols + cx) as u32;
            self.csr_counts[ci as usize] += 1;
            self.csr_pending.push((ci, entity_idx));
            let pending = std::mem::take(&mut self.csr_pending);
            self.rebuild_csr_from_pending(pending);
        }
    }

    fn rebuild_csr_from_pending(&mut self, mut pending: Vec<(u32, usize)>) {
        // Если уже были элементы, пересобираем из текущего CSR + pending (используется только в тестах).
        if !self.entities.is_empty() {
            for &ci in &self.active_dense_cells {
                let s = self.csr_offsets[ci as usize] as usize;
                let e = self.csr_offsets[ci as usize + 1] as usize;
                for &ent in &self.entities[s..e] {
                    pending.push((ci, ent));
                }
            }
        }
        self.active_dense_cells.clear();
        self.csr_counts.fill(0);
        for &(ci, _) in &pending {
            self.csr_counts[ci as usize] += 1;
        }
        let mut total = 0u32;
        for ci in 0..self.dense_len {
            self.csr_offsets[ci] = total;
            self.csr_cursors[ci] = total;
            let cnt = self.csr_counts[ci];
            if cnt > 0 {
                self.active_dense_cells.push(ci as u32);
                total += cnt;
            }
        }
        self.csr_offsets[self.dense_len] = total;
        self.entities.resize(total as usize, 0);
        for (ci, ent) in pending {
            let dst = self.cursors_advance(ci as usize);
            self.entities[dst] = ent;
        }
    }

    /// Завершает инкрементальную сборку (no-op для Linked и batch-CSR).
    pub fn finish(&mut self) {
        // Все режимы уже готовы к запросам.
    }

    /// Обход всех сущностей в конкретной ячейке (cx, cy).
    #[inline(always)]
    pub fn for_each_in_cell<F>(&self, cx: usize, cy: usize, mut f: F)
    where
        F: FnMut(usize) -> bool,
    {
        if self.entities.is_empty() || cx >= self.cols || cy >= self.rows {
            return;
        }
        match self.backend {
            SpatialGridBackend::HashedLinked => {
                let b_idx = bucket_idx(cx, cy);
                let mut curr = self.heads[b_idx];
                while curr != -1 {
                    let e = self.entities[curr as usize];
                    if !f(e) {
                        break;
                    }
                    curr = self.next[curr as usize];
                }
            }
            SpatialGridBackend::DenseLinked => {
                let ci = cy * self.cols + cx;
                let mut curr = self.heads[ci];
                while curr != -1 {
                    let e = self.entities[curr as usize];
                    if !f(e) {
                        break;
                    }
                    curr = self.next[curr as usize];
                }
            }
            SpatialGridBackend::DenseCsr => {
                let ci = cy * self.cols + cx;
                let start = self.csr_offsets[ci] as usize;
                let end = self.csr_offsets[ci + 1] as usize;
                for &e in &self.entities[start..end] {
                    if !f(e) {
                        break;
                    }
                }
            }
        }
    }

    #[inline(always)]
    fn visit_cell_unchecked<F>(&self, cx: usize, cy: usize, f: &mut F)
    where
        F: FnMut(usize),
    {
        match self.backend {
            SpatialGridBackend::HashedLinked => {
                let b_idx = bucket_idx(cx, cy);
                let mut curr = self.heads[b_idx];
                while curr != -1 {
                    f(self.entities[curr as usize]);
                    curr = self.next[curr as usize];
                }
            }
            SpatialGridBackend::DenseLinked => {
                let ci = cy * self.cols + cx;
                let mut curr = self.heads[ci];
                while curr != -1 {
                    f(self.entities[curr as usize]);
                    curr = self.next[curr as usize];
                }
            }
            SpatialGridBackend::DenseCsr => {
                let ci = cy * self.cols + cx;
                let start = self.csr_offsets[ci] as usize;
                let end = self.csr_offsets[ci + 1] as usize;
                for &e in &self.entities[start..end] {
                    f(e);
                }
            }
        }
    }

    /// Посещает ячейку с проверкой маски дубликатов (работает одинаково для Hash и Dense).
    #[inline(always)]
    pub fn visit_cell_with_mask<F>(
        &self,
        cx: usize,
        cy: usize,
        mask: &mut SpatialVisitedMask,
        f: &mut F,
    ) where
        F: FnMut(usize),
    {
        if self.entities.is_empty() || cx >= self.cols || cy >= self.rows {
            return;
        }
        let slot = match self.backend {
            SpatialGridBackend::HashedLinked => bucket_idx(cx, cy),
            SpatialGridBackend::DenseLinked | SpatialGridBackend::DenseCsr => cy * self.cols + cx,
        };
        if !mask.test_and_set(slot) {
            return;
        }
        self.visit_cell_unchecked(cx, cy, f);
    }

    /// Посещает каждую корзину (bucket) не более одного раза за счёт стековой битовой маски.
    #[inline(always)]
    pub fn visit_cell_unique<F>(
        &self,
        cx: usize,
        cy: usize,
        visited_buckets: &mut [u64; SPATIAL_TABLE_SIZE / 64],
        f: &mut F,
    ) where
        F: FnMut(usize),
    {
        if cx >= self.cols || cy >= self.rows {
            return;
        }
        let b_idx = bucket_idx(cx, cy);
        let word = b_idx >> 6;
        let bit = 1u64 << (b_idx & 63);
        if (visited_buckets[word] & bit) != 0 {
            return;
        }
        visited_buckets[word] |= bit;
        self.visit_cell_unchecked(cx, cy, f);
    }

    /// Запрос всех сущностей в радиусе с переиспользованием внешнего буфера (0 аллокаций в куче).
    #[inline]
    pub fn query_nearby_into(&self, pos: Vec2, radius: f32, out: &mut Vec<usize>) {
        out.clear();
        let pad = Vec2::splat(radius.max(0.0));
        let Some((min_cx, max_cx, min_cy, max_cy)) = self.query_cell_bounds(pos - pad, pos + pad) else {
            return;
        };

        match self.backend {
            SpatialGridBackend::HashedLinked => {
                for cy in min_cy..=max_cy {
                    for cx in min_cx..=max_cx {
                        let b_idx = bucket_idx(cx, cy);
                        let mut curr = self.heads[b_idx];
                        while curr != -1 {
                            out.push(self.entities[curr as usize]);
                            curr = self.next[curr as usize];
                        }
                    }
                }
            }
            SpatialGridBackend::DenseLinked => {
                for cy in min_cy..=max_cy {
                    let row_base = cy * self.cols;
                    for cx in min_cx..=max_cx {
                        let mut curr = self.heads[row_base + cx];
                        while curr != -1 {
                            out.push(self.entities[curr as usize]);
                            curr = self.next[curr as usize];
                        }
                    }
                }
            }
            SpatialGridBackend::DenseCsr => {
                for cy in min_cy..=max_cy {
                    let row_base = cy * self.cols;
                    for cx in min_cx..=max_cx {
                        let ci = row_base + cx;
                        let start = self.csr_offsets[ci] as usize;
                        let end = self.csr_offsets[ci + 1] as usize;
                        if start < end {
                            out.extend_from_slice(&self.entities[start..end]);
                        }
                    }
                }
            }
        }
    }

    /// Запрос всех сущностей в радиусе (проверяет соседние ячейки сетки).
    pub fn query_nearby(&self, pos: Vec2, radius: f32) -> Vec<usize> {
        let mut result = Vec::with_capacity(32);
        self.query_nearby_into(pos, radius, &mut result);
        result
    }

    /// Количество сущностей в окне ячеек вокруг `pos`.
    pub fn count_nearby(&self, pos: Vec2, radius: f32) -> usize {
        let pad = Vec2::splat(radius.max(0.0));
        let Some((min_cx, max_cx, min_cy, max_cy)) = self.query_cell_bounds(pos - pad, pos + pad) else {
            return 0;
        };

        let mut count = 0usize;
        match self.backend {
            SpatialGridBackend::HashedLinked => {
                for cy in min_cy..=max_cy {
                    for cx in min_cx..=max_cx {
                        let b_idx = bucket_idx(cx, cy);
                        let mut curr = self.heads[b_idx];
                        while curr != -1 {
                            count += 1;
                            curr = self.next[curr as usize];
                        }
                    }
                }
            }
            SpatialGridBackend::DenseLinked => {
                for cy in min_cy..=max_cy {
                    let row_base = cy * self.cols;
                    for cx in min_cx..=max_cx {
                        let mut curr = self.heads[row_base + cx];
                        while curr != -1 {
                            count += 1;
                            curr = self.next[curr as usize];
                        }
                    }
                }
            }
            SpatialGridBackend::DenseCsr => {
                for cy in min_cy..=max_cy {
                    let row_base = cy * self.cols;
                    for cx in min_cx..=max_cx {
                        let ci = row_base + cx;
                        count += (self.csr_offsets[ci + 1] - self.csr_offsets[ci]) as usize;
                    }
                }
            }
        }
        count
    }

    /// Без-аллокационный обход всех уникальных сущностей в мировом прямоугольнике [min_pos, max_pos].
    #[inline]
    pub fn query_aabb_unique<F>(&self, min_pos: Vec2, max_pos: Vec2, mut f: F)
    where
        F: FnMut(usize),
    {
        let Some((min_cx, max_cx, min_cy, max_cy)) = self.query_cell_bounds(min_pos, max_pos) else {
            return;
        };

        match self.backend {
            SpatialGridBackend::HashedLinked => {
                let mut visited = [0u64; SPATIAL_TABLE_SIZE / 64];
                for cy in min_cy..=max_cy {
                    for cx in min_cx..=max_cx {
                        self.visit_cell_unique(cx, cy, &mut visited, &mut f);
                    }
                }
            }
            SpatialGridBackend::DenseLinked | SpatialGridBackend::DenseCsr => {
                // В плотной сетке ячейки прямоугольника уже строго уникальны — маска дубликатов не нужна.
                for cy in min_cy..=max_cy {
                    for cx in min_cx..=max_cx {
                        self.visit_cell_unchecked(cx, cy, &mut f);
                    }
                }
            }
        }
    }

    /// Точное параметрическое отсечение отрезка по AABB (алгоритм Ляна–Барски).
    #[inline]
    fn clip_segment_liang_barsky(
        raw_x0: f32,
        raw_y0: f32,
        raw_x1: f32,
        raw_y1: f32,
        min_x: f32,
        max_x: f32,
        min_y: f32,
        max_y: f32,
    ) -> Option<(f32, f32, f32, f32)> {
        let dx = raw_x1 - raw_x0;
        let dy = raw_y1 - raw_y0;
        let mut t0 = 0.0f32;
        let mut t1 = 1.0f32;

        let checks = [
            (-dx, raw_x0 - min_x),
            (dx, max_x - raw_x0),
            (-dy, raw_y0 - min_y),
            (dy, max_y - raw_y0),
        ];

        for (p, q) in checks {
            if p == 0.0 {
                if q < 0.0 {
                    return None;
                }
            } else {
                let r = q / p;
                if p < 0.0 {
                    if r > t1 {
                        return None;
                    }
                    if r > t0 {
                        t0 = r;
                    }
                } else {
                    if r < t0 {
                        return None;
                    }
                    if r < t1 {
                        t1 = r;
                    }
                }
            }
        }

        if t0 > t1 {
            return None;
        }

        Some((
            raw_x0 + dx * t0,
            raw_y0 + dy * t0,
            raw_x0 + dx * t1,
            raw_y0 + dy * t1,
        ))
    }

    /// Консервативная толстая трассировка отрезка (Thick Supercover DDA / Capsule Traversal).
    /// Для коротких лучей в плотных бэкендах использует прямой уникальный обход AABB без маски.
    #[inline]
    pub fn query_segment_thick<F>(&self, a: Vec2, b: Vec2, radius: f32, mut f: F)
    where
        F: FnMut(usize),
    {
        if self.entities.is_empty() || self.cols == 0 || self.rows == 0 || !a.is_finite() || !b.is_finite() {
            return;
        }
        let rad = radius.max(0.0);

        // Быстрый путь для коротких лучей в плотной сетке (<= 16 ячеек в AABB, без маски дубликатов).
        if self.backend != SpatialGridBackend::HashedLinked {
            let pad = Vec2::splat(rad + 1e-3);
            let min_p = a.min(b) - pad;
            let max_p = a.max(b) + pad;
            let Some((min_cx, max_cx, min_cy, max_cy)) = self.query_cell_bounds(min_p, max_p) else {
                return;
            };
            let area = (max_cx - min_cx + 1) * (max_cy - min_cy + 1);
            if area <= 16 {
                for cy in min_cy..=max_cy {
                    for cx in min_cx..=max_cx {
                        self.visit_cell_unchecked(cx, cy, &mut f);
                    }
                }
                return;
            }
        }

        let offset = self.offset;
        let inv_h = self.inv_cell_size;
        let pad_norm = rad * inv_h + 1e-4;

        let raw_x0 = (a.x + offset) * inv_h;
        let raw_y0 = (a.y + offset) * inv_h;
        let raw_x1 = (b.x + offset) * inv_h;
        let raw_y1 = (b.y + offset) * inv_h;

        let Some((x0, y0, x1, y1)) = Self::clip_segment_liang_barsky(
            raw_x0,
            raw_y0,
            raw_x1,
            raw_y1,
            -pad_norm,
            self.cols as f32 + pad_norm,
            -pad_norm,
            self.rows as f32 + pad_norm,
        ) else {
            return;
        };

        let start_x = x0.floor() as isize;
        let start_y = y0.floor() as isize;
        let end_x = x1.floor() as isize;
        let end_y = y1.floor() as isize;

        let dx = x1 - x0;
        let dy = y1 - y0;

        let step_x: isize = if dx > 0.0 { 1 } else if dx < 0.0 { -1 } else { 0 };
        let step_y: isize = if dy > 0.0 { 1 } else if dy < 0.0 { -1 } else { 0 };

        let t_delta_x = if dx.abs() > 1e-7 { 1.0 / dx.abs() } else { f32::INFINITY };
        let t_delta_y = if dy.abs() > 1e-7 { 1.0 / dy.abs() } else { f32::INFINITY };

        let mut t_max_x = if step_x > 0 {
            ((start_x as f32 + 1.0) - x0) * t_delta_x
        } else if step_x < 0 {
            (x0 - start_x as f32) * t_delta_x
        } else {
            f32::INFINITY
        };

        let mut t_max_y = if step_y > 0 {
            ((start_y as f32 + 1.0) - y0) * t_delta_y
        } else if step_y < 0 {
            (y0 - start_y as f32) * t_delta_y
        } else {
            f32::INFINITY
        };

        let mut visited = self.new_visited_mask();
        let mut t_curr = 0.0f32;
        let max_steps = (end_x - start_x).abs() + (end_y - start_y).abs() + 4;
        let max_col = self.cols as isize - 1;
        let max_row = self.rows as isize - 1;

        for _ in 0..max_steps {
            let t_next = t_max_x.min(t_max_y).min(1.0);
            let qx0 = x0 + dx * t_curr;
            let qy0 = y0 + dy * t_curr;
            let qx1 = x0 + dx * t_next;
            let qy1 = y0 + dy * t_next;

            let nx_min = ((qx0.min(qx1) - pad_norm).floor() as isize).max(0);
            let nx_max = ((qx0.max(qx1) + pad_norm).floor() as isize).min(max_col);
            let ny_min = ((qy0.min(qy1) - pad_norm).floor() as isize).max(0);
            let ny_max = ((qy0.max(qy1) + pad_norm).floor() as isize).min(max_row);

            if nx_min <= nx_max && ny_min <= ny_max {
                for ny in ny_min as usize..=ny_max as usize {
                    for nx in nx_min as usize..=nx_max as usize {
                        self.visit_cell_with_mask(nx, ny, &mut visited, &mut f);
                    }
                }
            }

            if t_next >= 1.0 {
                break;
            }
            t_curr = t_next;

            if t_max_x < t_max_y {
                t_max_x += t_delta_x;
            } else if t_max_y < t_max_x {
                t_max_y += t_delta_y;
            } else {
                t_max_x += t_delta_x;
                t_max_y += t_delta_y;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_BACKENDS: [SpatialGridBackend; 3] = [
        SpatialGridBackend::HashedLinked,
        SpatialGridBackend::DenseLinked,
        SpatialGridBackend::DenseCsr,
    ];

    #[test]
    fn test_k_star_adjacent_zero_collisions_3x3() {
        let cols = 207;
        let rows = 139;

        for cy in 1..rows - 1 {
            for cx in 1..cols - 1 {
                let mut seen = [0usize; 9];
                let mut idx = 0;
                for dy in [-1isize, 0, 1] {
                    for dx in [-1isize, 0, 1] {
                        let ncx = (cx as isize + dx) as usize;
                        let ncy = (cy as isize + dy) as usize;
                        let b = bucket_idx(ncx, ncy);
                        assert!(b < SPATIAL_TABLE_SIZE);
                        for &prev in &seen[..idx] {
                            assert_ne!(
                                prev, b,
                                "Коллизия в окрестности 3x3 у ячейки ({}, {})! b={}",
                                cx, cy, b
                            );
                        }
                        seen[idx] = b;
                        idx += 1;
                    }
                }
            }
        }
    }

    #[test]
    fn test_spatial_grid_insert_query_clear() {
        for backend in ALL_BACKENDS {
            let mut grid = SpatialGrid::new_with_backend(25920.0, 17280.0, backend);
            if backend == SpatialGridBackend::HashedLinked {
                assert_eq!(grid.heads.len(), SPATIAL_TABLE_SIZE);
            }

            let p1 = Vec2::new(1000.0, 1000.0);
            let p2 = Vec2::new(1050.0, 1020.0);
            let p_far = Vec2::new(8000.0, 8000.0);

            grid.insert(1, p1);
            grid.insert(2, p2);
            grid.insert(99, p_far);

            let nearby = grid.query_nearby(p1, 100.0);
            assert!(nearby.contains(&1), "{:?} missed id 1", backend);
            assert!(nearby.contains(&2), "{:?} missed id 2", backend);
            assert!(!nearby.contains(&99), "{:?} leaked far id 99", backend);

            assert_eq!(grid.count_nearby(p1, 100.0), 2, "{:?} count_nearby", backend);

            grid.clear();
            assert_eq!(grid.count_nearby(p1, 100.0), 0, "{:?} clear count", backend);
            assert!(grid.active_coords().is_empty(), "{:?} active_coords after clear", backend);
        }
    }

    #[test]
    fn test_thick_supercover_dda_captures_boundary_mobs_without_duplicates() {
        for backend in ALL_BACKENDS {
            let mut grid = SpatialGrid::new_with_backend(25920.0, 17280.0, backend);

            let beam_a = Vec2::new(100.0, 380.0);
            let beam_b = Vec2::new(1100.0, 380.0);
            let mob_neighbor = Vec2::new(500.0, 395.0);
            let mob_center = Vec2::new(650.0, 380.0);
            let mob_far = Vec2::new(650.0, 700.0);

            grid.insert(10, mob_neighbor);
            grid.insert(20, mob_center);
            grid.insert(30, mob_far);

            let mut hits = Vec::new();
            grid.query_segment_thick(beam_a, beam_b, 40.0, |idx| hits.push(idx));

            assert!(hits.contains(&10), "{:?}: missed neighbor within corridor", backend);
            assert!(hits.contains(&20), "{:?}: missed centerline target", backend);
            assert!(!hits.contains(&30), "{:?}: hit far mob", backend);
            let mut copy = hits.clone();
            copy.sort_unstable();
            copy.dedup();
            assert_eq!(hits.len(), copy.len(), "{:?}: duplicate visits!", backend);
        }
    }

    #[test]
    fn test_thick_supercover_dda_diagonal_corner_safety() {
        for backend in ALL_BACKENDS {
            let mut grid = SpatialGrid::new_with_backend(25920.0, 17280.0, backend);
            let a = Vec2::new(100.0, 100.0);
            let b = Vec2::new(1124.0, 1124.0);

            let targets = [
                Vec2::new(256.0, 256.0),
                Vec2::new(512.0, 492.0),
                Vec2::new(768.0, 790.0),
            ];
            grid.rebuild_from_positions(&targets);

            let mut found = Vec::new();
            grid.query_segment_thick(a, b, 35.0, |idx| found.push(idx));
            assert!(found.contains(&0), "{:?}", backend);
            assert!(found.contains(&1), "{:?}", backend);
            assert!(found.contains(&2), "{:?}", backend);
        }
    }

    #[test]
    fn test_query_segment_thick_out_of_bounds_oracle_liang_barsky() {
        let dist_point_to_segment = |p: Vec2, a: Vec2, b: Vec2| -> f32 {
            let ab = b - a;
            let len_sq = ab.length_squared();
            if len_sq < 1e-8 {
                return p.distance(a);
            }
            let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
            p.distance(a + ab * t)
        };

        let mut positions = Vec::new();
        for ix in 0..30 {
            for iy in 0..20 {
                positions.push(Vec2::new(ix as f32 * 850.0, iy as f32 * 850.0 - 200.0));
            }
        }
        let boundary_hit_id = positions.len();
        positions.push(Vec2::new(4400.0, -200.0));
        let bent_corner_id = positions.len();
        positions.push(Vec2::new(25800.0, -200.0));

        let scenarios = [
            (Vec2::new(2000.0, 1000.0), Vec2::new(32000.0, -14000.0), 40.0f32),
            (Vec2::new(-5000.0, 4000.0), Vec2::new(35000.0, 2000.0), 120.0f32),
            (Vec2::new(-1000.0, -280.0), Vec2::new(8000.0, -280.0), 64.0f32),
            (Vec2::new(-2000.0, -2000.0), Vec2::new(30000.0, -2000.0), 50.0f32),
            // Короткий луч (попадает в AABB <= 16 ячеек быстрый путь)
            (Vec2::new(840.0, 640.0), Vec2::new(900.0, 690.0), 30.0f32),
        ];

        for backend in ALL_BACKENDS {
            let mut grid = SpatialGrid::new_with_backend(25920.0, 17280.0, backend);
            grid.rebuild_from_positions(&positions);

            for (case_idx, &(a, b, radius)) in scenarios.iter().enumerate() {
                let mut hits = Vec::new();
                grid.query_segment_thick(a, b, radius, |id| hits.push(id));

                let mut sorted = hits.clone();
                sorted.sort_unstable();
                sorted.dedup();
                assert_eq!(hits.len(), sorted.len(), "{:?} Case {}: duplicate visits", backend, case_idx);

                for (id, &pos) in positions.iter().enumerate() {
                    if grid.get_cell_coords(pos).is_none() {
                        continue;
                    }
                    if dist_point_to_segment(pos, a, b) <= radius {
                        assert!(
                            sorted.binary_search(&id).is_ok(),
                            "{:?} Case {}: oracle hit id={} at {:?} missed",
                            backend,
                            case_idx,
                            id,
                            pos
                        );
                    }
                }

                if case_idx == 0 {
                    assert!(sorted.binary_search(&boundary_hit_id).is_ok(), "{:?}", backend);
                    assert!(sorted.binary_search(&bent_corner_id).is_err(), "{:?}", backend);
                }
                if case_idx == 3 {
                    assert!(hits.is_empty(), "{:?}", backend);
                }
            }
        }
    }

    #[test]
    fn test_dense_backends_exact_cell_parity_with_oracle() {
        let mut positions = Vec::with_capacity(600);
        for i in 0..600usize {
            let x = (i * 2137) % 25690;
            let y = (i * 1999) % 17070;
            positions.push(Vec2::new(x as f32, y as f32));
        }
        // Дублирующиеся/пограничные точки на сетке и около 0
        positions.push(Vec2::new(0.0, 0.0));
        positions.push(Vec2::new(0.0, 0.0));
        positions.push(Vec2::new(25919.0, 17279.0));

        let mut g_linked = SpatialGrid::new_with_backend(25920.0, 17280.0, SpatialGridBackend::DenseLinked);
        let mut g_csr = SpatialGrid::new_with_backend(25920.0, 17280.0, SpatialGridBackend::DenseCsr);
        g_linked.rebuild_from_positions(&positions);
        g_csr.rebuild_from_positions(&positions);

        let probes = [
            (Vec2::new(0.0, 0.0), 180.0),
            (Vec2::new(12800.0, 8640.0), 450.0),
            (Vec2::new(25900.0, 17250.0), 220.0),
        ];
        for (center, radius) in probes {
            let mut a = g_linked.query_nearby(center, radius);
            let mut b = g_csr.query_nearby(center, radius);
            a.sort_unstable();
            b.sort_unstable();
            assert_eq!(a, b, "DenseLinked and DenseCsr must return identical candidate sets");
            assert_eq!(g_linked.count_nearby(center, radius), g_csr.count_nearby(center, radius));
            for (idx, &p) in positions.iter().enumerate() {
                if p.distance(center) <= radius {
                    assert!(b.binary_search(&idx).is_ok(), "Dense oracle missed idx={}", idx);
                }
            }
        }
        assert_eq!(g_linked.active_coords().len(), g_csr.active_coords().len());
    }
}
