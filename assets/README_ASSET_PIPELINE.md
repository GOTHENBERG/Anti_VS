# Anti_VS — Canonical Asset Drop-in Pipeline Guide

Данный документ описывает структуру директорий и стандарт именования файлов для бесшовной интеграции (drop-in replacement) приобретенных пакетов из **Golden Asset Stack**.

---

## 1. Карта директорий и правила размещения

### 🗡️ `assets/heroes/` (Боссы игроков)
* **Источник:** *Pixel Crawler: Heroes & Characters* (Anokolisa)
* **Разрешение:** Холст `16x28` или `32x32` / `32x48` px.
* **Именование файлов:**
  - `knight_idle_0.png` ... `knight_idle_3.png` (4 кадра покоя)
  - `knight_walk_0.png` ... `knight_walk_3.png` (4 кадра шага)
  - `paladin_idle_0.png` ... `paladin_idle_3.png` (для кооп-босса)
  - `dark_knight_idle_0.png` ... (альтернативный облик)
  - `rogue_idle_0.png` ... (альтернативный облик)

---

### 👾 `assets/monsters/horde/` (Орда Директора: Тиры 1–8)
* **Источник:** *Topdown Monsters / Undead* (BDragon1727 & Anokolisa)
* **Разрешение:** `16x16`, `24x24`, `32x32` (Т8: `64x64`)
* **Именование файлов:**
  - `mob_tier01_skeleton.png` (16x16 / 24x24, белая кость)
  - `mob_tier02_hound_0.png` ... `3.png` (32x16 / 32x24, 4 кадра бега)
  - `mob_tier03_cultist.png` (24x24 / 32x32, индиго мантия)
  - `mob_tier04_golem.png` (32x32 / 48x48, ледяные кристаллы)
  - `mob_tier05_kamikaze.png` (24x24 / 32x32, яркий фитиль)
  - `mob_tier06_shadow_demon.png` (32x32 / 36x24, рогатый призрак)
  - `mob_tier07_fire_elemental.png` (32x32, огненный дух)
  - `mob_tier08_bringer_death.png` (64x64 / 96x96, Жнец Смерти с косой)

---

### 💀 `assets/monsters/legion/` (Высший Легион: Тиры 9–16)
* **Источник:** *Topdown Monsters & Dark Beings* (BDragon1727 & Anokolisa)
* **Разрешение:** `32x32`, `48x48`, `64x64`
* **Именование файлов:**
  - `mob_tier09_dread_phalanx.png` (32x32 / 40x40, строй легионеров со щитами)
  - `mob_tier10_hellhound.png` (36x24 / 48x32, адские церберы)
  - `mob_tier11_void_cultist.png` (32x36 / 40x40, архикультисты Бездны)
  - `mob_tier12_obsidian_colossus.png` (48x48 / 64x64, обсидиановые гиганты)
  - `mob_tier13_nuke_bomber.png` (32x32, мучители)
  - `mob_tier14_shadow_stalker.png` (40x40, ловчие Бездны)
  - `mob_tier15_solar_archon.png` (48x48, падшие серафимы)
  - `mob_tier16_avatar_oblivion.png` (64x64 / 80x80, Аватар Забвения)

---

### 👑 `assets/titans/` (Герои Директора: Тиры 17 и 18)
* **Источник:** *The DARK Series: Overlords & Titans* (Penusbmic)
* **Разрешение:** Холст `96x96` – `128x128` px (колоссальный масштаб).
* **Именование файлов:**
  - `mob_tier17_void_lord.png` (Владыка Пустоты: мантия, рога, Чёрная Дыра, неон)
  - `mob_tier18_inferno_archdemon.png` (Архидемон Инферно: лавовые жилы, рога, инферно)

---

### 🏛️ `assets/structures/` (Строения и Обелиски)
* **Источник:** *Pixel Crawler Shrines & Crypt* + *Penusbmic Monoliths*
* **Именование файлов:**
  - `structure_wall.png` (50x50 px, защитная стена-баррикада)
  - `structure_altar_t01.png` ... `structure_altar_t16.png` (или базовый `structure_altar.png` 60x60 / 84x108 px)
  - `structure_obelisk.png` (300x300 px, спящий обелиск-монолит)
  - `structure_obelisk_active.png` (300x300 px, пробуждённый инфернальный разлом с вортексом)

---

### 🗺️ `assets/environment/` (Плиты арены и пол)
* **Источник:** *Pixel Crawler: Dungeon Tileset* (Anokolisa)
* **Разрешение:** Плиты `64x64` px (рендеринг в движке по 100x100 px).
* **Именование файлов:**
  - `floor_dungeon_plain.png` (базовая серо-антрацитовая плита, яркость 15-20%)
  - `floor_dungeon_cracked.png` (плиты с трещинами)
  - `floor_dungeon_blood.png` (кровавые следы битв)
  - `floor_dungeon_runic.png` (магические руны на камне)
  - `wall_brick.png` (ограждающие стены арены)

---

### 💎 `assets/icons/` (Иконки снаряжения и реликвий)
* **Источник:** *7Soul's RPG Graphics* (7Soul) / *CazWolf*
* **Разрешение:** Строго `32x32` Pixel Art.
* **Категории:**
  - `weapons/` — 20 базовых оружий (меч, топор, лук, жезл, молот, молния...)
  - `evolutions/` — 20 совершенных оружий (буря клинков, адское пекло, луч смерти...)
  - `passives/` — 14 базовых пассивных реликвий (перчатка, сердце, фолиант...)
  - `legendary/` — 14 легендарных артефактов (эгида бессмертия, октарин, рапира...)
