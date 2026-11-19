//! Рисунок курсоров Windows: векторные фигуры в сетке 64×64 растеризуются с плавными краями
//! сразу в несколько размеров (32, 48, 64, 96, 128) и пишутся в один `.cur`.
//!
//! Рисунок повторяет старое приложение: белая внешняя линия, тонкая контрастная линия, заливка
//! (однотонная или узор). Узоры и палитры приходят из таблицы курсоров интерфейса; здесь они
//! проверяются. Растеризация и запись `.cur` написаны вручную: без зависимостей и без unsafe.
//! Файлы пишет порт, в тестах на диск ничего не попадает.

use crate::appearance::{parse_hex, AppearanceError};

/// Размеры кадров в `.cur`: Windows сама берёт ближайший к нужному масштабу экрана.
pub const CURSOR_SIZES: [usize; 5] = [32, 48, 64, 96, 128];
/// Сторона сетки, в которой нарисованы фигуры.
const GRID: f64 = 64.0;
/// Рисунок чуть уменьшен вокруг горячей точки, чтобы белая линия не упиралась в край кадра.
const FIT: f64 = 0.91;
/// Общие фигуры (стрелка) живут в JSON: тот же файл рисует превью в интерфейсе.
const SHAPES_JSON: &str = include_str!("../assets/cursor-shapes.json");

/// Стили заливки курсора: четыре узора из старого приложения, однотонный и новые.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    Plain,
    Stripes,
    Dots,
    Checker,
    Cross,
    /// Плавный переход от основного цвета ко второму сверху вниз.
    Gradient,
    /// Две половины по диагонали.
    Split,
    /// Ломаные полосы.
    Zigzag,
    /// Кольца вокруг острия.
    Rings,
    /// Ромбы.
    Diamonds,
    /// Тёмная заливка и светящийся контур основного цвета.
    Neon,
}

impl Pattern {
    pub const ALL: [Pattern; 11] = [
        Pattern::Plain,
        Pattern::Stripes,
        Pattern::Dots,
        Pattern::Checker,
        Pattern::Cross,
        Pattern::Gradient,
        Pattern::Split,
        Pattern::Zigzag,
        Pattern::Rings,
        Pattern::Diamonds,
        Pattern::Neon,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Stripes => "stripes",
            Self::Dots => "dots",
            Self::Checker => "checker",
            Self::Cross => "cross",
            Self::Gradient => "gradient",
            Self::Split => "split",
            Self::Zigzag => "zigzag",
            Self::Rings => "rings",
            Self::Diamonds => "diamonds",
            Self::Neon => "neon",
        }
    }

    pub fn parse(name: &str) -> Result<Self, AppearanceError> {
        Self::ALL
            .into_iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| AppearanceError::Invalid("неизвестный узор курсора".into()))
    }
}

/// Роль курсора в схеме Windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Arrow,
    Help,
    AppStarting,
    Wait,
    Crosshair,
    IBeam,
    Hand,
    No,
    SizeAll,
    SizeNwse,
    SizeNesw,
    SizeNs,
    SizeWe,
    UpArrow,
    NwPen,
    Pin,
    Person,
}

impl Role {
    pub const ALL: [Role; 17] = [
        Role::Arrow,
        Role::Help,
        Role::AppStarting,
        Role::Wait,
        Role::Crosshair,
        Role::IBeam,
        Role::Hand,
        Role::No,
        Role::SizeAll,
        Role::SizeNwse,
        Role::SizeNesw,
        Role::SizeNs,
        Role::SizeWe,
        Role::UpArrow,
        Role::NwPen,
        Role::Pin,
        Role::Person,
    ];

    /// Имя значения в `HKCU\Control Panel\Cursors`.
    pub fn registry_name(self) -> &'static str {
        match self {
            Role::Arrow => "Arrow",
            Role::Help => "Help",
            Role::AppStarting => "AppStarting",
            Role::Wait => "Wait",
            Role::Crosshair => "Crosshair",
            Role::IBeam => "IBeam",
            Role::Hand => "Hand",
            Role::No => "No",
            Role::SizeAll => "SizeAll",
            Role::SizeNwse => "SizeNWSE",
            Role::SizeNesw => "SizeNESW",
            Role::SizeNs => "SizeNS",
            Role::SizeWe => "SizeWE",
            Role::UpArrow => "UpArrow",
            Role::NwPen => "NWPen",
            Role::Pin => "Pin",
            Role::Person => "Person",
        }
    }

    /// Роли с анимацией (`.ani`): «Фоновая работа» и «Занят».
    pub fn is_animated(self) -> bool {
        matches!(self, Role::AppStarting | Role::Wait)
    }

    /// Горячая точка в сетке 64×64.
    fn hotspot(self) -> (f64, f64) {
        match self {
            Role::Arrow
            | Role::Help
            | Role::AppStarting
            | Role::NwPen
            | Role::Pin
            | Role::Person => arrow_hotspot(),
            Role::Hand => (29.0, 6.0),
            Role::UpArrow => (32.0, 6.0),
            _ => (32.0, 32.0),
        }
    }
}

type Pt = (f64, f64);

fn shapes() -> serde_json::Value {
    serde_json::from_str(SHAPES_JSON).expect("cursor-shapes.json проверен тестами")
}

fn arrow_path() -> String {
    shapes()["arrow"]
        .as_str()
        .expect("в cursor-shapes.json есть arrow")
        .to_string()
}

fn arrow_hotspot() -> Pt {
    let v = shapes();
    (
        v["arrowHotspot"][0].as_f64().unwrap_or(10.5),
        v["arrowHotspot"][1].as_f64().unwrap_or(7.0),
    )
}

// ---------- Геометрия ----------

/// Разбирает путь SVG (команды M L H V Q Z, абсолютные) в замкнутые ломаные.
pub fn flatten_path(d: &str) -> Vec<Vec<Pt>> {
    let mut tokens = Vec::new();
    let mut number = String::new();
    for c in d.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() || c == '.' || (c == '-' && number.is_empty()) {
            number.push(c);
        } else {
            if !number.is_empty() {
                tokens.push(number.clone());
                number.clear();
            }
            if c.is_ascii_alphabetic() {
                tokens.push(c.to_string());
            } else if c == '-' {
                number.push(c);
            }
        }
    }
    let mut out: Vec<Vec<Pt>> = Vec::new();
    let mut cur: Vec<Pt> = Vec::new();
    let (mut x, mut y) = (0.0, 0.0);
    let mut cmd = ' ';
    let mut i = 0;
    let num = |tokens: &[String], i: usize| tokens.get(i).and_then(|t| t.parse::<f64>().ok());
    while i < tokens.len() {
        if tokens[i].chars().next().is_some_and(char::is_alphabetic) {
            cmd = tokens[i].chars().next().unwrap_or(' ');
            i += 1;
            if cmd == 'Z' {
                if cur.len() > 2 {
                    out.push(std::mem::take(&mut cur));
                }
                continue;
            }
        }
        match cmd {
            'M' | 'L' => {
                let (Some(nx), Some(ny)) = (num(&tokens, i), num(&tokens, i + 1)) else {
                    break;
                };
                (x, y) = (nx, ny);
                cur.push((x, y));
                i += 2;
                if cmd == 'M' {
                    cmd = 'L';
                }
            }
            'H' => {
                let Some(nx) = num(&tokens, i) else { break };
                x = nx;
                cur.push((x, y));
                i += 1;
            }
            'V' => {
                let Some(ny) = num(&tokens, i) else { break };
                y = ny;
                cur.push((x, y));
                i += 1;
            }
            'Q' => {
                let (Some(cx), Some(cy), Some(nx), Some(ny)) = (
                    num(&tokens, i),
                    num(&tokens, i + 1),
                    num(&tokens, i + 2),
                    num(&tokens, i + 3),
                ) else {
                    break;
                };
                const STEPS: usize = 12;
                for k in 1..=STEPS {
                    let t = k as f64 / STEPS as f64;
                    let u = 1.0 - t;
                    cur.push((
                        u * u * x + 2.0 * u * t * cx + t * t * nx,
                        u * u * y + 2.0 * u * t * cy + t * t * ny,
                    ));
                }
                (x, y) = (nx, ny);
                i += 4;
            }
            _ => break,
        }
    }
    if cur.len() > 2 {
        out.push(cur);
    }
    out
}

fn circle(cx: f64, cy: f64, r: f64) -> Vec<Pt> {
    const N: usize = 72;
    (0..N)
        .map(|k| {
            let a = std::f64::consts::TAU * k as f64 / N as f64;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

fn rotate(points: &mut [Pt], degrees: f64, about: Pt) {
    let (s, c) = degrees.to_radians().sin_cos();
    for p in points {
        let (dx, dy) = (p.0 - about.0, p.1 - about.1);
        *p = (about.0 + dx * c - dy * s, about.1 + dx * s + dy * c);
    }
}

/// Геометрия одного элемента рисунка.
enum Geo {
    /// Заливка: несколько замкнутых ломаных (чётно-нечётное правило).
    Fill(Vec<Vec<Pt>>),
    /// Линия заданной толщины со скруглёнными концами.
    Stroke(Vec<Pt>, f64),
}

impl Geo {
    /// Знаковое расстояние до фигуры: меньше нуля внутри.
    fn signed_distance(&self, px: f64, py: f64) -> f64 {
        match self {
            Geo::Fill(polys) => {
                let mut inside = false;
                let mut best = f64::MAX;
                for poly in polys {
                    let n = poly.len();
                    for i in 0..n {
                        let (x1, y1) = poly[i];
                        let (x2, y2) = poly[(i + 1) % n];
                        if (y1 > py) != (y2 > py) && px < (x2 - x1) * (py - y1) / (y2 - y1) + x1 {
                            inside = !inside;
                        }
                        best = best.min(segment_distance(px, py, (x1, y1), (x2, y2)));
                    }
                }
                if inside {
                    -best
                } else {
                    best
                }
            }
            Geo::Stroke(line, width) => {
                let best = line
                    .windows(2)
                    .map(|w| segment_distance(px, py, w[0], w[1]))
                    .fold(f64::MAX, f64::min);
                best - width / 2.0
            }
        }
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        let mut add = |p: &Pt| {
            x0 = x0.min(p.0);
            y0 = y0.min(p.1);
            x1 = x1.max(p.0);
            y1 = y1.max(p.1);
        };
        match self {
            Geo::Fill(polys) => polys.iter().flatten().for_each(&mut add),
            Geo::Stroke(line, width) => {
                line.iter().for_each(&mut add);
                x0 -= width / 2.0;
                y0 -= width / 2.0;
                x1 += width / 2.0;
                y1 += width / 2.0;
            }
        }
        (x0, y0, x1, y1)
    }
}

fn segment_distance(px: f64, py: f64, a: Pt, b: Pt) -> f64 {
    let (ex, ey) = (b.0 - a.0, b.1 - a.1);
    let len2 = ex * ex + ey * ey;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((px - a.0) * ex + (py - a.1) * ey) / len2).clamp(0.0, 1.0)
    };
    ((px - a.0 - t * ex).powi(2) + (py - a.1 - t * ey).powi(2)).sqrt()
}

/// Чем закрашен элемент.
#[derive(Clone, Copy)]
enum Ink {
    Black,
    Second,
    /// Второй цвет, приглушённый (дорожка под бегущей дугой).
    SecondFaint,
}

enum Kind {
    /// Фигура курсора: белая внешняя линия, контур, заливка узором.
    Shape,
    /// Значок-бейдж: контур и заливка вторым цветом.
    Badge,
    /// Один слой краски.
    Draw(Ink),
}

struct Item {
    geo: Geo,
    kind: Kind,
}

fn shape(polys: Vec<Vec<Pt>>) -> Item {
    Item {
        geo: Geo::Fill(polys),
        kind: Kind::Shape,
    }
}

fn badge(glyph: Vec<Item>) -> Vec<Item> {
    let mut items = vec![Item {
        geo: Geo::Fill(vec![circle(46.0, 16.0, 11.0)]),
        kind: Kind::Badge,
    }];
    items.extend(glyph);
    items
}

fn ink(geo: Geo, ink: Ink) -> Item {
    Item {
        geo,
        kind: Kind::Draw(ink),
    }
}

fn line(points: &[Pt], width: f64, color: Ink) -> Item {
    ink(Geo::Stroke(points.to_vec(), width), color)
}

fn dot(cx: f64, cy: f64, r: f64, color: Ink) -> Item {
    ink(Geo::Fill(vec![circle(cx, cy, r)]), color)
}

/// Дуга окружности от `start` градусов длиной `span`, ломаная для `Geo::Stroke`.
fn arc(cx: f64, cy: f64, r: f64, start: f64, span: f64) -> Vec<Pt> {
    let steps = (span / 6.0).ceil().max(2.0) as usize;
    (0..=steps)
        .map(|k| {
            let a = (start + span * k as f64 / steps as f64).to_radians();
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

/// Все элементы рисунка роли в сетке 64×64 в порядке отрисовки. `phase` (0..1) — кадр
/// анимации «Занят» и «Фоновая работа»: оборот бегущей дуги.
fn items(role: Role, phase: f64) -> Vec<Item> {
    let turn = phase * 360.0;
    let arrow = || shape(flatten_path(&arrow_path()));
    let size_arrow = |angle: f64| {
        let mut polys = flatten_path(
            "M8,29 L19,18 Q22,16 22,20 V27 H42 V20 Q42,16 45,18 L56,29 Q59,32 56,35 \
             L45,46 Q42,48 42,44 V37 H22 V44 Q22,48 19,46 L8,35 Q5,32 8,29 Z",
        );
        for poly in &mut polys {
            rotate(poly, angle, (32.0, 32.0));
        }
        vec![shape(polys)]
    };
    match role {
        Role::Arrow => vec![arrow()],
        Role::NwPen => {
            let mut v = vec![arrow()];
            v.extend(badge(vec![
                line(&[(41.0, 21.0), (50.0, 11.0)], 2.8, Ink::Black),
                dot(40.6, 21.4, 1.8, Ink::Black),
            ]));
            v
        }
        Role::Help => {
            let mut v = vec![arrow()];
            v.extend(badge(vec![
                line(
                    &[
                        (41.8, 13.4),
                        (42.4, 10.8),
                        (44.4, 9.4),
                        (47.0, 9.6),
                        (49.0, 11.4),
                        (48.6, 13.8),
                        (46.4, 15.4),
                        (46.0, 17.2),
                    ],
                    2.4,
                    Ink::Black,
                ),
                dot(46.0, 21.0, 1.6, Ink::Black),
            ]));
            v
        }
        Role::AppStarting => {
            let mut v = vec![arrow()];
            v.extend(badge(vec![line(
                &arc(46.0, 16.0, 5.2, turn - 90.0, 240.0),
                3.0,
                Ink::Black,
            )]));
            v
        }
        Role::Pin => {
            let mut v = vec![arrow()];
            v.extend(badge(vec![
                line(&[(41.0, 16.0), (51.0, 16.0)], 2.6, Ink::Black),
                line(&[(46.0, 11.0), (46.0, 21.0)], 2.6, Ink::Black),
            ]));
            v
        }
        Role::Person => {
            let mut v = vec![arrow()];
            v.extend(badge(vec![
                dot(46.0, 13.0, 3.0, Ink::Black),
                line(&[(40.5, 24.0), (46.0, 18.5), (51.5, 24.0)], 3.0, Ink::Black),
            ]));
            v
        }
        Role::IBeam => vec![shape(flatten_path(
            "M23,8 Q23,6 25,6 H39 Q41,6 41,8 V11 H35 V53 H41 V58 H23 V53 H29 V11 H23 Z",
        ))],
        Role::Crosshair => vec![shape(flatten_path(
            "M29,6 H35 V27 H57 V34 H35 V57 H29 V34 H7 V27 H29 Z",
        ))],
        Role::Hand => vec![shape(flatten_path(
            "M25,30 V11 Q25,6 29,6 Q33,6 33,11 V27 Q35,22 39,25 Q42,22 46,28 Q51,27 53,33 \
             L51,47 Q50,56 43,58 H29 Q24,57 21,51 L11,37 Q8,32 13,30 Q16,29 20,34 L25,39 Z",
        ))],
        Role::UpArrow => vec![shape(flatten_path(
            "M32,6 Q30,6 28,9 L10,28 Q7,32 13,32 H23 V55 Q23,58 26,58 H38 Q41,58 41,55 V32 \
             H51 Q57,32 54,28 L36,9 Q34,6 32,6 Z",
        ))],
        Role::SizeAll => {
            let mut v = vec![shape(vec![circle(32.0, 32.0, 21.0)])];
            v.push(line(&[(17.0, 32.0), (47.0, 32.0)], 5.0, Ink::Black));
            v.push(line(&[(32.0, 17.0), (32.0, 47.0)], 5.0, Ink::Black));
            v
        }
        Role::No => vec![
            shape(vec![circle(32.0, 32.0, 21.0)]),
            line(&[(18.0, 18.0), (46.0, 46.0)], 6.0, Ink::Black),
        ],
        Role::Wait => vec![
            shape(vec![circle(32.0, 32.0, 21.0)]),
            // Дорожка и бегущая дуга второго цвета.
            line(&arc(32.0, 32.0, 13.0, 0.0, 360.0), 7.0, Ink::SecondFaint),
            line(&arc(32.0, 32.0, 13.0, turn - 90.0, 110.0), 7.0, Ink::Second),
        ],
        Role::SizeWe => size_arrow(0.0),
        Role::SizeNs => size_arrow(90.0),
        Role::SizeNwse => size_arrow(45.0),
        Role::SizeNesw => size_arrow(-45.0),
    }
}

// ---------- Краска ----------

type Rgb = [f64; 3];

fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Расстояние до `centre` по кругу с периодом 16 (для полос).
fn ring_dist(a: f64, centre: f64) -> f64 {
    ((a - centre + 8.0).rem_euclid(16.0) - 8.0).abs()
}

/// Цвет заливки в точке сетки 64×64 (ячейка узора 16).
fn fill_color(pattern: Pattern, main: Rgb, second: Rgb, x: f64, y: f64) -> Rgb {
    const WHITE: Rgb = [255.0; 3];
    const DARK: Rgb = [18.0, 18.0, 22.0];
    let (cx, cy) = (x.rem_euclid(16.0), y.rem_euclid(16.0));
    match pattern {
        Pattern::Plain => main,
        Pattern::Stripes => {
            let a = x + y;
            if ring_dist(a, 12.0) <= 2.9 || ring_dist(a, 8.0) <= 2.9 {
                second
            } else {
                main
            }
        }
        Pattern::Dots => {
            if (cx - 4.0).hypot(cy - 4.0) <= 3.0 {
                second
            } else if (cx - 12.0).hypot(cy - 12.0) <= 2.0 {
                WHITE
            } else {
                main
            }
        }
        Pattern::Checker => {
            if (cx < 8.0) == (cy < 8.0) {
                second
            } else {
                main
            }
        }
        Pattern::Cross => {
            if (cx - 8.0).abs() <= 1.0 {
                WHITE
            } else if (cy - 8.0).abs() <= 2.5 {
                second
            } else {
                main
            }
        }
        Pattern::Gradient => mix(main, second, ((y - 6.0) / 52.0).clamp(0.0, 1.0)),
        Pattern::Split => {
            if x + y < 58.0 {
                main
            } else {
                second
            }
        }
        Pattern::Zigzag => {
            let tri = ((x.rem_euclid(12.0) / 12.0) * 2.0 - 1.0).abs() * 6.0;
            if (y + tri).rem_euclid(14.0) < 5.0 {
                second
            } else {
                main
            }
        }
        Pattern::Rings => {
            let d = (x - 10.5).hypot(y - 7.0);
            if d.rem_euclid(10.0) < 4.0 {
                second
            } else {
                main
            }
        }
        Pattern::Diamonds => {
            if (cx - 8.0).abs() + (cy - 8.0).abs() < 5.0 {
                second
            } else {
                main
            }
        }
        Pattern::Neon => DARK,
    }
}

/// Растр RGBA `size`×`size` (не премультиплицированный) одной роли.
pub fn render(
    role: Role,
    pattern: Pattern,
    color: &str,
    secondary: &str,
    size: usize,
) -> Result<Vec<u8>, AppearanceError> {
    render_frame(role, pattern, color, secondary, size, 0.0)
}

/// То же для кадра анимации: `phase` от 0 до 1 — доля оборота.
pub fn render_frame(
    role: Role,
    pattern: Pattern,
    color: &str,
    secondary: &str,
    size: usize,
    phase: f64,
) -> Result<Vec<u8>, AppearanceError> {
    let (r, g, b) = parse_hex(color)?;
    let (r2, g2, b2) = parse_hex(secondary)?;
    let main = [f64::from(r), f64::from(g), f64::from(b)];
    let second = [f64::from(r2), f64::from(g2), f64::from(b2)];
    let outline = if pattern == Pattern::Neon {
        main
    } else {
        [0.0; 3]
    };
    let (hx, hy) = role.hotspot();
    // Сетка -> пиксели: масштаб `k`, неподвижна горячая точка.
    let k = FIT * size as f64 / GRID;
    let (ox, oy) = (hx * size as f64 / GRID, hy * size as f64 / GRID);
    let to_design = |u: f64, v: f64| (hx + (u - ox) / k, hy + (v - oy) / k);

    let mut acc = vec![[0.0f64; 4]; size * size]; // премультиплицированный RGBA, 0..1
    let over = |dst: &mut [f64; 4], rgb: Rgb, alpha: f64| {
        if alpha <= 0.0 {
            return;
        }
        let inv = 1.0 - alpha;
        for c in 0..3 {
            dst[c] = rgb[c] / 255.0 * alpha + dst[c] * inv;
        }
        dst[3] = alpha + dst[3] * inv;
    };
    let cover = |sd: f64, grow: f64| (0.5 - (sd - grow) * k).clamp(0.0, 1.0);
    let ink_rgb = |ink: Ink| match ink {
        Ink::Black => [0.0; 3],
        Ink::Second | Ink::SecondFaint => second,
    };

    for item in items(role, phase) {
        let (x0, y0, x1, y1) = item.geo.bounds();
        let margin = 4.0;
        for py in 0..size {
            for px in 0..size {
                let (x, y) = to_design(px as f64 + 0.5, py as f64 + 0.5);
                if x < x0 - margin || x > x1 + margin || y < y0 - margin || y > y1 + margin {
                    continue;
                }
                let sd = item.geo.signed_distance(x, y);
                let dst = &mut acc[py * size + px];
                match item.kind {
                    Kind::Shape => {
                        over(dst, [255.0; 3], cover(sd, 2.5));
                        over(dst, outline, cover(sd, 1.2));
                        let fill = cover(sd, -1.2);
                        if fill > 0.0 {
                            // Узор усредняется по 3×3 точкам: края полос тоже гладкие.
                            let mut sum = [0.0; 3];
                            for sy in 0..3 {
                                for sx in 0..3 {
                                    let (fx, fy) = to_design(
                                        px as f64 + (sx as f64 + 0.5) / 3.0,
                                        py as f64 + (sy as f64 + 0.5) / 3.0,
                                    );
                                    let c = fill_color(pattern, main, second, fx, fy);
                                    for i in 0..3 {
                                        sum[i] += c[i] / 9.0;
                                    }
                                }
                            }
                            over(dst, sum, fill);
                        }
                    }
                    Kind::Badge => {
                        over(dst, [0.0; 3], cover(sd, 1.0));
                        over(dst, second, cover(sd, -1.0));
                    }
                    Kind::Draw(color) => {
                        let strength = if matches!(color, Ink::SecondFaint) {
                            0.3
                        } else {
                            1.0
                        };
                        over(dst, ink_rgb(color), cover(sd, 0.0) * strength);
                    }
                }
            }
        }
    }

    let mut out = vec![0u8; size * size * 4];
    for (i, p) in acc.iter().enumerate() {
        let alpha = p[3];
        if alpha > 0.0 {
            for c in 0..3 {
                out[i * 4 + c] = (p[c] / alpha * 255.0).round().clamp(0.0, 255.0) as u8;
            }
            out[i * 4 + 3] = (alpha * 255.0).round() as u8;
        }
    }
    Ok(out)
}

/// Горячая точка роли в пикселях кадра `size`.
pub fn hotspot(role: Role, size: usize) -> (u16, u16) {
    let (hx, hy) = role.hotspot();
    (
        (hx * size as f64 / GRID).round() as u16,
        (hy * size as f64 / GRID).round() as u16,
    )
}

/// Кадр `.cur`: DIB 32 бита (BGRA снизу вверх) и маска AND.
fn dib_frame(rgba: &[u8], size: usize) -> Vec<u8> {
    let mask_stride = size.div_ceil(32) * 4;
    let mut out = Vec::with_capacity(40 + size * size * 4 + mask_stride * size);
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(&((size * 2) as u32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&((size * size * 4 + mask_stride * size) as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 16]);
    for row in (0..size).rev() {
        for px in rgba[row * size * 4..(row + 1) * size * 4].chunks(4) {
            out.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }
    }
    for row in (0..size).rev() {
        let mut mask = vec![0u8; mask_stride];
        for x in 0..size {
            if rgba[(row * size + x) * 4 + 3] == 0 {
                mask[x / 8] |= 0x80 >> (x % 8);
            }
        }
        out.extend_from_slice(&mask);
    }
    out
}

/// Файл курсора из готовых кадров (DIB): каталог с горячими точками и сами кадры.
fn cur_file(role: Role, frames: &[(usize, Vec<u8>)]) -> Vec<u8> {
    let mut cur = vec![0, 0, 2, 0];
    cur.extend_from_slice(&(frames.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * frames.len();
    for (size, data) in frames {
        let (hx, hy) = hotspot(role, *size);
        // Для курсора в полях «плоскости» и «глубина» каталога лежит горячая точка.
        cur.extend_from_slice(&[*size as u8, *size as u8, 0, 0]);
        cur.extend_from_slice(&hx.to_le_bytes());
        cur.extend_from_slice(&hy.to_le_bytes());
        cur.extend_from_slice(&(data.len() as u32).to_le_bytes());
        cur.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += data.len();
    }
    for (_, data) in frames {
        cur.extend_from_slice(data);
    }
    cur
}

/// Файл `.cur` со всеми кадрами `CURSOR_SIZES` и горячей точкой каждого кадра.
pub fn build_cur(
    role: Role,
    pattern: Pattern,
    color: &str,
    secondary: &str,
) -> Result<Vec<u8>, AppearanceError> {
    let mut frames = Vec::new();
    for size in CURSOR_SIZES {
        let rgba = render(role, pattern, color, secondary, size)?;
        frames.push((size, dib_frame(&rgba, size)));
    }
    Ok(cur_file(role, &frames))
}

/// Размеры кадров анимации: достаточно трёх, чтобы файл не рос.
pub const ANI_SIZES: [usize; 3] = [32, 48, 64];
/// Кадров на оборот.
pub const ANI_FRAMES: usize = 16;

/// Длительность кадра в «джиффи» (1/60 с): «Занят» крутится спокойнее, чем «Фоновая работа».
fn ani_jiffies(role: Role) -> u32 {
    if role == Role::Wait {
        4
    } else {
        3
    }
}

fn riff_chunk(out: &mut Vec<u8>, id: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
}

fn riff_list(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    let mut data = kind.to_vec();
    data.extend_from_slice(body);
    riff_chunk(out, b"LIST", &data);
}

/// Анимированный курсор `.ani` (RIFF `ACON`): заголовок `anih`, длительности `rate`,
/// порядок `seq ` и список `fram` из кадров-значков (каждый — полный `.cur`).
/// Годится только для ролей с `is_animated`.
pub fn build_ani(
    role: Role,
    pattern: Pattern,
    color: &str,
    secondary: &str,
) -> Result<Vec<u8>, AppearanceError> {
    if !role.is_animated() {
        return Err(AppearanceError::Invalid("эта роль не анимированная".into()));
    }
    let mut icons = Vec::new();
    for frame in 0..ANI_FRAMES {
        let phase = frame as f64 / ANI_FRAMES as f64;
        let mut dibs = Vec::new();
        for size in ANI_SIZES {
            let rgba = render_frame(role, pattern, color, secondary, size, phase)?;
            dibs.push((size, dib_frame(&rgba, size)));
        }
        icons.push(cur_file(role, &dibs));
    }
    let n = ANI_FRAMES as u32;
    // anih: размер заголовка, кадры, шаги, ширина, высота, глубина, плоскости, скорость, флаги
    // (1 — кадры это значки, 2 — есть порядок `seq `).
    let mut anih = Vec::new();
    for v in [36, n, n, 32, 32, 32, 1, ani_jiffies(role), 3] {
        anih.extend_from_slice(&u32::to_le_bytes(v));
    }
    let rate: Vec<u8> = (0..n)
        .flat_map(|_| ani_jiffies(role).to_le_bytes())
        .collect();
    let seq: Vec<u8> = (0..n).flat_map(u32::to_le_bytes).collect();
    let mut info = Vec::new();
    riff_chunk(&mut info, b"INAM", b"OCHISTKA\0");
    riff_chunk(&mut info, b"IART", b"OCHISTKA\0");
    let mut fram = Vec::new();
    for icon in &icons {
        riff_chunk(&mut fram, b"icon", icon);
    }
    let mut body = b"ACON".to_vec();
    riff_list(&mut body, b"INFO", &info);
    riff_chunk(&mut body, b"anih", &anih);
    riff_chunk(&mut body, b"rate", &rate);
    riff_chunk(&mut body, b"seq ", &seq);
    riff_list(&mut body, b"fram", &fram);
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le16(b: &[u8], at: usize) -> usize {
        u16::from_le_bytes([b[at], b[at + 1]]) as usize
    }
    fn le32(b: &[u8], at: usize) -> usize {
        u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize
    }

    #[test]
    fn cur_header_lists_every_size_with_scaled_hotspots() {
        let cur = build_cur(Role::Arrow, Pattern::Stripes, "#B4FF00", "#FF009D").unwrap();
        assert_eq!(&cur[..4], &[0, 0, 2, 0], "тип 2 — курсор");
        assert_eq!(le16(&cur, 4), CURSOR_SIZES.len());
        let mut end = 6 + 16 * CURSOR_SIZES.len();
        for (i, size) in CURSOR_SIZES.iter().enumerate() {
            let e = 6 + 16 * i;
            assert_eq!((cur[e] as usize, cur[e + 1] as usize), (*size, *size));
            let (hx, hy) = hotspot(Role::Arrow, *size);
            assert_eq!(
                (le16(&cur, e + 4), le16(&cur, e + 6)),
                (hx as usize, hy as usize)
            );
            let (len, offset) = (le32(&cur, e + 8), le32(&cur, e + 12));
            assert_eq!(offset, end, "кадры идут подряд");
            // Заголовок DIB: 40 байт, ширина, удвоенная высота, 32 бита.
            assert_eq!(le32(&cur, offset), 40);
            assert_eq!(le32(&cur, offset + 4), *size);
            assert_eq!(le32(&cur, offset + 8), size * 2);
            assert_eq!(le16(&cur, offset + 14), 32);
            end += len;
        }
        assert_eq!(cur.len(), end);
    }

    #[test]
    fn sizes_cover_32_to_128() {
        assert_eq!(CURSOR_SIZES, [32, 48, 64, 96, 128]);
    }

    #[test]
    fn hotspot_scales_with_the_frame() {
        let (x32, y32) = hotspot(Role::Arrow, 32);
        let (x128, y128) = hotspot(Role::Arrow, 128);
        assert!((x128 as i32 - 4 * x32 as i32).abs() <= 2);
        assert!((y128 as i32 - 4 * y32 as i32).abs() <= 2);
        assert_eq!(hotspot(Role::IBeam, 64), (32, 32));
    }

    #[test]
    fn render_has_opaque_fill_smooth_edges_and_transparent_corner() {
        let rgba = render(Role::Arrow, Pattern::Plain, "#FF0000", "#FF0000", 128).unwrap();
        assert_eq!(rgba.len(), 128 * 128 * 4);
        assert_eq!(rgba[3], 0, "угол прозрачный");
        assert!(
            rgba.chunks(4).any(|p| p == [255, 0, 0, 255]),
            "есть красная заливка"
        );
        assert!(
            rgba.chunks(4).any(|p| p[3] > 0 && p[3] < 255),
            "края сглажены (частичная прозрачность)"
        );
        assert!(
            rgba.chunks(4).any(|p| p == [255, 255, 255, 255]),
            "есть белая внешняя линия"
        );
    }

    #[test]
    fn every_role_and_pattern_renders_something_in_the_frame() {
        for role in Role::ALL {
            let rgba = render(role, Pattern::Plain, "#0066FF", "#FFD400", 48).unwrap();
            let covered = rgba.chunks(4).filter(|p| p[3] > 0).count();
            assert!(covered > 100, "{role:?}: рисунок пустой");
            // Края кадра прозрачны: рисунок целиком помещается.
            for i in 0..48 {
                assert_eq!(rgba[i * 4 + 3], 0, "{role:?}: верхний край");
                assert_eq!(rgba[(47 * 48 + i) * 4 + 3], 0, "{role:?}: нижний край");
                assert_eq!(rgba[(i * 48) * 4 + 3], 0, "{role:?}: левый край");
                assert_eq!(rgba[(i * 48 + 47) * 4 + 3], 0, "{role:?}: правый край");
            }
        }
        let plain = render(Role::Arrow, Pattern::Plain, "#B4FF00", "#FF009D", 64).unwrap();
        for pattern in Pattern::ALL.into_iter().filter(|p| *p != Pattern::Plain) {
            let other = render(Role::Arrow, pattern, "#B4FF00", "#FF009D", 64).unwrap();
            assert_ne!(plain, other, "{pattern:?} отличается от однотонного");
        }
    }

    #[test]
    fn there_are_at_least_ten_styles_with_unique_names() {
        assert!(Pattern::ALL.len() >= 10);
        for p in Pattern::ALL {
            assert_eq!(Pattern::parse(p.name()), Ok(p));
        }
    }

    #[test]
    fn neon_has_a_coloured_outline_and_dark_fill() {
        let rgba = render(Role::Arrow, Pattern::Neon, "#00FFFF", "#FF00FF", 96).unwrap();
        assert!(rgba.chunks(4).any(|p| p == [0, 255, 255, 255]));
        assert!(rgba.chunks(4).any(|p| p == [18, 18, 22, 255]));
    }

    #[test]
    fn arrow_path_flattens_to_one_closed_outline() {
        let polys = flatten_path(&arrow_path());
        assert_eq!(polys.len(), 1);
        assert!(polys[0].len() > 40, "кривые разбиты на отрезки");
    }

    #[test]
    fn shapes_json_is_valid() {
        let v = shapes();
        assert_eq!(v["grid"].as_f64(), Some(64.0));
        assert!(v["arrow"].as_str().unwrap().starts_with("M12,7"));
    }

    /// Проверяет один `.cur`: тип 2, каталог, кадры 32 бита с маской AND и удвоенной высотой.
    fn check_cur(cur: &[u8], role: Role) -> usize {
        assert_eq!(&cur[..4], &[0, 0, 2, 0]);
        let count = le16(cur, 4);
        let mut end = 6 + 16 * count;
        for i in 0..count {
            let e = 6 + 16 * i;
            let size = cur[e] as usize;
            assert_eq!(cur[e + 1] as usize, size);
            let (hx, hy) = hotspot(role, size);
            assert_eq!(
                (le16(cur, e + 4), le16(cur, e + 6)),
                (hx as usize, hy as usize)
            );
            assert!(
                le16(cur, e + 4) < size && le16(cur, e + 6) < size,
                "горячая точка в кадре"
            );
            let (len, offset) = (le32(cur, e + 8), le32(cur, e + 12));
            assert_eq!(offset, end);
            let mask = size.div_ceil(32) * 4 * size;
            assert_eq!(
                len,
                40 + size * size * 4 + mask,
                "заголовок + BGRA + маска AND"
            );
            assert_eq!(le32(cur, offset + 8), size * 2, "высота удвоена");
            assert_eq!(le16(cur, offset + 14), 32);
            assert_eq!(le32(cur, offset + 16), 0, "BI_RGB");
            end += len;
        }
        assert_eq!(cur.len(), end);
        count
    }

    #[test]
    fn every_role_builds_a_valid_cur() {
        for role in Role::ALL {
            let cur = build_cur(role, Pattern::Plain, "#B4FF00", "#FF009D").unwrap();
            assert_eq!(check_cur(&cur, role), CURSOR_SIZES.len(), "{role:?}");
        }
    }

    #[test]
    fn ani_is_a_well_formed_riff_acon_with_rotating_frames() {
        let ani = build_ani(Role::Wait, Pattern::Stripes, "#B4FF00", "#FF009D").unwrap();
        assert_eq!(&ani[..4], b"RIFF");
        assert_eq!(le32(&ani, 4), ani.len() - 8, "размер RIFF");
        assert_eq!(&ani[8..12], b"ACON");
        // Обходим фрагменты верхнего уровня; нечётные размеры дополняются нулём.
        let mut at = 12;
        let mut seen = Vec::new();
        let mut frames = Vec::new();
        while at < ani.len() {
            let id = String::from_utf8_lossy(&ani[at..at + 4]).to_string();
            let len = le32(&ani, at + 4);
            let data = &ani[at + 8..at + 8 + len];
            match id.as_str() {
                "anih" => {
                    assert_eq!(len, 36);
                    assert_eq!(le32(data, 0), 36);
                    assert_eq!(le32(data, 4), ANI_FRAMES, "кадры");
                    assert_eq!(le32(data, 8), ANI_FRAMES, "шаги");
                    assert_eq!(le32(data, 32), 3, "значки + seq");
                    assert!(le32(data, 28) > 0, "скорость");
                }
                "rate" => assert_eq!(len, 4 * ANI_FRAMES),
                "seq " => {
                    assert_eq!(len, 4 * ANI_FRAMES);
                    assert_eq!(le32(data, 4 * 5), 5);
                }
                "LIST" if &data[..4] == b"fram" => {
                    let mut p = 4;
                    while p < data.len() {
                        assert_eq!(&data[p..p + 4], b"icon");
                        let l = le32(data, p + 4);
                        frames.push(data[p + 8..p + 8 + l].to_vec());
                        p += 8 + l + l % 2;
                    }
                }
                "LIST" => assert_eq!(&data[..4], b"INFO"),
                other => panic!("лишний фрагмент {other}"),
            }
            seen.push(id);
            at += 8 + len + len % 2;
        }
        assert_eq!(at, ani.len());
        assert_eq!(seen, ["LIST", "anih", "rate", "seq ", "LIST"]);
        assert_eq!(frames.len(), ANI_FRAMES);
        for f in &frames {
            assert_eq!(check_cur(f, Role::Wait), ANI_SIZES.len());
        }
        assert_ne!(frames[0], frames[4], "кадры различаются (дуга вращается)");
        assert_eq!(frames[0].len(), frames[7].len());
    }

    #[test]
    fn both_busy_roles_animate_and_others_refuse_ani() {
        let ani = build_ani(Role::AppStarting, Pattern::Plain, "#087FFF", "#F5FF00").unwrap();
        assert_eq!(&ani[8..12], b"ACON");
        assert!(Role::ALL.iter().filter(|r| r.is_animated()).count() == 2);
        assert!(build_ani(Role::Arrow, Pattern::Plain, "#000000", "#000000").is_err());
    }

    #[test]
    fn bad_input_is_refused() {
        assert!(Pattern::parse("wave").is_err());
        assert!(render(Role::Arrow, Pattern::Plain, "red", "#000000", 32).is_err());
        assert!(build_cur(Role::Arrow, Pattern::Plain, "#000000", "zz").is_err());
    }
}
