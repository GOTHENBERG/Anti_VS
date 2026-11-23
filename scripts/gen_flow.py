# gen_flow.py
# Генератор Flow Field для арены 3000x3000, сетка 100x100, h=30
# Уравнение: Delta u = -1, u(boss)=0, Neumann на стенах и границах
# Выход: flow_field.npz, u_raw.bin (40000 байт), v_raw.bin (80000 байт), flow_field.rs
# Зависимости: только numpy, matplotlib опционально для картинки

import numpy as np
import math
import os

# ============ 1. Параметры ============
N = 100
h = 30.0
ib, jb = 50, 50  # босс: i=X, j=Y, (1500,1500) px
OUT_DIR = "assets/flow"
os.makedirs(OUT_DIR, exist_ok=True)
USE_PLOT = True  # поставь False, если нет matplotlib

# ============ 2. Маска препятствий ============
# ВАЖНО: omega_obs[j, i], j = Y вниз, i = X вправо
# Спецификация, не транспонированный код из прошлого сообщения
omega_obs = np.zeros((N, N), dtype=bool)

# 4 Древних Обелиска 11x11
omega_obs[4:15, 4:15] = True    # NW: i 4-14, j 4-14
omega_obs[4:15, 85:96] = True   # NE: i 85-95, j 4-14
omega_obs[85:96, 4:15] = True   # SW: i 4-14, j 85-95
omega_obs[85:96, 85:96] = True  # SE: i 85-95, j 85-95

# Стартовый Алтарь Крови 4x4
omega_obs[50:54, 40:44] = True  # i 40-43, j 50-53

# Тактические баррикады
omega_obs[35:66, 45] = True     # защитная линия: i=45, j 35-65
omega_obs[40:61, 58] = True     # Г-ловушка вертикаль: i=58, j 40-60
omega_obs[60, 50:59] = True     # Г-ловушка перемычка: j=60, i 50-58

assert not omega_obs[jb, ib], "Босс не должен быть внутри препятствия!"
print(f"Препятствий: {int(omega_obs.sum())}")
print(f"Свободных: {int((~omega_obs).sum())}, неизвестных M = {int((~omega_obs).sum()) - 1}")

# ============ 3. Индексация неизвестных ============
idx = -np.ones((N, N), dtype=int)
cnt = 0
for j in range(N):
    for i in range(N):
        if not omega_obs[j, i] and not (i == ib and j == jb):
            idx[j, i] = cnt
            cnt += 1
M = cnt

# ============ 4. Сборка SPD системы A u = b ============
# Для узла: n_free * u - sum(u_neigh) = h^2
# n_free = число соседей не стена и не вне карты, включая босса
# Стена / вне карты = ghost = u_i, вклад 0
# Босс u=0, в сумму не входит, но в n_free входит
diag = np.zeros(M, dtype=np.float64)
src = []
dst = []
for j in range(N):
    for i in range(N):
        k = idx[j, i]
        if k < 0:
            continue
        nfree = 0
        for di, dj in [(1, 0), (-1, 0), (0, 1), (0, -1)]:
            ni, nj = i + di, j + dj
            if ni < 0 or ni >= N or nj < 0 or nj >= N:
                continue
            if omega_obs[nj, ni]:
                continue
            nfree += 1
            if not (ni == ib and nj == jb):
                src.append(k)
                dst.append(idx[nj, ni])
        diag[k] = nfree
src = np.array(src, dtype=int)
dst = np.array(dst, dtype=int)
rhs = np.full(M, h * h, dtype=np.float64)
print(f"Рёбер: {len(src)}, неизвестных: {M}")

def mv(x):
    y = diag * x
    np.subtract.at(y, src, x[dst])
    return y

# ============ 5. Conjugate Gradient ============
# Можно добавить Jacobi предобуславливатель: x /= diag, но тут и так сходится за ~650 итераций
x = np.zeros(M, dtype=np.float64)
r = rhs - mv(x)
p = r.copy()
rsold = float(np.dot(r, r))
print(f"CG старт |r| = {math.sqrt(rsold):.2f}")
converged = False
for it in range(5000):
    Ap = mv(p)
    denom = float(np.dot(p, Ap))
    if denom == 0:
        break
    alpha = rsold / denom
    x += alpha * p
    r -= alpha * Ap
    rsnew = float(np.dot(r, r))
    if it % 200 == 0:
        print(f"  iter {it} |r| = {math.sqrt(rsnew):.4f}")
    if math.sqrt(rsnew) < 1e-4:
        print(f"CG сошёлся: iter {it}, |r| = {math.sqrt(rsnew):.2e}")
        converged = True
        break
    p = r + (rsnew / rsold) * p
    rsold = rsnew
if not converged:
    print("ВНИМАНИЕ: CG не сошёлся до 1e-4, увеличь maxiter или добавь предобуславливатель")

# ============ 6. Восстановить U ============
U = np.full((N, N), np.inf, dtype=np.float64)
U[jb, ib] = 0.0
for j in range(N):
    for i in range(N):
        if idx[j, i] >= 0:
            U[j, i] = x[idx[j, i]]

# Проверка остатка для случайных узлов, опционально
# Для узла (i,j): (sum_neigh - n_free*u) / h^2 должно быть -1
def check_residual(i, j):
    if omega_obs[j, i] or (i == ib and j == jb):
        return None
    s = 0.0
    nfree = 0
    for di, dj in [(1, 0), (-1, 0), (0, 1), (0, -1)]:
        ni, nj = i + di, j + dj
        if ni < 0 or ni >= N or nj < 0 or nj >= N:
            continue
        if omega_obs[nj, ni]:
            continue
        nfree += 1
        s += U[nj, ni]
    return (s - nfree * U[j, i]) / (h * h)

for (ti, tj) in [(20, 20), (70, 20), (30, 70), (80, 80), (55, 55)]:
    res = check_residual(ti, tj)
    if res is not None:
        print(f"  residual Delta u в ({ti},{tj}) = {res:.4f} (должно быть -1)")

# ============ 7. Flow Field V = -grad U / |grad U| ============
# На границах используем одностороннюю разность, чтобы не было смещения как при ghost
Gx = np.zeros((N, N), dtype=np.float64)
Gy = np.zeros((N, N), dtype=np.float64)
for j in range(N):
    for i in range(N):
        if omega_obs[j, i]:
            continue
        # X-градиент
        if i == 0:
            # односторонняя вперёд
            if not omega_obs[j, i + 1]:
                Gx[j, i] = (U[j, i + 1] - U[j, i]) / h
            else:
                Gx[j, i] = 0.0
        elif i == N - 1:
            if not omega_obs[j, i - 1]:
                Gx[j, i] = (U[j, i] - U[j, i - 1]) / h
            else:
                Gx[j, i] = 0.0
        else:
            left = U[j, i - 1] if not omega_obs[j, i - 1] else U[j, i]
            right = U[j, i + 1] if not omega_obs[j, i + 1] else U[j, i]
            Gx[j, i] = (right - left) / (2 * h)
        # Y-градиент, Y вниз
        if j == 0:
            if not omega_obs[j + 1, i]:
                Gy[j, i] = (U[j + 1, i] - U[j, i]) / h
            else:
                Gy[j, i] = 0.0
        elif j == N - 1:
            if not omega_obs[j - 1, i]:
                Gy[j, i] = (U[j, i] - U[j - 1, i]) / h
            else:
                Gy[j, i] = 0.0
        else:
            up = U[j - 1, i] if not omega_obs[j - 1, i] else U[j, i]
            down = U[j + 1, i] if not omega_obs[j + 1, i] else U[j, i]
            Gy[j, i] = (down - up) / (2 * h)

norm = np.sqrt(Gx**2 + Gy**2) + 1e-12
Vx = -Gx / norm
Vy = -Gy / norm
Vx[jb, ib] = 0.0
Vy[jb, ib] = 0.0
Vx[omega_obs] = 0.0
Vy[omega_obs] = 0.0

# ============ 8. Тест следования ============
def follow(sx, sy, max_steps=5000):
    x, y = float(sx), float(sy)
    for step in range(max_steps):
        ix, iy = int(round(x)), int(round(y))
        if ix == ib and iy == jb:
            return step, True, (x, y)
        if ix < 0 or ix >= N or iy < 0 or iy >= N:
            return step, False, (x, y)
        if omega_obs[iy, ix]:
            return step, False, (x, y)
        x += Vx[iy, ix]
        y += Vy[iy, ix]
    return max_steps, False, (x, y)

print("Тест следования по V:")
for start in [(1, 1), (98, 1), (1, 98), (98, 98), (55, 55), (30, 50), (70, 50), (50, 10), (50, 90)]:
    steps, ok, end = follow(start[0], start[1])
    print(f"  {start} -> шагов {steps}, дошёл: {ok}, конец {end}")

# ============ 9. Сохранение ============
U32 = np.where(np.isinf(U), np.float32(1e9), U).astype(np.float32)
V = np.stack([Vx, Vy], axis=-1).astype(np.float32)  # (100,100,2), V[...,0]=Vx, V[...,1]=Vy

np.savez(os.path.join(OUT_DIR, "flow_field.npz"), u=U32, v=V, obs=omega_obs)
U32.tofile(os.path.join(OUT_DIR, "u_raw.bin"))
V.tofile(os.path.join(OUT_DIR, "v_raw.bin"))
print(f"Сохранено: flow_field.npz, u_raw.bin ({U32.nbytes} байт), v_raw.bin ({V.nbytes} байт)")

# .rs файл для include! без bytemuck
def f32_to_rust(v):
    if math.isinf(float(v)):
        return "1e9f32"
    return f"{float(v):.8e}f32".replace("e", "e")

with open(os.path.join(OUT_DIR, "flow_field.rs"), "w", encoding="utf-8") as f:
    f.write("// Auto-generated by gen_flow.py. Delta u = -1, u(boss)=0, Neumann.\n")
    f.write("// Index: u[j*100 + i], v[(j*100+i)*2 + 0] = Vx, +1 = Vy. j=Y down, i=X right.\n")
    f.write("// Obstacles in u = 1e9, in v = (0,0).\n\n")
    f.write("pub const N: usize = 100;\n")
    f.write("pub const H: f32 = 30.0;\n")
    f.write("pub const BOSS_I: usize = 50;\n")
    f.write("pub const BOSS_J: usize = 50;\n\n")
    f.write("pub const U: [f32; 10000] = [\n")
    flat_u = U32.flatten()
    for k in range(0, 10000, 10):
        chunk = ", ".join(f32_to_rust(v) for v in flat_u[k:k+10])
        f.write(f"    {chunk},\n")
    f.write("];\n\n")
    f.write("pub const V: [f32; 20000] = [\n")
    flat_v = V.flatten()
    for k in range(0, 20000, 10):
        chunk = ", ".join(f32_to_rust(v) for v in flat_v[k:k+10])
        f.write(f"    {chunk},\n")
    f.write("];\n\n")
    f.write("pub const OBS: [bool; 10000] = [\n")
    flat_o = omega_obs.flatten()
    for k in range(0, 10000, 20):
        chunk = ", ".join("true" if v else "false" for v in flat_o[k:k+20])
        f.write(f"    {chunk},\n")
    f.write("];\n")
print("Сохранено: flow_field.rs")

# ============ 10. Картинка, опционально ============
if USE_PLOT:
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        fig, axes = plt.subplots(1, 2, figsize=(14, 6))
        ax = axes[0]
        im = ax.imshow(np.where(np.isinf(U), np.nan, U), cmap="inferno", origin="upper")
        ax.set_title("U: Delta=-1, boss=0 (Y down)")
        fig.colorbar(im, ax=ax, shrink=0.8)
        oy, ox = np.where(omega_obs)
        ax.scatter(ox, oy, s=3, c="white", alpha=0.7)
        ax.scatter([ib], [jb], c="cyan", s=100, marker="*", edgecolors="white")
        ax = axes[1]
        ax.imshow(np.where(omega_obs, 0, 1), cmap="gray", origin="upper", alpha=0.15)
        step = 3
        Y, X = np.mgrid[0:N:step, 0:N:step]
        Uq = Vx[::step, ::step]
        Vq = Vy[::step, ::step]
        mobs = omega_obs[::step, ::step]
        Uq = np.where(mobs, 0, Uq)
        Vq = np.where(mobs, 0, Vq)
        ax.quiver(X, Y, Uq, Vq, color="red", scale=28, width=0.003)
        ax.scatter([ib], [jb], c="cyan", s=130, marker="*", edgecolors="black")
        ax.set_title("Flow V = -grad U / |grad U|")
        ax.set_xlim(-1, N)
        ax.set_ylim(N, -1)
        plt.tight_layout()
        plt.savefig(os.path.join(OUT_DIR, "flow_field.png"), dpi=150)
        print("Сохранено: flow_field.png")
    except Exception as e:
        print(f"Картинка не сделана: {e}")

print("Готово.")
