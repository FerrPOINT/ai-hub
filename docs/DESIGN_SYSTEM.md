# Визуальная система AI Hub

## Направление

Operational console для доступа к моделям, качества и расходов. Основная задача:
увидеть состояние, объяснить расход/ошибку, изменить конфигурацию безопасно.
Без декоративного hero, glass/gradients и метрик без оснований.

Base владеет shell/tokens/primitives. В prototype встроен LF-нормализованный snapshot
frontend/src/ui/tokens.css exact .base-revision; @theme static преобразован в :root
для автономного HTML. Это замороженная design reference, не production fork.
Theme names dark/gray/light сохраняются. Семантические text/surface/border/accent/
success/warning/danger/focus цвета, foreground согласован с theme.

## Геометрия

Header 60 px, sidebar 264/72, gutter clamp(16px,2vw,32px), block gap20,
controls 40 desktop/44 mobile, radius6 (framed panels8). Body14, labels12, title24.
Large KPI digits — fixed-size hierarchy, не viewport-scaled text.
wide: весь work area; reading760 слева; detail fluid+aside320, stack below1024.
Expanded sidebar >=1280, rail768–1279, modal drawer<768.

## Компоненты

| Компонент    | Правило                                                                     |
| ------------ | --------------------------------------------------------------------------- |
| Header       | Service selector/account/theme/global notifications; page CRUD ниже         |
| Page header  | Один h1, descriptor, actions; detail breadcrumb возвращает к списку         |
| Metric       | Definition + known value/unknown + basis/as_of; loading не 0                |
| Table        | Identifiers secondary, status text, confidence рядом с amount               |
| Chart        | Явные units/period/legend, human-sized axis labels вне scaled SVG           |
| Form         | Source поля vs derived metadata; native min/max/pattern + server validation |
| Dialog       | Named title, destructive effect, pending guard, focus return                |
| Tabs         | Стабильная геометрия, selected state, distinct content                      |
| Disclosure   | Scoped safe metadata; content result требует отдельного permission          |
| Empty/error  | Разные причины и actions; no CRUD for404/no retry inference                 |
| Notification | Scope/period/threshold и actionable context; no secret payload              |

Dense tables на mobile переходят в labelled cards, без потери колонок/действий.
Desktop horizontal scroll только локальный named data region при необходимости.
Изменение темы не меняет геометрию/данные. Badge имеет text, не один color signal.
Money форматируется отдельно от ledger arithmetic: unknown=«Неизвестно»/«—»,
zero только known value, tiny amount с exact value доступна detail/tooltip.

## Motion и accessibility

Нет декоративной анимации. Async feedback по state, reduced motion.
Keyboard navigation, visible focus, modal Escape/trap/return, labels и errors связаны.
Tables/charts имеют accessible names/описания, icon buttons — имя действия.
Target viewport matrix включает phone, rail/expanded transitions и large desktop.
Full acceptance — IAB render/geometry/flows, не source-only match.
