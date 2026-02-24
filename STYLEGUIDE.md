# Стайлгайд

Единый визуальный стиль и правила верстки для компонентов приложения.

## 1. Цвета

- **Не использовать** захардкоженные цвета (rgba, hex) в компонентах.
- Использовать **Mantine CSS-переменные**: `var(--mantine-color-*)`.
- **Бордеры:** `var(--mantine-color-default-border)`.
- **Приглушённый текст:** проп `c="dimmed"` у Text.
- **Фоны:** `var(--mantine-color-body)`, для контрастных блоков — `var(--mantine-color-dark-6)` / `var(--mantine-color-gray-0)`.
- **Акцентный цвет:** blue (дефолт Mantine).
- **Опасные действия:** `color="red"`.
- **Успех:** `color="green"`.

## 2. Типографика

- **Заголовок страницы:** `Title order={2}` (SettingsPage, SnippetsPage).
- **Заголовок секции:** `Text size="sm" fw={500}`.
- **Основной текст:** `Text size="sm"`.
- **Подсказки / мелкий текст:** `Text size="xs" c="dimmed"`.
- **Размер шрифта сообщений:** настраиваемый через `settings.font_size`.

## 3. Отступы

- **Между секциями:** `gap="lg"` (Stack).
- **Внутри секции:** `gap="sm"` (Stack).
- **Padding контейнеров:** `p="md"`.
- **Padding компактных элементов:** `p="xs"`.
- **Gap между кнопками в группе:** `gap="sm"` (Group).

## 4. Компоненты

### ActionIcon (иконки-кнопки)

- **Навигация / тулбар:** `size="lg"`, `variant="subtle"`.
- **Действия на элементах** (редактировать, удалить, копировать): `size="xs"`, `variant="subtle"`.
- Иконки: размер 18 для `size="lg"`, 16 для `size="sm"`, 14 для `size="xs"`.

### Button

- **Основное действие:** `variant="filled"` (или дефолт).
- **Вторичное действие:** `variant="light"`.
- **Отмена:** `variant="subtle"`.
- **Опасное:** `color="red"`.
- **Ширина:** `fullWidth` в модалках и формах, auto в тулбарах.

### Modal

- **Размер:** `size="sm"` для подтверждений, `size="md"` для форм.
- Всегда: `title`, кнопки внизу в `Group justify="flex-end"`.

### Card

- Для элементов списка: `withBorder`, `p="sm"`.

### Tooltip

- Оборачивать **все** ActionIcon без текстовой метки.

### TextInput / Textarea / Select / Slider

- Label через проп `label` (не отдельный Text).
- Размер: дефолтный (не задавать `size`, если не нужен компактный).

## 5. Иконки

- **Источник:** `@tabler/icons-react`.
- **Размер по умолчанию:** `size={18}`.
- **Стиль:** `stroke={1.5}` (если поддерживается).

## 6. Лейаут

- **Страницы** (Settings, Snippets): ScrollArea с `maxWidth` 800–900px, `mx="auto"`, `p="lg"`.
- **Хедер чата:** `height` 40px, `borderBottom`, `flexShrink: 0`.
- **Сайдбары:** `flexShrink: 0`, фиксированная ширина через проп.
- **Основная область:** `flex: 1`, `minWidth: 0`.

## 7. Анимации

- **Opacity transitions:** 0.2s.
- **Hover-эффекты:** через CSS-классы, не инлайн-стили.

## 8. Уведомления

- **Успех:** `notify.success()` — зелёный, 3 сек.
- **Ошибка:** `notify.error()` — красный, 5 сек.
- **Предупреждение:** `notify.warning()` — жёлтый, 4 сек.
- **Инфо:** `notify.info()` — синий, 3 сек.
