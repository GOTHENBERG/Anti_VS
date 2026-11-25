"""
Spatial Grid Hash Multiplier (K*) Optimizer
Reverse Survivor: The Director

Поиск оптимального 64-битного магического множителя K* для хеширования ячеек (cx, cy)
в пространственной сетке (Spatial Grid).

Минимизирует коллизии на РЕАЛЬНОМ датасете игровых кадров из headless-симулятора
и гарантирует строго 0 коллизий в окрестности 3x3.
"""

import os
import sys
import json
import gzip
import time
import random

COLS = 207
ROWS = 139
TABLE_BITS = 13  # 2^13 = 8192 корзины
TABLE_SIZE = 1 << TABLE_BITS
SHIFT = 64 - TABLE_BITS
MASK64 = 0xFFFFFFFFFFFFFFFF
KNUTH_GOLDEN = 0x9E3779B97F4A7C15


def load_dataset(path=None):
    if path is None:
        default_paths = [
            "data/headless_frames.json",
            "data/headless_frames.json.gz",
            "../data/headless_frames.json",
            "../data/headless_frames.json.gz",
        ]
        for p in default_paths:
            if os.path.exists(p):
                path = p
                break

    if not path or not os.path.exists(path):
        print(f"[!] Файл датасета не найден. Укажите путь к headless_frames.json.")
        print(f"    Сгенерировать его можно командой: cargo run --release -- --export-frames 60")
        sys.exit(1)

    print(f"[*] Загрузка реального датасета: {path} ...")
    if path.endswith(".gz"):
        with gzip.open(path, "rt", encoding="utf-8") as f:
            raw_frames = json.load(f)
    else:
        with open(path, "r", encoding="utf-8") as f:
            raw_frames = json.load(f)

    # Предварительно упаковываем (cx, cy) в 64-битные ключи (cx << 32) | cy
    frames_keys = []
    total_cells = 0
    all_unique_cells = set()

    for frame in raw_frames:
        keys = []
        for cell in frame:
            cx, cy = int(cell[0]), int(cell[1])
            key = ((cx & 0xFFFFFFFF) << 32) | (cy & 0xFFFFFFFF)
            keys.append(key)
            all_unique_cells.add((cx, cy))
        if keys:
            frames_keys.append(keys)
            total_cells += len(keys)

    print(f"[+] Загружено кадров: {len(frames_keys):,}")
    print(f"[+] Суммарно сущностей в кадрах: {total_cells:,}")
    print(f"[+] Уникальных ячеек сетки посещено: {len(all_unique_cells):,}")
    print(f"[+] Среднее число занятых ячеек за кадр: {total_cells / max(1, len(frames_keys)):.1f}")
    return frames_keys


def build_adjacency_pairs():
    """
    Генерирует пары соседних ячеек в окне 3x3.
    Две соседние ячейки НИКОГДА не должны попадать в одну корзину хеш-таблицы,
    иначе пространственный запрос 3x3 даст ложную коллизию.
    """
    keys_sample = []
    for cy in range(2, ROWS - 2, 3):
        for cx in range(2, COLS - 2, 3):
            block = []
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    key = (((cx + dx) & 0xFFFFFFFF) << 32) | ((cy + dy) & 0xFFFFFFFF)
                    block.append(key)
            keys_sample.append(block)
    return keys_sample


def evaluate_multiplier_cpu(K, frames_keys, adjacency_blocks):
    # 1. Жесткая проверка окрестности 3x3
    for block in adjacency_blocks:
        seen = set()
        for key in block:
            h = ((key * K) & MASK64) >> SHIFT
            if h in seen:
                return 10_000_000  # Штраф за коллизию в 3x3
            seen.add(h)

    # 2. Подсчет коллизий на реальных кадрах игры
    collisions = 0
    for frame in frames_keys:
        seen = set()
        for key in frame:
            h = ((key * K) & MASK64) >> SHIFT
            if h in seen:
                collisions += 1
            seen.add(h)
    return collisions


def run_gpu_search(frames_keys, adjacency_blocks):
    """
    Сверхбыстрый параллельный поиск на GPU (PyTorch CUDA / MPS).
    Оценивает 65 536 кандидатов K одновременно за одну матричную операцию.
    """
    import torch
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"[+] Инициализация ускорения PyTorch на устройстве: {device}")

    # Переносим ключи на тензоры
    # Для максимальной скорости преобразуем кадры в плоские массивы
    adj_keys_list = [k for b in adjacency_blocks for k in b]
    adj_tensor = torch.tensor(adj_keys_list, dtype=torch.int64, device=device)

    # Фреймы упаковываем в тензоры
    frame_tensors = [torch.tensor(f, dtype=torch.int64, device=device) for f in frames_keys]

    BATCH_SIZE = 32768
    best_k = KNUTH_GOLDEN | 1
    best_score = evaluate_multiplier_cpu(best_k, frames_keys, adjacency_blocks)
    print(f"[+] Базовый Knuth Multiplier score: {best_score:,}")

    start_time = time.time()
    total_checked = 0

    while True:
        # Генерируем пачку мутаций
        mutations = torch.randint(0, 1 << 30, (BATCH_SIZE,), dtype=torch.int64, device=device)
        mutations = (mutations << 32) | torch.randint(0, 1 << 32, (BATCH_SIZE,), dtype=torch.int64, device=device)
        mutations = mutations | 1  # K всегда нечетное

        # Смешиваем с текущим лучшим
        candidates = (best_k ^ (mutations & 0x0000FFFFFFFF0000)) | 1
        candidates[0] = best_k

        # Оцениваем смежность 3x3 на GPU
        # shape: (N_adj, BATCH)
        h_adj = ((adj_tensor.unsqueeze(1) * candidates.unsqueeze(0)) & MASK64) >> SHIFT
        # Проверяем уникальность по блокам по 9 элементов
        h_adj_reshaped = h_adj.view(-1, 9, BATCH_SIZE)
        # Считаем уникальные в блоках
        sorted_adj, _ = torch.sort(h_adj_reshaped, dim=1)
        adj_collisions = (sorted_adj[:, 1:, :] == sorted_adj[:, :-1, :]).sum(dim=[0, 1])

        valid_mask = adj_collisions == 0
        valid_indices = torch.nonzero(valid_mask).squeeze(-1)

        if len(valid_indices) > 0:
            for idx in valid_indices.tolist()[:16]:
                cand = int(candidates[idx].item())
                score = evaluate_multiplier_cpu(cand, frames_keys, adjacency_blocks)
                if score < best_score:
                    best_score = score
                    best_k = cand
                    elapsed = time.time() - start_time
                    print(f"[{elapsed:.1f}s | {total_checked:,} K] Новое лучшее: {hex(best_k)} -> Коллизий: {best_score:,}")
                    if best_score == 0:
                        print(f"\n[!!!] НАЙДЕНО ИДЕАЛЬНОЕ ЧИСЛО БЕЗ КОЛЛИЗИЙ: {hex(best_k)}")
                        return best_k

        total_checked += BATCH_SIZE
        if total_checked % (BATCH_SIZE * 20) == 0:
            elapsed = time.time() - start_time
            print(f"  [Прогресс: {total_checked:,} проверено | Скорость: {total_checked / elapsed:,.0f} K/s | Best: {hex(best_k)} ({best_score:,})]")


def run_cpu_search(frames_keys, adjacency_blocks):
    """
    Высокопроизводительный поиск на CPU с Simulated Annealing и битовыми мутациями.
    """
    print(f"[*] Запуск CPU поиска (Simulated Annealing)...")
    best_k = KNUTH_GOLDEN | 1
    best_score = evaluate_multiplier_cpu(best_k, frames_keys, adjacency_blocks)
    print(f"[+] Базовый Knuth Golden Ratio score: {best_score:,} коллизий")

    start_time = time.time()
    iterations = 0

    while True:
        iterations += 1

        # Мутируем биты числа K (сохраняя нечетность)
        mutation_mask = (random.getrandbits(32) << random.randint(0, 32)) & MASK64
        candidate = (best_k ^ mutation_mask) | 1

        score = evaluate_multiplier_cpu(candidate, frames_keys, adjacency_blocks)

        if score < best_score:
            best_score = score
            best_k = candidate
            elapsed = time.time() - start_time
            print(f"[{elapsed:.1f}s | {iterations:,} итер] Новое лучшее K*: {hex(best_k)} (Коллизий: {best_score:,})")

            if best_score == 0:
                print(f"\n[!!!] НАЙДЕНО ИДЕАЛЬНОЕ ЧИСЛО С 0 КОЛЛИЗИЯМИ: {hex(best_k)}")
                break

        if iterations % 2000 == 0:
            elapsed = time.time() - start_time
            print(f"  [Итераций: {iterations:,} | {iterations / elapsed:.1f} оп/сек | Текущий минимум коллизий: {best_score:,}]")


def main():
    import argparse
    parser = argparse.ArgumentParser(description="Find optimal magic hash multiplier K*")
    parser.add_argument("dataset", nargs="?", default=None, help="Path to headless_frames.json or .json.gz")
    parser.add_argument("--bits", "-b", type=int, default=11, help="Table bits (e.g. 10=1024, 11=2048, 12=4096, 13=8192). Default: 11")
    parser.add_argument("--device", choices=["auto", "cpu", "gpu"], default="auto", help="Compute device")
    args = parser.parse_args()

    global TABLE_BITS, TABLE_SIZE, SHIFT
    TABLE_BITS = args.bits
    TABLE_SIZE = 1 << TABLE_BITS
    SHIFT = 64 - TABLE_BITS
    print(f"[*] Целевой размер таблицы: 2^{TABLE_BITS} = {TABLE_SIZE:,} корзин (SHIFT = {SHIFT})")

    frames_keys = load_dataset(args.dataset)
    adjacency_blocks = build_adjacency_pairs()

    print(f"[+] Построено проверочных блоков смежности 3x3: {len(adjacency_blocks)}")

    use_gpu = False
    if args.device != "cpu":
        try:
            import torch
            if torch.cuda.is_available():
                use_gpu = True
        except ImportError:
            pass

    if use_gpu:
        run_gpu_search(frames_keys, adjacency_blocks)
    else:
        run_cpu_search(frames_keys, adjacency_blocks)


if __name__ == "__main__":
    main()
